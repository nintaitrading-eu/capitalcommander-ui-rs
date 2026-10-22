use crate::calculatorfinance_lib::trade_type;
use crate::data::{NewFinancing, NewTrade, Store, Trade};
use crate::check_date;
use libcalculatorfinance::{calculate_risk_percentage_initial, calculate_stoploss_initial};
use std::path::Path;

#[test]
fn min_target_uses_entry_side_and_hides_incomplete_input()
{
    let long = crate::min_target(["100", "10", "5", "10"], true).unwrap();
    let short = crate::min_target(["100", "10", "5", "10"], false).unwrap();
    assert!((long - 102.10526315789474).abs() < 1e-10);
    assert!((short - 97.89473684210526).abs() < 1e-10);
    assert_eq!(crate::min_target(["", "10", "5", "10"], true), None);
    assert_eq!(crate::min_target(["100", "0", "5", "10"], false), None);
}

#[test]
fn valid_r_multiple_requires_positive_risk()
{
    let mut trade = Trade {
        id: 1,
        product: "A".into(),
        date_buy: "2026-01-01".into(),
        date_sell: "2026-01-02".into(),
        is_long: true,
        shares_buy: 1,
        shares_sell: 1,
        price_buy: 1.0,
        price_sell: 2.0,
        exchange_rate_buy: 1.0,
        exchange_rate_sell: 1.0,
        commission_buy: 0.0,
        tax_buy: 0.0,
        commission_sell: 0.0,
        tax_sell: 0.0,
        cost_other: 0.0,
        risk_initial: 10.0,
        risk_initial_converted: 10.0,
        risk_actual: Some(10.0),
        risk_actual_converted: Some(10.0),
        risk_percent_initial: None,
        risk_percent_actual: None,
        trade_pool: None,
        trade_pool_converted: None,
        stoploss_initial: None,
        stoploss_actual: None,
        profit_loss: Some(-5.0),
        profit_loss_converted: Some(-5.0),
        profit_loss_total: None,
        profit_loss_total_converted: None,
    };
    assert_eq!(trade.r_multiple(), Some(-0.5));
    trade.risk_initial = 0.0;
    assert_eq!(trade.r_multiple(), None);
}

#[test]
fn rejects_invalid_calendar_date()
{
    assert!(check_date("2026-02-29", "Date", false).is_err());
    assert!(check_date("2024-02-29", "Date", false).is_ok());
}

#[test]
fn stoploss_follows_risk_input_for_both_trade_sides()
{
    assert_eq!(
        calculate_stoploss_initial(12.0, 2, 0.0, 0.0, 12.0, 2, 0.0, 0.0, 1.0, 400.0, trade_type(true)),
        10.0
    );
    assert_eq!(
        calculate_stoploss_initial(12.0, 2, 0.0, 0.0, 12.0, 2, 0.0, 0.0, 1.0, 400.0, trade_type(false)),
        14.0
    );
}

#[test]
fn risk_percent_follows_entered_stoploss_for_both_trade_sides()
{
    assert_eq!(
        calculate_risk_percentage_initial(12.0, 2, 0.0, 0.0, 12.0, 2, 0.0, 0.0, 10.0, 400.0, trade_type(true)),
        1.0
    );
    assert_eq!(
        calculate_risk_percentage_initial(12.0, 2, 0.0, 0.0, 12.0, 2, 0.0, 0.0, 14.0, 400.0, trade_type(false)),
        1.0
    );
}

#[test]
fn saves_entered_stoploss_and_derived_risk()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let stoploss = 11.25;
    let risk_percent = calculate_risk_percentage_initial(
        12.0, 2, 0.0, 1.0, 0.0, 2, 0.0, 1.0, stoploss, 75000.0, trade_type(true),
    );
    store
        .add_trade(NewTrade {
            product: ".MGOLD.cfd".into(),
            date_buy: "2026-09-28".into(),
            date_sell: None,
            is_long: true,
            shares_buy: 2,
            shares_sell: 2,
            price_buy: 12.0,
            price_sell: 0.0,
            exchange_rate_buy: 1.0,
            exchange_rate_sell: 1.0,
            commission_buy: 1.0,
            tax_buy: 0.0,
            commission_sell: 1.0,
            tax_sell: 0.0,
            cost_other: 0.0,
            risk_initial: 0.0,
            risk_percent_initial: Some(risk_percent),
            risk_percent_actual: None,
            trade_pool: None,
            stoploss_initial: Some(stoploss),
        })
        .unwrap();
    let trade = &store.load().unwrap().trades[0];
    assert_eq!(trade.stoploss_initial, Some(stoploss));
    assert_eq!(trade.risk_initial, 3.5);
    assert_eq!(trade.risk_percent_initial, Some(risk_percent));
}

#[test]
fn calculates_and_saves_trade_risks_when_closed()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let stoploss = calculate_stoploss_initial(
        12.0, 2, 0.25, 0.5, 0.0, 2, 0.25, 1.0, 1.0, 400.0, trade_type(true),
    );
    let open = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: None,
        is_long: true,
        shares_buy: 2,
        shares_sell: 2,
        price_buy: 12.0,
        price_sell: 0.0,
        exchange_rate_buy: 1.0,
        exchange_rate_sell: 1.0,
        commission_buy: 0.5,
        tax_buy: 0.25,
        commission_sell: 1.0,
        tax_sell: 0.25,
        cost_other: 0.0,
        risk_initial: 999.0,
        risk_percent_initial: Some(1.0),
        risk_percent_actual: None,
        trade_pool: Some(400.0),
        stoploss_initial: Some(stoploss),
    };
    store.add_trade(open).unwrap();
    let trade = &store.load().unwrap().trades[0];
    assert_eq!(trade.risk_initial, 4.0);
    assert_eq!(trade.risk_actual, None);
    assert_eq!(
        (trade.risk_percent_initial, trade.trade_pool),
        (Some(1.0), Some(400.0))
    );
    let linked_pool: (i64, f64) = store
        .connection
        .query_row(
            "SELECT t.pool_id, p.pool_value FROM t_trade t JOIN t_pool p ON p.pool_id = t.pool_id",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(linked_pool.1, 400.0);
    store
        .connection
        .execute("INSERT INTO t_pool(pool_value, pool_value_converted) VALUES (500, 500)", [])
        .unwrap();
    let id = trade.id;
    store
        .update_trade(
            id,
            NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-29".into()),
                is_long: true,
                shares_buy: 2,
                shares_sell: 2,
                price_buy: 12.0,
                price_sell: 8.0,
                exchange_rate_buy: 1.0,
                exchange_rate_sell: 1.0,
                commission_buy: 0.5,
                tax_buy: 0.25,
                commission_sell: 1.0,
                tax_sell: 0.25,
                cost_other: 0.0,
                risk_initial: 999.0,
                risk_percent_initial: Some(1.0),
                risk_percent_actual: None,
                trade_pool: Some(500.0),
                stoploss_initial: Some(stoploss),
            },
        )
        .unwrap();
    let trade = &store.load().unwrap().trades[0];
    assert_eq!(trade.risk_initial, 4.0);
    assert!((trade.risk_actual.unwrap() - 9.6).abs() < 1e-10);
    assert_eq!(trade.trade_pool, Some(linked_pool.1));
    let pool_id: i64 = store
        .connection
        .query_row("SELECT pool_id FROM t_trade", [], |row| row.get(0))
        .unwrap();
    assert_eq!(pool_id, linked_pool.0);
    let saved: (f64, Option<f64>) = store
        .connection
        .query_row(
            "SELECT risk_initial, risk_actual FROM t_trade_calculated",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(saved.0, 4.0);
    assert!((saved.1.unwrap() - 9.6).abs() < 1e-10);
    let calculated_columns: i64 = store.connection.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('t_trade_calculated') WHERE name IN ('risk_initial', 'risk_actual', 'risk_percent_initial', 'risk_percent_actual', 'stoploss_initial', 'stoploss_actual')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(calculated_columns, 6);
}

#[test]
fn loads_latest_pool_value()
{
    let store = Store::open(Path::new(":memory:")).unwrap();
    assert_eq!(store.load().unwrap().pool_value_converted, Some(75000.0));

    store
        .connection
        .execute("INSERT INTO t_pool(pool_value, pool_value_converted) VALUES (?1, ?1)", [82500.5])
        .unwrap();
    assert_eq!(store.load().unwrap().pool_value_converted, Some(82500.5));

    store.connection.execute("DELETE FROM t_pool", []).unwrap();
    assert_eq!(store.load().unwrap().pool_value_converted, None);
}

#[test]
fn adds_market_and_uses_it_for_products()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    assert!(store.add_market(" ", "Exchange", "BE").is_err());
    assert!(store.add_market("XBRU", " ", "BE").is_err());

    store
        .add_market(" XBRU ", " Euronext Brussels ", " BE ")
        .unwrap();
    let journal = store.load().unwrap();
    let market = journal
        .markets
        .iter()
        .find(|market| market.code == "XBRU")
        .unwrap();
    assert_eq!(market.name, "Euronext Brussels");
    assert_eq!(market.country, "BE");

    store
        .add_product("BRU.cfd", "Brussels product", "EUR", "XBRU")
        .unwrap();
    assert_eq!(
        store
            .load()
            .unwrap()
            .products
            .iter()
            .find(|product| product.name == "BRU.cfd")
            .unwrap()
            .market,
        "XBRU"
    );
    assert!(store.add_market("XBRU", "Duplicate", "BE").is_err());
}

#[test]
fn profit_and_loss_totals_include_all_trade_costs()
{
    // Totals: sale proceeds after 2% tax, less purchase after 1% tax,
    // less 0.5 and 0.25 commissions, plus 0.4 other cost.
    for (price_sell, expected_profit_loss, expected_total) in [
        (12.0, 4.0, 3.37),  // 23.52 - 19.8 - 0.75 + 0.4
        (8.0, -4.0, -4.47), // 15.68 - 19.8 - 0.75 + 0.4
    ]
    {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store
            .add_trade(NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-29".into()),
                is_long: true,
                shares_buy: 2,
                shares_sell: 2,
                price_buy: 10.0,
                price_sell,
                exchange_rate_buy: 1.0,
                exchange_rate_sell: 1.0,
                commission_buy: 0.5,
                tax_buy: 1.0,
                commission_sell: 0.25,
                tax_sell: 2.0,
                cost_other: 0.4,
                risk_initial: 2.0,
                risk_percent_initial: Some(0.0),
                risk_percent_actual: None,
                trade_pool: None,
                stoploss_initial: Some(9.0),
            })
            .unwrap();

        let trade = &store.load().unwrap().trades[0];
        assert_eq!(trade.profit_loss, Some(expected_profit_loss));
        assert!((trade.profit_loss_total.unwrap() - expected_total).abs() < 1e-10);
    }
}

#[test]
fn persists_relations_and_rejects_unknown_trade()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let initial = store.load().unwrap();
    assert!(initial.trades.is_empty());
    assert!(initial.products.iter().any(|p| p.name == ".MGOLD.cfd"));
    store
        .add_product("TEST.cfd", "Test contract", "EUR", "cfd other non-share")
        .unwrap();
    store
        .add_trade(NewTrade {
            product: "TEST.cfd".into(),
            date_buy: "2026-09-28".into(),
            date_sell: Some("2026-09-29".into()),
            is_long: true,
            shares_buy: 2,
            shares_sell: 2,
            price_buy: 10.0,
            price_sell: 12.0,
            exchange_rate_buy: 1.0,
            exchange_rate_sell: 1.0,
            commission_buy: 0.5,
            tax_buy: 0.25,
            commission_sell: 1.5,
            tax_sell: 0.35,
            cost_other: 0.25,
            risk_initial: 2.0,
            risk_percent_initial: Some(0.0),
            risk_percent_actual: None,
            trade_pool: None,
            stoploss_initial: Some(9.0),
        })
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades[0].product, "TEST.cfd");
    assert_eq!(journal.trades[0].shares_buy, 2);
    assert!((journal.trades[0].profit_loss_total.unwrap() - 2.216).abs() < 1e-10);
    let saved_costs: (f64, f64, f64) = store
        .connection
        .query_row(
            "SELECT commission_buy, tax_buy, other FROM t_trade_cost",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(saved_costs, (0.5, 0.25, 0.25));
    let trade_id = journal.trades[0].id;
    store
        .add_financing(NewFinancing {
            trade_id,
            date: "2026-09-29".into(),
            quantity: 2,
            price: 12.0,
            rate: 2.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.1,
            comment: "Test".into(),
        })
        .unwrap();
    let financing = store.load().unwrap().financing;
    assert_eq!(financing.len(), 1);
    assert_eq!(financing[0].trade_id, trade_id);
    assert_eq!(financing[0].comment, "Test");
    assert!(store
        .add_financing(NewFinancing {
            trade_id: 999,
            date: "2026-09-29".into(),
            quantity: 1,
            price: 1.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 1.0,
            comment: String::new(),
        })
        .is_err());
}

#[test]
fn closes_an_existing_trade_without_changing_its_id_or_financing()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let open_trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: None,
        is_long: true,
        shares_buy: 2,
        shares_sell: 2,
        price_buy: 10.0,
        price_sell: 11.0,
        exchange_rate_buy: 1.0,
        exchange_rate_sell: 1.0,
        commission_buy: 0.5,
        tax_buy: 0.25,
        commission_sell: 0.0,
        tax_sell: 0.0,
        cost_other: 0.0,
        risk_initial: 2.0,
        risk_percent_initial: Some(0.0),
        risk_percent_actual: None,
        trade_pool: None,
        stoploss_initial: Some(9.0),
    };
    store.add_trade(open_trade).unwrap();
    let id = store.load().unwrap().trades[0].id;
    assert_eq!(store.load().unwrap().trades[0].price_sell, 11.0);
    assert_eq!(store.load().unwrap().trades[0].profit_loss, None);
    store
        .add_financing(NewFinancing {
            trade_id: id,
            date: "2026-09-29".into(),
            quantity: 2,
            price: 10.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.1,
            comment: String::new(),
        })
        .unwrap();
    store
        .update_trade(
            id,
            NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-30".into()),
                is_long: true,
                shares_buy: 2,
                shares_sell: 2,
                price_buy: 10.0,
                price_sell: 12.0,
                exchange_rate_buy: 1.0,
                exchange_rate_sell: 1.0,
                commission_buy: 0.5,
                tax_buy: 0.25,
                commission_sell: 1.0,
                tax_sell: 0.25,
                cost_other: 0.4,
                risk_initial: 2.0,
                risk_percent_initial: Some(0.0),
                risk_percent_actual: None,
                trade_pool: None,
                stoploss_initial: Some(9.0),
            },
        )
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades.len(), 1);
    assert_eq!(journal.trades[0].id, id);
    assert_eq!(journal.trades[0].date_sell, "2026-09-30");
    assert_eq!(journal.trades[0].price_sell, 12.0);
    assert_eq!(journal.trades[0].cost_other, 0.4);
    assert_eq!(journal.trades[0].profit_loss, Some(4.0));
    let pool_after_close = journal.pool_value_converted;
    assert!(pool_after_close.unwrap() > 75000.0);
    assert_eq!(journal.financing.len(), 1);
    assert_eq!(journal.financing[0].trade_id, id);
    let closed_trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-30".into()),
        is_long: true,
        shares_buy: 2,
        shares_sell: 2,
        price_buy: 10.0,
        price_sell: 12.0,
        exchange_rate_buy: 1.0,
        exchange_rate_sell: 1.0,
        commission_buy: 0.5,
        tax_buy: 0.25,
        commission_sell: 1.0,
        tax_sell: 0.25,
        cost_other: 0.4,
        risk_initial: 2.0,
        risk_percent_initial: Some(0.0),
        risk_percent_actual: None,
        trade_pool: None,
        stoploss_initial: Some(9.0),
    };
    assert!(store.update_trade(id, closed_trade).is_err());
    assert_eq!(store.load().unwrap().pool_value_converted, pool_after_close);
}

#[test]
fn stores_short_trade_and_financing()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let stoploss = calculate_stoploss_initial(
        12.0, 2, 0.1, 0.2, 10.0, 2, 0.1, 0.3,
        1.5 / 75000.0 * 100.0, 75000.0, trade_type(false),
    );
    let trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-29".into()),
        is_long: false,
        shares_buy: 2,
        shares_sell: 2,
        price_buy: 12.0,
        price_sell: 10.0,
        exchange_rate_buy: 1.2,
        exchange_rate_sell: 1.1,
        commission_buy: 0.2,
        tax_buy: 0.1,
        commission_sell: 0.3,
        tax_sell: 0.1,
        cost_other: 0.2,
        risk_initial: 1.5,
        risk_percent_initial: Some(1.5 / 75000.0 * 100.0),
        risk_percent_actual: None,
        trade_pool: None,
        stoploss_initial: Some(stoploss),
    };
    store.add_trade(trade).unwrap();
    let journal = store.load().unwrap();
    assert!((journal.trades[0].profit_loss.unwrap() - 4.0).abs() < 1e-10);
    assert!((journal.trades[0].profit_loss_converted.unwrap() - 4.8).abs() < 1e-10);
    assert!((journal.trades[0].profit_loss_total.unwrap() - 3.696).abs() < 1e-10);
    assert!((journal.trades[0].profit_loss_total_converted.unwrap() - 4.4352).abs() < 1e-10);
    assert!((journal.trades[0].risk_initial - 1.54).abs() < 1e-10);
    assert!((journal.trades[0].r_multiple().unwrap() - 4.0 / 1.54).abs() < 1e-10);
    assert!((journal.pool_value_converted.unwrap()
        - journal.trades[0].trade_pool_converted.unwrap()
        - journal.trades[0].profit_loss_total_converted.unwrap()).abs() < 1e-10);

    let id = journal.trades[0].id;
    store
        .add_financing(NewFinancing {
            trade_id: id,
            date: "2026-09-29".into(),
            quantity: 2,
            price: 10.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.4,
            comment: String::new(),
        })
        .unwrap();
    let financing = store.load().unwrap().financing;
    assert_eq!(financing.len(), 1);
    assert_eq!(financing[0].trade_id, id);
    assert_eq!(financing[0].quantity, 2);
}

#[test]
fn a_losing_trade_reduces_the_pool_only_when_closed()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: None,
        is_long: true,
        shares_buy: 2,
        shares_sell: 2,
        price_buy: 12.0,
        price_sell: 10.0,
        exchange_rate_buy: 1.0,
        exchange_rate_sell: 1.0,
        commission_buy: 0.0,
        tax_buy: 0.0,
        commission_sell: 0.0,
        tax_sell: 0.0,
        cost_other: 0.0,
        risk_initial: 2.0,
        risk_percent_initial: Some(0.0),
        risk_percent_actual: None,
        trade_pool: None,
        stoploss_initial: Some(11.0),
    };
    store.add_trade(trade).unwrap();
    assert_eq!(store.load().unwrap().pool_value_converted, Some(75000.0));
    let id = store.load().unwrap().trades[0].id;
    store
        .update_trade(
            id,
            NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-29".into()),
                is_long: true,
                shares_buy: 2,
                shares_sell: 2,
                price_buy: 12.0,
                price_sell: 10.0,
                exchange_rate_buy: 1.0,
                exchange_rate_sell: 1.0,
                commission_buy: 0.0,
                tax_buy: 0.0,
                commission_sell: 0.0,
                tax_sell: 0.0,
                cost_other: 0.0,
                risk_initial: 2.0,
                risk_percent_initial: Some(0.0),
                risk_percent_actual: None,
                trade_pool: None,
                stoploss_initial: Some(11.0),
            },
        )
        .unwrap();
    assert_eq!(store.load().unwrap().trades[0].profit_loss, Some(-4.0));
    assert_eq!(store.load().unwrap().pool_value_converted, Some(74996.0));
    let count: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM t_pool", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 4);
}

#[test]
fn stores_exchange_rates_and_financing()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    store
        .add_trade(NewTrade {
            product: ".MGOLD.cfd".into(),
            date_buy: "2026-09-28".into(),
            date_sell: None,
            is_long: true,
            shares_buy: 2,
            shares_sell: 2,
            price_buy: 10.0,
            price_sell: 0.0,
            exchange_rate_buy: 1.2,
            exchange_rate_sell: 1.0,
            commission_buy: 0.1,
            tax_buy: 0.0,
            commission_sell: 0.2,
            tax_sell: 0.0,
            cost_other: 0.0,
            risk_initial: 1.2,
            risk_percent_initial: Some(0.0),
            risk_percent_actual: None,
            trade_pool: None,
            stoploss_initial: Some(9.5),
        })
        .unwrap();
    let id = store.load().unwrap().trades[0].id;
    store
        .update_trade(
            id,
            NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-29".into()),
                is_long: true,
                shares_buy: 2,
                shares_sell: 2,
                price_buy: 10.0,
                price_sell: 12.0,
                exchange_rate_buy: 1.2,
                exchange_rate_sell: 1.1,
                commission_buy: 0.1,
                tax_buy: 0.0,
                commission_sell: 0.2,
                tax_sell: 0.0,
                cost_other: 0.0,
                risk_initial: 1.2,
                risk_percent_initial: Some(0.0),
                risk_percent_actual: None,
                trade_pool: None,
                stoploss_initial: Some(9.5),
            },
        )
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(
        (
            journal.trades[0].exchange_rate_buy,
            journal.trades[0].exchange_rate_sell
        ),
        (1.2, 1.1)
    );
    assert!((journal.trades[0].profit_loss.unwrap() - 4.0).abs() < 1e-10);
    assert!((journal.trades[0].profit_loss_converted.unwrap() - 4.4).abs() < 1e-10);
    assert!((journal.trades[0].r_multiple().unwrap() - 4.0 / 1.2).abs() < 1e-10);
    assert!((journal.pool_value_converted.unwrap()
        - journal.trades[0].trade_pool_converted.unwrap()
        - journal.trades[0].profit_loss_total_converted.unwrap()).abs() < 1e-10);
    store
        .add_financing(NewFinancing {
            trade_id: id,
            date: "2026-09-29".into(),
            quantity: 2,
            price: 12.0,
            rate: 1.0,
            exchange_rate: 1.1,
            days: 1,
            value: 0.5,
            comment: String::new(),
        })
        .unwrap();
    let financing = store.load().unwrap().financing;
    assert_eq!(financing.len(), 1);
    assert_eq!(financing[0].trade_id, id);
    assert_eq!(financing[0].value, 0.5);
}
#[test]
fn date_range_accepts_open_ends_and_rejects_invalid_ranges()
{
    assert!(super::validate_date_range("", "").is_ok());
    assert!(super::validate_date_range("2026-09-28", "").is_ok());
    assert!(super::validate_date_range("", "2026-09-28").is_ok());
    assert!(super::validate_date_range("2026-09-28", "2026-09-28").is_ok());
    assert!(super::validate_date_range("2026-09-29", "2026-09-28").is_err());
    assert!(super::validate_date_range("2026-02-30", "").is_err());
    assert!(super::date_in_range(
        "2026-09-28",
        "2026-09-28",
        "2026-09-28"
    ));
    assert!(!super::date_in_range("2026-09-27", "2026-09-28", ""));
    assert!(!super::date_in_range("2026-09-29", "", "2026-09-28"));
}
