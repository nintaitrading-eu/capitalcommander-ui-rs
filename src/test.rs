use crate::data::{calculate_trade_stoploss, NewFinancing, NewTrade, Store, Trade};
use crate::check_date;
use std::path::Path;

#[test]
fn valid_r_multiple_requires_positive_risk()
{
    let mut trade = Trade {
        id: 1,
        product: "A".into(),
        date_buy: "2026-01-01".into(),
        date_sell: "2026-01-02".into(),
        is_long: true,
        quantity: 1.0,
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
        risk_actual: Some(10.0),
        risk_percent: None,
        trade_pool: None,
        profit_loss: Some(-5.0),
        profit_loss_total: None,
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
    assert_eq!(calculate_trade_stoploss(1.0, 400.0, 12.0, 2.0, 1.0, 2.0, true).unwrap(), 11.0);
    assert_eq!(calculate_trade_stoploss(1.0, 400.0, 12.0, 2.0, 1.0, 2.0, false).unwrap(), 13.0);
    assert!(calculate_trade_stoploss(1.0, 75000.0, 12.0, 2.0, 1.0, 2.0, true).is_err());
}

#[test]
fn calculates_and_saves_trade_risks_when_closed()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let open = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: None,
        is_long: true,
        quantity: 2.0,
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
        risk_percent: Some(1.0),
        trade_pool: Some(400.0),
    };
    store.add_trade(open).unwrap();
    let trade = &store.load().unwrap().trades[0];
    assert_eq!(trade.risk_initial, 4.0);
    assert_eq!(trade.risk_actual, None);
    assert_eq!(
        (trade.risk_percent, trade.trade_pool),
        (Some(1.0), Some(400.0))
    );
    let linked_pool: (i64, f64) = store.connection.query_row(
        "SELECT t.pool_id, p.pool_value FROM t_trade t JOIN t_pool p ON p.pool_id = t.pool_id",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).unwrap();
    assert_eq!(linked_pool.1, 400.0);
    store.connection.execute("INSERT INTO t_pool(pool_value) VALUES (500)", []).unwrap();
    let id = trade.id;
    store
        .update_trade(
            id,
            NewTrade {
                product: ".MGOLD.cfd".into(),
                date_buy: "2026-09-28".into(),
                date_sell: Some("2026-09-29".into()),
                is_long: true,
                quantity: 2.0,
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
                risk_percent: Some(1.0),
                trade_pool: Some(500.0),
            },
        )
        .unwrap();
    let trade = &store.load().unwrap().trades[0];
    assert_eq!(trade.risk_initial, 4.0);
    assert_eq!(trade.risk_actual, Some(10.0));
    assert_eq!(trade.trade_pool, Some(linked_pool.1));
    let pool_id: i64 = store.connection.query_row("SELECT pool_id FROM t_trade", [], |row| row.get(0)).unwrap();
    assert_eq!(pool_id, linked_pool.0);
    let saved: (f64, Option<f64>) = store
        .connection
        .query_row(
            "SELECT risk_initial, risk_actual FROM t_trade_calculated",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(saved, (4.0, Some(10.0)));
    let calculated_columns: i64 = store.connection.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('t_trade_calculated') WHERE name IN ('risk_initial', 'risk_actual', 'risk_percent', 'trade_pool', 'stoploss')",
        [], |row| row.get(0),
    ).unwrap();
    assert_eq!(calculated_columns, 4);
}

#[test]
fn loads_latest_pool_value()
{
    let store = Store::open(Path::new(":memory:")).unwrap();
    assert_eq!(store.load().unwrap().pool_value, Some(75000.0));

    store
        .connection
        .execute("INSERT INTO t_pool(pool_value) VALUES (?1)", [82500.5])
        .unwrap();
    assert_eq!(store.load().unwrap().pool_value, Some(82500.5));

    store.connection.execute("DELETE FROM t_pool", []).unwrap();
    assert_eq!(store.load().unwrap().pool_value, None);
}

#[test]
fn adds_market_and_uses_it_for_products()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    assert!(store.add_market(" ", "Exchange", "BE").is_err());
    assert!(store.add_market("XBRU", " ", "BE").is_err());

    store.add_market(" XBRU ", " Euronext Brussels ", " BE ").unwrap();
    let journal = store.load().unwrap();
    let market = journal.markets.iter().find(|market| market.code == "XBRU").unwrap();
    assert_eq!(market.name, "Euronext Brussels");
    assert_eq!(market.country, "BE");

    store.add_product("BRU.cfd", "Brussels product", "EUR", "XBRU").unwrap();
    assert_eq!(
        store.load().unwrap().products.iter().find(|product| product.name == "BRU.cfd").unwrap().market,
        "XBRU"
    );
    assert!(store.add_market("XBRU", "Duplicate", "BE").is_err());
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
            quantity: 2.0,
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
            risk_percent: None,
            trade_pool: None,
        })
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades[0].r_multiple(), Some(2.0));
    assert!((journal.pool_value.unwrap() - 75001.15).abs() < 1e-10);
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
            quantity: 2.0,
            price: 12.0,
            rate: 2.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.1,
            note: "Test".into(),
        })
        .unwrap();
    assert_eq!(store.load().unwrap().financing.len(), 1);
    let net: f64 = store
        .connection
        .query_row(
            "SELECT profit_loss_total FROM t_trade_calculated",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!((net - 1.05).abs() < 1e-10);
    assert!(store
        .add_financing(NewFinancing {
            trade_id: 999,
            date: "2026-09-29".into(),
            quantity: 1.0,
            price: 1.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 1.0,
            note: String::new(),
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
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    };
    store.add_trade(open_trade).unwrap();
    let id = store.load().unwrap().trades[0].id;
    assert_eq!(store.load().unwrap().trades[0].price_sell, 11.0);
    assert_eq!(store.load().unwrap().trades[0].profit_loss, None);
    store
        .add_financing(NewFinancing {
            trade_id: id,
            date: "2026-09-29".into(),
            quantity: 2.0,
            price: 10.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.1,
            note: String::new(),
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
                quantity: 2.0,
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
                risk_percent: None,
                trade_pool: None,
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
    assert_eq!(journal.pool_value, Some(75001.5));
    assert_eq!(journal.financing.len(), 1);
    let net: f64 = store
        .connection
        .query_row(
            "SELECT profit_loss_total FROM t_trade_calculated",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!((net - 1.5).abs() < 1e-10);
    let closed_trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-30".into()),
        is_long: true,
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    };
    assert!(store.update_trade(id, closed_trade).is_err());
    assert_eq!(store.load().unwrap().pool_value, Some(75001.5));
}

#[test]
fn calculates_fractional_short_trade_with_absolute_costs()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-29".into()),
        is_long: false,
        quantity: 1.5,
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
        risk_percent: None,
        trade_pool: None,
    };
    store.add_trade(trade).unwrap();
    let journal = store.load().unwrap();
    assert!((journal.trades[0].profit_loss.unwrap() - 5.1).abs() < 1e-10);
    assert!((journal.trades[0].r_multiple().unwrap() - 3.4).abs() < 1e-10);
    assert!((journal.pool_value.unwrap() - 75004.2).abs() < 1e-10);
    let net: f64 = store
        .connection
        .query_row(
            "SELECT profit_loss_total FROM t_trade_calculated",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!((net - 4.2).abs() < 1e-10);

    let id = journal.trades[0].id;
    store
        .add_financing(NewFinancing {
            trade_id: id,
            date: "2026-09-29".into(),
            quantity: 1.5,
            price: 10.0,
            rate: 1.0,
            exchange_rate: 1.0,
            days: 1,
            value: 0.4,
            note: String::new(),
        })
        .unwrap();
    let net: f64 = store
        .connection
        .query_row(
            "SELECT profit_loss_total FROM t_trade_calculated",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!((net - 3.8).abs() < 1e-10);
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
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    };
    store.add_trade(trade).unwrap();
    assert_eq!(store.load().unwrap().pool_value, Some(75000.0));
    let id = store.load().unwrap().trades[0].id;
    store.update_trade(id, NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-29".into()),
        is_long: true,
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    }).unwrap();
    assert_eq!(store.load().unwrap().trades[0].profit_loss, Some(-4.0));
    assert_eq!(store.load().unwrap().pool_value, Some(74996.0));
    let count: i64 = store.connection.query_row("SELECT COUNT(*) FROM t_pool", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn converts_trade_prices_to_eur_for_gross_net_pool_and_financing()
{
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    store.add_trade(NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: None,
        is_long: true,
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    }).unwrap();
    let id = store.load().unwrap().trades[0].id;
    store.update_trade(id, NewTrade {
        product: ".MGOLD.cfd".into(),
        date_buy: "2026-09-28".into(),
        date_sell: Some("2026-09-29".into()),
        is_long: true,
        quantity: 2.0,
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
        risk_percent: None,
        trade_pool: None,
    }).unwrap();
    let journal = store.load().unwrap();
    assert_eq!((journal.trades[0].exchange_rate_buy, journal.trades[0].exchange_rate_sell), (1.2, 1.1));
    assert!((journal.trades[0].profit_loss.unwrap() - 2.4).abs() < 1e-10);
    assert!((journal.trades[0].r_multiple().unwrap() - 2.0).abs() < 1e-10);
    assert!((journal.pool_value.unwrap() - 75002.1).abs() < 1e-10);
    store.add_financing(NewFinancing {
        trade_id: id,
        date: "2026-09-29".into(),
        quantity: 2.0,
        price: 12.0,
        rate: 1.0,
        exchange_rate: 1.1,
        days: 1,
        value: 0.5,
        note: String::new(),
    }).unwrap();
    let net: f64 = store.connection.query_row(
        "SELECT profit_loss_total FROM t_trade_calculated", [], |row| row.get(0)
    ).unwrap();
    assert!((net - 1.6).abs() < 1e-10);
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
    assert!(super::date_in_range("2026-09-28", "2026-09-28", "2026-09-28"));
    assert!(!super::date_in_range("2026-09-27", "2026-09-28", ""));
    assert!(!super::date_in_range("2026-09-29", "", "2026-09-28"));
}
