mod data;
mod util;
#[path = "calculatorfinance-lib.rs"]
mod calculatorfinance_lib;

use data::{Journal, NewFinancing, NewTrade, Store, Trade};
use calculatorfinance_lib::{trade_type};
use libcalculatorfinance::{calculate_risk_initial, calculate_stoploss_initial, calculate_risk_percentage_initial, calculate_risk_percentage, convert_from_orig};
use util::{check_date, date_in_range, format_number, validated_as_positive};
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError>
{
    /* Setup */

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
    set_markets(&window, &journal.borrow());
    set_parameters(&window, &journal.borrow());

    refresh(&window, &journal.borrow());

    window.set_status(
        format!(
            "Loaded {} trades and {} financing entries",
            journal.borrow().trades.len(),
            journal.borrow().financing.len()
        )
        .into(),
    );

    /* Event: Filter journal */

    let weak = window.as_weak();
    let model = journal.clone();
    window.on_filter_journal(move |from, to|
    {
        if let Some(window) = weak.upgrade()
        {
            match validate_date_range(&from, &to)
            {
                Ok(()) =>
                {
                    window.set_journal_date_from(from);
                    window.set_journal_date_to(to);
                    refresh(&window, &model.borrow());
                    window.set_status("Trade journal updated".into());
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
    });

    /* Event: Filter trade costs */

    let weak = window.as_weak();
    let model = journal.clone();
    window.on_filter_trade_costs(move |from, to|
    {
        if let Some(window) = weak.upgrade()
        {
            match validate_date_range(&from, &to)
            {
                Ok(()) =>
                {
                    window.set_costs_date_from(from.clone());
                    window.set_costs_date_to(to.clone());
                    refresh_trade_costs(&window, &model.borrow(), &from, &to);
                    window.set_status("Trade costs updated".into());
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
    });

    /* Event: on_calculate_risk_initial*/

    window.on_calculate_risk_initial(
        |stoploss,
         price_buy,
         price_sell,
         shares_buy,
         shares_sell,
         commission_buy,
         commission_sell,
         tax_buy,
         tax_sell,
         side| {
            let inputs: Option<Vec<f64>> = [
                stoploss,
                price_buy,
                price_sell,
                shares_buy,
                shares_sell,
                commission_buy,
                commission_sell,
                tax_buy,
                tax_sell,
            ]
            .iter()
            .map(|value| value.trim().parse::<f64>().ok())
            .collect();
            inputs
                .and_then(|values| {
                    Some(calculate_risk_initial(
                        values[1],
                        values[3] as i32,
                        values[7],
                        values[5],
                        values[2],
                        values[4] as i32,
                        values[8],
                        values[6],
                        values[0],
                        trade_type(side == "Long"),
                    ))
                })
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".into())
                .into()
        },
    );

    /* Event: on_calculate_stoploss_initial */
    
    window.on_calculate_stoploss_initial(
        |pool,
         risk_percent,
         price_buy,
         price_sell,
         shares_buy,
         shares_sell,
         commission_buy,
         commission_sell,
         tax_buy,
         tax_sell,
         side| {
            let inputs: Option<Vec<f64>> = [
                pool,
                risk_percent,
                price_buy,
                price_sell,
                shares_buy,
                shares_sell,
                commission_buy,
                commission_sell,
                tax_buy,
                tax_sell,
            ]
            .iter()
            .map(|value| value.trim().parse::<f64>().ok())
            .collect();
            inputs
                .and_then(|values| {
                    if values[0] <= 0.0 || values[1] < 0.0
                    {
                        return None;
                    }
                    Some(calculate_stoploss_initial(
                        values[2],
                        values[4] as i32,
                        values[8],
                        values[6],
                        values[3],
                        values[5] as i32,
                        values[9],
                        values[7],
                        values[1],
                        values[0],
                        trade_type(side == "Long"),
                    ))
                })
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".into())
                .into()
        },
    );

    /* Event: on_calculate_risk_percentage_initial */

    window.on_calculate_risk_percentage_initial(
        |price_buy,
         shares_buy,
         tax_buy,
         commission_buy,
         price_sell,
         shares_sell,
         tax_sell,
         commission_sell,
         stoploss,
         pool,
         side| {
            let inputs: Option<Vec<f64>> = [
                price_buy,
                shares_buy,
                tax_buy,
                commission_buy,
                price_sell,
                shares_sell,
                tax_sell,
                commission_sell,
                stoploss,
                pool,
            ]
            .iter()
            .map(|value| value.trim().parse::<f64>().ok())
            .collect();
            inputs
                .and_then(|values| {
                    if values[9] <= 0.0
                    {
                        return None;
                    }
                    Some(calculate_risk_percentage_initial(
                        values[0],
                        values[1] as i32,
                        values[2],
                        values[3],
                        values[4],
                        values[5] as i32,
                        values[6],
                        values[7],
                        values[8],
                        values[9],
                        trade_type(side == "Long"),
                    ))
                })
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".into())
                .into()
        },
    );

    /* Event: on_save_trade */

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_trade(
        move |trade_id,
              product,
              side,
              date_buy,
              date_sell,
              shares_buy,
              shares_sell,
              price_buy,
              price_sell,
              exchange_rate_buy,
              exchange_rate_sell,
              commission_buy,
              tax_buy,
              commission_sell,
              tax_sell,
              cost_other,
              risk,
              stoploss,
              trade_pool| {
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
                    check_date(&date_buy, "Buy date", false)?;
                    check_date(&date_sell, "Sell date", true)?;
                    if !date_sell.is_empty() && date_sell < date_buy
                    {
                        return Err("Sell date must be on or after buy date".into());
                    }
                    let shares_buy = validated_as_positive(&shares_buy, "Shares buy")? as i32;
                    let shares_sell = validated_as_positive(&shares_sell, "Shares sell")? as i32;
                    let price_buy = validated_as_positive(&price_buy, "Buy price")?;
                    let price_sell = validated_as_positive(&price_sell, "Sell price")?;
                    let exchange_rate_buy = validated_as_positive(&exchange_rate_buy, "Buy exchange rate")?;
                    let exchange_rate_sell = validated_as_positive(&exchange_rate_sell, "Sell exchange rate")?;
                    let commission_buy = validated_as_positive(&commission_buy, "Commission buy")?;
                    let tax_buy = validated_as_positive(&tax_buy, "Tax buy")?;
                    let commission_sell = validated_as_positive(&commission_sell, "Commission sell")?;
                    let tax_sell = validated_as_positive(&tax_sell, "Tax sell")?;
                    let cost_other = validated_as_positive(&cost_other, "Other costs")?;
                    let stoploss_initial = if stoploss.trim().is_empty()
                    {
                        None
                    }
                    else
                    {
                        Some(validated_as_positive(&stoploss, "Stoploss")?)
                    };
                    let risk_percent_entered = if risk.trim().is_empty()
                    {
                        None
                    }
                    else
                    {
                        Some(validated_as_positive(&risk, "Risk %")?)
                    };
                    if risk_percent_entered.is_none() && stoploss_initial.is_none()
                    {
                        return Err("Enter risk % or stoploss".into());
                    }
                    let trade_pool = validated_as_positive(&trade_pool, "Risk pool")?;
                    if trade_pool == 0.0
                    {
                        return Err("Risk pool must be greater than zero".into());
                    }
                    let is_long = side == "Long";
                    let stoploss_initial = stoploss_initial.or_else(|| risk_percent_entered.map(|risk_percent| {
                        calculate_stoploss_initial(
                            price_buy, shares_buy, tax_buy, commission_buy,
                            price_sell, shares_sell, tax_sell, commission_sell,
                            risk_percent, trade_pool, trade_type(is_long),
                        )
                    })).ok_or("Enter risk % or stoploss")?;
                    if !stoploss_initial.is_finite() || stoploss_initial < 0.0
                    {
                        return Err("Risk % produces an invalid stoploss".into());
                    }
                    let risk_initial = calculate_risk_initial(
                        price_buy, shares_buy, tax_buy, commission_buy,
                        price_sell, shares_sell, tax_sell, commission_sell,
                        stoploss_initial, trade_type(is_long),
                    );
                    if !risk_initial.is_finite() || risk_initial < 0.0
                    {
                        return Err("Stoploss produces an invalid initial risk".into());
                    }
                    let risk_percent_initial = calculate_risk_percentage_initial(
                        price_buy, shares_buy, tax_buy, commission_buy,
                        price_sell, shares_sell, tax_sell, commission_sell,
                        stoploss_initial, trade_pool, trade_type(is_long),
                    );
                    if !risk_percent_initial.is_finite() || risk_percent_initial < 0.0
                    {
                        return Err("Stoploss produces an invalid risk percentage".into());
                    }
                    let risk_percent_actual = if date_sell.is_empty()
                    {
                        None
                    }
                    else
                    {
                        Some(calculate_risk_percentage(price_buy, shares_buy, tax_buy, commission_buy, price_sell, shares_sell, tax_sell, commission_sell, stoploss_initial, trade_pool, trade_type(is_long)))
                    };
                    let trade_pool = if trade_id == 0
                    {
                        None
                    }
                    else
                    {
                        Some(trade_pool)
                    };
                    let trade = NewTrade {
                        product: product.into(),
                        date_buy: date_buy.into(),
                        date_sell: if date_sell.is_empty()
                        {
                            None
                        }
                        else
                        {
                            Some(date_sell.into())
                        },
                        is_long,
                        shares_buy,
                        shares_sell,
                        price_buy,
                        price_sell,
                        exchange_rate_buy,
                        exchange_rate_sell,
                        commission_buy,
                        tax_buy,
                        commission_sell,
                        tax_sell,
                        cost_other,
                        risk_initial,
                        risk_percent_initial: Some(risk_percent_initial),
                        risk_percent_actual,
                        trade_pool,
                        stoploss_initial: Some(stoploss_initial),
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
                        refresh(&window, &model.borrow());
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

    /* Event: on_save_financing */

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_financing(
        move |trade_id, date, quantity, price, rate, exchange_rate, days, value, comment| {
            if let Some(window) = weak.upgrade()
            {
                let result = (|| -> Result<(), String> {
                    let trade_id: i64 = trade_id
                        .trim()
                        .parse()
                        .map_err(|_| "Trade ID must be a positive integer")?;
                    check_date(&date, "Date", false)?;
                    let quantity = validated_as_positive(&quantity, "Quantity")? as i32;
                    let price = validated_as_positive(&price, "Price")?;
                    let rate = validated_as_positive(&rate, "Rate")?;
                    let exchange_rate = validated_as_positive(&exchange_rate, "Exchange rate")?;
                    let days: i64 = days
                        .trim()
                        .parse()
                        .map_err(|_| "Days must be a positive integer")?;
                    if days == 0
                    {
                        return Err("Days must be greater than zero".into());
                    }
                    let price_eur = convert_from_orig(price, exchange_rate);
                    let calculated = quantity as f64 * price_eur * rate / 100.0 * days as f64 / 360.0;
                    let value = if value.trim().is_empty()
                    {
                        calculated
                    }
                    else
                    {
                        validated_as_positive(&value, "Value")?
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
                        comment: comment.into(),
                    })?;
                    *model.borrow_mut() = database.borrow().load()?;
                    Ok(())
                })();
                match result
                {
                    Ok(()) =>
                    {
                        refresh(&window, &model.borrow());
                        window.set_status("Financing entry saved".into());
                    }
                    Err(error) => window.set_status(error.into()),
                }
            }
        },
    );

    /* Event: on_save_product */

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
                    refresh(&window, &model.borrow());
                    window.set_status("Product saved".into());
                }
                Err(error) => window.set_status(error.into()),
            }
        }
    });

    /* Event: on_save_market */

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_market(move |code, name, country| {
        if let Some(window) = weak.upgrade()
        {
            let result = (|| -> Result<(), String> {
                database.borrow_mut().add_market(&code, &name, &country)?;
                *model.borrow_mut() = database.borrow().load()?;
                Ok(())
            })();
            match result
            {
                Ok(()) =>
                {
                    set_markets(&window, &model.borrow());
                    window.set_status("Market saved".into());
                }
                Err(error) => window.set_status(error.into()),
            }
        }
    });

    /* Event: on_save_parameter */

    let weak = window.as_weak();
    let model = journal.clone();
    let database = store.clone();
    window.on_save_parameter(move |id, name, value, value_default, description, datatype| {
        if let Some(window) = weak.upgrade()
        {
            let result = (|| -> Result<(), String> {
                database.borrow_mut().save_parameter(
                    id as i64,
                    &name,
                    &value,
                    &value_default,
                    &description,
                    &datatype,
                )?;
                *model.borrow_mut() = database.borrow().load()?;
                Ok(())
            })();
            match result
            {
                Ok(()) =>
                {
                    set_parameters(&window, &model.borrow());
                    window.set_status("Parameter saved".into());
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
    });

    /* Event: on_quit */

    window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    /* UI startup */

    window.window().set_maximized(true);
    window.run()
}

fn set_products(window: &AppWindow, journal: &Journal)
{
    let products: Vec<_> = journal.products.iter().map(|p| p.name.clone()).collect();
    window.set_products(strings(products));
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

fn set_markets(window: &AppWindow, journal: &Journal)
{
    window.set_market_codes(strings(
        journal
            .markets
            .iter()
            .map(|market| market.code.clone())
            .collect(),
    ));
    let details: Vec<_> = journal
        .markets
        .iter()
        .map(|market| MarketRow {
            code: market.code.clone().into(),
            name: market.name.clone().into(),
            country: market.country.clone().into(),
        })
        .collect();
    window.set_market_details(ModelRc::from(Rc::new(VecModel::from(details))));
}

fn set_parameters(window: &AppWindow, journal: &Journal)
{
    window.set_base_currency(
        journal.parameters.iter()
            .find(|parameter| parameter.name == "base_currency")
            .map(|parameter| parameter.value.as_str())
            .unwrap_or_default()
            .into(),
    );
    let rows: Vec<_> = journal.parameters.iter().map(|parameter| ParameterRow {
        id: parameter.id as i32,
        name: parameter.name.clone().into(),
        value: parameter.value.clone().into(),
        value_default: parameter.value_default.clone().into(),
        description: parameter.description.clone().into(),
        datatype: parameter.datatype.clone().into(),
    }).collect();
    window.set_parameters(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn strings(values: Vec<String>) -> ModelRc<SharedString>
{
    ModelRc::from(Rc::new(VecModel::from(
        values.into_iter().map(Into::into).collect::<Vec<_>>(),
    )))
}

fn refresh(window: &AppWindow, journal: &Journal)
{
    window.set_current_pool_exact(
        journal
            .pool_value_converted
            .map(|value| value.to_string())
            .unwrap_or_default()
            .into(),
    );
    window.set_current_pool(
        journal
            .pool_value_converted
            .map(|value| format!("{value:.2}"))
            .unwrap_or_default()
            .into(),
    );
    let from = window.get_journal_date_from();
    let to = window.get_journal_date_to();
    let selected: Vec<_> = journal
        .trades
        .iter()
        .filter(|trade| date_in_range(&trade.date_buy, &from, &to))
        .collect();
    let is_closed: Vec<_> = selected
        .iter()
        .filter(|trade| trade.profit_loss.is_some())
        .collect();
    let wins = is_closed
        .iter()
        .filter(|trade| trade.profit_loss.unwrap_or(0.0) >= 0.0)
        .count();
    let r_values: Vec<_> = is_closed
        .iter()
        .filter_map(|trade| trade.r_multiple())
        .collect();
    let total_pl: f64 = is_closed
        .iter()
        .filter_map(|trade| trade.profit_loss_total)
        .sum();
    window.set_trade_count(is_closed.len().to_string().into());
    window.set_win_rate(
        if is_closed.is_empty()
        {
            "-".into()
        }
        else
        {
            format!("{:.1}%", 100.0 * wins as f64 / is_closed.len() as f64).into()
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
            is_closed: trade.profit_loss.is_some(),
            product: trade.product.clone().into(),
            side: (if trade.is_long { "Long" } else { "Short" }).into(),
            dates: format!(
                "{} → {}",
                trade.date_buy,
                if trade.date_sell.is_empty()
                {
                    "open"
                }
                else
                {
                    &trade.date_sell
                }
            )
            .into(),
            date_buy: trade.date_buy.clone().into(),
            date_sell: trade.date_sell.clone().into(),
            shares_buy: format_number(trade.shares_buy.into()).into(),
            shares_sell: format_number(trade.shares_sell.into()).into(),
            edit_shares_buy: trade.shares_buy.to_string().into(),
            edit_shares_sell: trade.shares_sell.to_string().into(),
            price_buy: trade.price_buy.to_string().into(),
            price_sell: trade.price_sell.to_string().into(),
            exchange_rate_buy: trade.exchange_rate_buy.to_string().into(),
            exchange_rate_sell: trade.exchange_rate_sell.to_string().into(),
            commission_buy: trade.commission_buy.to_string().into(),
            tax_buy: trade.tax_buy.to_string().into(),
            commission_sell: trade.commission_sell.to_string().into(),
            tax_sell: trade.tax_sell.to_string().into(),
            cost_other: trade.cost_other.to_string().into(),
            risk_percent_initial: trade
                .risk_percent_initial
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            risk_percent_actual: trade
                .risk_percent_actual
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            stoploss_initial: trade
                .stoploss_initial
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            stoploss_actual: trade
                .stoploss_actual
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            trade_pool: trade
                .trade_pool
                .map(|value| value.to_string())
                .unwrap_or_default()
                .into(),
            risk_initial: trade.risk_initial.to_string().into(),
            risk_actual: trade
                .risk_actual
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "—".into())
                .into(),
            profit: trade
                .profit_loss_total
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
    let finance: Vec<_> = journal.financing.iter().collect();
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
                format_number(entry.quantity.into()),
                entry.price,
                entry.rate,
                entry.days
            )
            .into(),
            value: format!("{:.2}", entry.value).into(),
            comment: entry.comment.clone().into(),
        })
        .collect();
    window.set_financing(ModelRc::from(Rc::new(VecModel::from(finance_rows))));
    refresh_trade_costs(
        window,
        journal,
        &window.get_costs_date_from(),
        &window.get_costs_date_to(),
    );
}

fn validate_date_range(from: &str, to: &str) -> Result<(), String>
{
    check_date(from, "From date", true)?;
    check_date(to, "To date", true)?;
    if !from.is_empty() && !to.is_empty() && from > to
    {
        return Err("From date must be on or before to date".into());
    }
    Ok(())
}

fn refresh_trade_costs(window: &AppWindow, journal: &Journal, from: &str, to: &str)
{
    let selected: Vec<_> = journal
        .trades
        .iter()
        .filter(|trade| date_in_range(&trade.date_buy, from, to))
        .collect();
    let sum = |cost: fn(&Trade) -> f64| -> f64 { selected.iter().map(|trade| cost(trade)).sum() };
    let commission_buy = sum(|trade| trade.commission_buy);
    let tax_buy = sum(|trade| trade.tax_buy);
    let commission_sell = sum(|trade| trade.commission_sell);
    let tax_sell = sum(|trade| trade.tax_sell);
    let other = sum(|trade| trade.cost_other);
    window.set_costs_count(selected.len().to_string().into());
    window.set_costs_commission_buy(format!("{commission_buy:.2}").into());
    window.set_costs_tax_buy(format!("{tax_buy:.2}").into());
    window.set_costs_commission_sell(format!("{commission_sell:.2}").into());
    window.set_costs_tax_sell(format!("{tax_sell:.2}").into());
    window.set_costs_other(format!("{other:.2}").into());
    window.set_costs_total(
        format!(
            "{:.2}",
            commission_buy + tax_buy + commission_sell + tax_sell + other
        )
        .into(),
    );
    let rows: Vec<_> = selected
        .iter()
        .rev()
        .map(|trade| TradeCostRow {
            id: trade.id as i32,
            product: trade.product.clone().into(),
            date_buy: trade.date_buy.clone().into(),
            date_sell: trade.date_sell.clone().into(),
            commission_buy: format!("{:.2}", trade.commission_buy).into(),
            tax_buy: format!("{:.2}", trade.tax_buy).into(),
            commission_sell: format!("{:.2}", trade.commission_sell).into(),
            tax_sell: format!("{:.2}", trade.tax_sell).into(),
            other: format!("{:.2}", trade.cost_other).into(),
            total: format!(
                "{:.2}",
                trade.commission_buy
                    + trade.tax_buy
                    + trade.commission_sell
                    + trade.tax_sell
                    + trade.cost_other
            )
            .into(),
        })
        .collect();
    window.set_trade_costs(ModelRc::from(Rc::new(VecModel::from(rows))));
}

#[cfg(test)]
mod test;
