mod data;

use data::{Journal, NewFinancing, NewTrade, Store, Trade};
use libcalculatorfinance::{calculate_risk_input, convert_from_orig};
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError>
{
    let database_path = data::database_path();
    let store = match Store::open(&database_path)
    {
        Ok(store) => store,
        Err(error) =>
        {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    let journal = store
        .load()
        .expect("Initialized database should be readable");
    let store = Rc::new(RefCell::new(store));
    let journal = Rc::new(RefCell::new(journal));
    let window = AppWindow::new()?;
    window.set_source_label(database_path.display().to_string().into());
    set_products(&window, &journal.borrow());
    refresh(&window, &journal.borrow(), "All products");
    window.set_status(
        format!(
            "Loaded {} trades and {} financing entries",
            journal.borrow().trades.len(),
            journal.borrow().financing.len()
        )
        .into(),
    );

    let weak = window.as_weak();
    let model = journal.clone();
    window.on_filter_product(move |product| {
        if let Some(window) = weak.upgrade()
        {
            refresh(&window, &model.borrow(), &product);
        }
    });

    window.on_calculate_risk_input(|pool, percent| {
        let value = pool
            .trim()
            .parse::<f64>()
            .ok()
            .zip(percent.trim().parse::<f64>().ok())
            .filter(|(pool, percent)| {
                pool.is_finite() && *pool >= 0.0 && percent.is_finite() && *percent >= 0.0
            })
            .map(|(pool, percent)| calculate_risk_input(pool, percent))
            .filter(|value| value.is_finite());
        value
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "—".into())
            .into()
    });
    window.on_calculate_stoploss(|pool, percent, price, quantity, exchange_rate, commission_buy,
                                  tax_buy, commission_sell, tax_sell, side| {
        let inputs: Option<Vec<f64>> = [pool, percent, price, quantity, exchange_rate,
                                        commission_buy, tax_buy, commission_sell, tax_sell]
            .iter().map(|value| value.trim().parse::<f64>().ok()).collect();
        inputs.and_then(|values| data::calculate_trade_stoploss(
            values[1], values[0], values[2], values[3], values[4],
            values[5] + values[6] + values[7] + values[8], side == "Long",
        ).ok())
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "—".into()).into()
    });

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_trade(
        move |trade_id,
              product,
              side,
              buy_date,
              sell_date,
              quantity,
              buy_price,
              sell_price,
              exchange_rate_buy,
              exchange_rate_sell,
              commission_buy,
              tax_buy,
              commission_sell,
              tax_sell,
              other_costs,
              risk,
              risk_pool,
              filter| {
            if let Some(window) = weak.upgrade()
            {
                let result = (|| -> Result<(), String> {
                    let product = product.trim();
                    if !model
                        .borrow()
                        .products
                        .iter()
                        .any(|item| item.name == product)
                    {
                        return Err("Select a product by name".into());
                    }
                    check_date(&buy_date, "Buy date", false)?;
                    check_date(&sell_date, "Sell date", true)?;
                    if !sell_date.is_empty() && sell_date < buy_date
                    {
                        return Err("Sell date must be on or after buy date".into());
                    }
                    let quantity = positive(&quantity, "Quantity")?;
                    let buy_price = nonnegative(&buy_price, "Buy price")?;
                    let sell_price = nonnegative(&sell_price, "Sell price")?;
                    let exchange_rate_buy = positive(&exchange_rate_buy, "Buy exchange rate")?;
                    let exchange_rate_sell = positive(&exchange_rate_sell, "Sell exchange rate")?;
                    let commission_buy = nonnegative(&commission_buy, "Commission buy")?;
                    let tax_buy = nonnegative(&tax_buy, "Tax buy")?;
                    let commission_sell = nonnegative(&commission_sell, "Commission sell")?;
                    let tax_sell = nonnegative(&tax_sell, "Tax sell")?;
                    let other_costs = nonnegative(&other_costs, "Other costs")?;
                    let risk_percent = if risk.trim().is_empty()
                    {
                        None
                    }
                    else
                    {
                        Some(nonnegative(&risk, "Risk %")?)
                    };
                    let risk_pool = if risk_percent.is_none() || risk_pool.trim().is_empty()
                    {
                        None
                    }
                    else
                    {
                        Some(nonnegative(&risk_pool, "Risk pool")?)
                    };
                    if trade_id == 0 && (risk_percent.is_none() || risk_pool.is_none())
                    {
                        return Err("Enter risk % and a pool value".into());
                    }
                    if risk_percent.is_some() != risk_pool.is_some()
                    {
                        return Err("Risk % requires a pool value".into());
                    }
                    let initial_risk = if trade_id == 0
                    {
                        0.0
                    }
                    else
                    {
                        model
                            .borrow()
                            .trades
                            .iter()
                            .find(|trade| trade.id == trade_id as i64)
                            .map(|trade| trade.initial_risk)
                            .ok_or("Selected trade no longer exists")?
                    };
                    let is_long = side == "Long";
                    let trade = NewTrade {
                        product: product.into(),
                        buy_date: buy_date.into(),
                        sell_date: if sell_date.is_empty()
                        {
                            None
                        }
                        else
                        {
                            Some(sell_date.into())
                        },
                        is_long,
                        buy_price,
                        sell_price,
                        exchange_rate_buy,
                        exchange_rate_sell,
                        quantity,
                        commission_buy,
                        tax_buy,
                        commission_sell,
                        tax_sell,
                        other_costs,
                        initial_risk,
                        risk_percent,
                        risk_pool,
                    };
                    if trade_id == 0
                    {
                        database.borrow_mut().add_trade(trade)?;
                    }
                    else
                    {
                        database.borrow_mut().update_trade(trade_id as i64, trade)?;
                    }
                    *model.borrow_mut() = database.borrow().load()?;
                    Ok(())
                })();
                match result
                {
                    Ok(()) =>
                    {
                        refresh(&window, &model.borrow(), &filter);
                        window.set_status(
                            if trade_id == 0
                            {
                                "Trade saved"
                            }
                            else
                            {
                                "Trade updated"
                            }
                            .into(),
                        );
                        true
                    }
                    Err(error) =>
                    {
                        window.set_status(error.into());
                        false
                    }
                }
            }
            else
            {
                false
            }
        },
    );

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_financing(
        move |trade_id, date, quantity, price, rate, exchange_rate, days, value, note| {
            if let Some(window) = weak.upgrade()
            {
                let result = (|| -> Result<(), String> {
                    let trade_id: i64 = trade_id
                        .trim()
                        .parse()
                        .map_err(|_| "Trade ID must be a positive integer")?;
                    check_date(&date, "Date", false)?;
                    let quantity = positive(&quantity, "Quantity")?;
                    let price = nonnegative(&price, "Price")?;
                    let rate = nonnegative(&rate, "Rate")?;
                    let exchange_rate = positive(&exchange_rate, "Exchange rate")?;
                    let days: i64 = days
                        .trim()
                        .parse()
                        .map_err(|_| "Days must be a positive integer")?;
                    if days == 0
                    {
                        return Err("Days must be greater than zero".into());
                    }
                    let price_eur = convert_from_orig(price, exchange_rate);
                    let calculated = quantity * price_eur * rate / 100.0 * days as f64 / 360.0;
                    let value = if value.trim().is_empty()
                    {
                        calculated
                    }
                    else
                    {
                        nonnegative(&value, "Value")?
                    };
                    if !value.is_finite()
                    {
                        return Err("Calculated value is too large".into());
                    }
                    database.borrow_mut().add_financing(NewFinancing {
                        trade_id,
                        date: date.into(),
                        quantity,
                        price,
                        rate,
                        exchange_rate,
                        days,
                        value,
                        note: note.into(),
                    })?;
                    *model.borrow_mut() = database.borrow().load()?;
                    Ok(())
                })();
                match result
                {
                    Ok(()) =>
                    {
                        refresh(&window, &model.borrow(), &window.get_selected_product());
                        window.set_status("Financing entry saved".into());
                    }
                    Err(error) => window.set_status(error.into()),
                }
            }
        },
    );

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_product(move |name, description, currency, market| {
        if let Some(window) = weak.upgrade()
        {
            let result = (|| -> Result<(), String> {
                database
                    .borrow_mut()
                    .add_product(&name, &description, &currency, &market)?;
                *model.borrow_mut() = database.borrow().load()?;
                Ok(())
            })();
            match result
            {
                Ok(()) =>
                {
                    set_products(&window, &model.borrow());
                    refresh(&window, &model.borrow(), &window.get_selected_product());
                    window.set_status("Product saved".into());
                }
                Err(error) => window.set_status(error.into()),
            }
        }
    });

    window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
    window.window().set_maximized(true);
    window.run()
}

fn set_products(window: &AppWindow, journal: &Journal)
{
    let products: Vec<_> = journal.products.iter().map(|p| p.name.clone()).collect();
    let mut filters = vec!["All products".to_string()];
    filters.extend(products.iter().cloned());
    window.set_products(strings(products));
    window.set_filters(strings(filters));
    let details: Vec<_> = journal
        .products
        .iter()
        .map(|product| ProductRow {
            name: product.name.clone().into(),
            description: product.description.clone().into(),
            currency: product.currency.clone().into(),
            market: product.market.clone().into(),
        })
        .collect();
    window.set_product_details(ModelRc::from(Rc::new(VecModel::from(details))));
}

fn strings(values: Vec<String>) -> ModelRc<SharedString>
{
    ModelRc::from(Rc::new(VecModel::from(
        values.into_iter().map(Into::into).collect::<Vec<_>>(),
    )))
}

fn refresh(window: &AppWindow, journal: &Journal, product: &str)
{
    window.set_current_pool(
        journal
            .pool_value
            .map(|value| format!("{value:.2}"))
            .unwrap_or_default()
            .into(),
    );
    let matches = |trade: &&Trade| product == "All products" || trade.product == product;
    let selected: Vec<_> = journal.trades.iter().filter(matches).collect();
    let closed: Vec<_> = selected
        .iter()
        .filter(|trade| trade.profit_loss.is_some())
        .collect();
    let wins = closed
        .iter()
        .filter(|trade| trade.profit_loss.unwrap_or(0.0) >= 0.0)
        .count();
    let r_values: Vec<_> = closed
        .iter()
        .filter_map(|trade| trade.r_multiple())
        .collect();
    let total_pl: f64 = closed.iter().filter_map(|trade| trade.profit_loss).sum();
    window.set_trade_count(closed.len().to_string().into());
    window.set_win_rate(
        if closed.is_empty()
        {
            "-".into()
        }
        else
        {
            format!("{:.1}%", 100.0 * wins as f64 / closed.len() as f64).into()
        },
    );
    window.set_expectancy(
        if r_values.is_empty()
        {
            "-".into()
        }
        else
        {
            format!(
                "{:.3} R",
                r_values.iter().sum::<f64>() / r_values.len() as f64
            )
            .into()
        },
    );
    window.set_total_pl(format!("{total_pl:+.2}").into());
    let rows: Vec<_> = selected
        .iter()
        .rev()
        .map(|trade| TradeRow {
            id: trade.id as i32,
            closed: trade.profit_loss.is_some(),
            product: trade.product.clone().into(),
            side: (if trade.is_long { "Long" } else { "Short" }).into(),
            dates: format!(
                "{} → {}",
                trade.buy_date,
                if trade.sell_date.is_empty()
                {
                    "open"
                }
                else
                {
                    &trade.sell_date
                }
            )
            .into(),
            buy_date: trade.buy_date.clone().into(),
            sell_date: trade.sell_date.clone().into(),
            quantity: format_number(trade.quantity).into(),
            edit_quantity: trade.quantity.to_string().into(),
            buy_price: trade.buy_price.to_string().into(),
            sell_price: trade.sell_price.to_string().into(),
            exchange_rate_buy: trade.exchange_rate_buy.to_string().into(),
            exchange_rate_sell: trade.exchange_rate_sell.to_string().into(),
            commission_buy: trade.commission_buy.to_string().into(),
            tax_buy: trade.tax_buy.to_string().into(),
            commission_sell: trade.commission_sell.to_string().into(),
            tax_sell: trade.tax_sell.to_string().into(),
            other_costs: trade.other_costs.to_string().into(),
            risk_percent: trade
                .risk_percent
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            risk_pool: trade
                .risk_pool
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            risk_initial: format!("{:.2}", trade.initial_risk).into(),
            risk_actual: trade
                .displayed_actual_risk()
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "—".into())
                .into(),
            profit: trade
                .profit_loss
                .map(|n| format!("{n:+.2}"))
                .unwrap_or_else(|| "—".into())
                .into(),
            r: trade
                .r_multiple()
                .map(|n| format!("{n:+.2} R"))
                .unwrap_or_else(|| "-".into())
                .into(),
        })
        .collect();
    window.set_trades(ModelRc::from(Rc::new(VecModel::from(rows))));
    let finance: Vec<_> = journal
        .financing
        .iter()
        .filter(|entry| {
            product == "All products"
                || journal
                    .trades
                    .iter()
                    .any(|t| t.id == entry.trade_id && t.product == product)
        })
        .collect();
    let financing_total: f64 = finance.iter().map(|entry| entry.value).sum();
    window.set_financing_total(format!("{financing_total:.2}").into());
    let finance_rows: Vec<_> = finance
        .iter()
        .rev()
        .map(|entry| FinancingRow {
            id: entry.id as i32,
            trade: format!("Trade #{}", entry.trade_id).into(),
            date: entry.date.clone().into(),
            terms: format!(
                "{} × {:.2} · {:.4}% · {} d",
                format_number(entry.quantity),
                entry.price,
                entry.rate,
                entry.days
            )
            .into(),
            value: format!("{:.2}", entry.value).into(),
            note: entry.note.clone().into(),
        })
        .collect();
    window.set_financing(ModelRc::from(Rc::new(VecModel::from(finance_rows))));
}

fn format_number(value: f64) -> String
{
    if value.fract() == 0.0
    {
        format!("{value:.0}")
    }
    else
    {
        format!("{value:.2}")
    }
}

fn nonnegative(input: &str, label: &str) -> Result<f64, String>
{
    let value: f64 = input
        .trim()
        .parse()
        .map_err(|_| format!("{label} must be a number"))?;
    if !value.is_finite() || value < 0.0
    {
        return Err(format!("{label} must be a nonnegative finite number"));
    }
    Ok(value)
}

fn positive(input: &str, label: &str) -> Result<f64, String>
{
    let value = nonnegative(input, label)?;
    if value == 0.0
    {
        return Err(format!("{label} must be greater than zero"));
    }
    Ok(value)
}

fn check_date(input: &str, label: &str, optional: bool) -> Result<(), String>
{
    if optional && input.is_empty()
    {
        return Ok(());
    }
    let bytes = input.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err(format!("{label} must use YYYY-MM-DD"));
    }
    let year: u16 = input[0..4].parse().unwrap_or(0);
    let month: u8 = input[5..7].parse().unwrap_or(0);
    let day: u8 = input[8..10].parse().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month
    {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day
    {
        return Err(format!("{label} is not a valid date"));
    }
    Ok(())
}

#[cfg(test)]
mod test;
