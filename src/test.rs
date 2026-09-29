use crate::data::{NewFinancing, NewTrade, Store, Trade};
use crate::check_date;
use std::path::Path;

#[test]
fn valid_r_multiple_requires_positive_risk() {
    let mut trade = Trade {
        id: 1,
        product: "A".into(),
        buy_date: "2026-01-01".into(),
        sell_date: "2026-01-02".into(),
        is_long: true,
        quantity: 1.0,
        buy_price: 1.0,
        sell_price: 2.0,
        commission_buy: 0.0,
        tax_buy: 0.0,
        commission_sell: 0.0,
        tax_sell: 0.0,
        other_costs: 0.0,
        initial_risk: 10.0,
        profit_loss: Some(-5.0),
    };
    assert_eq!(trade.r_multiple(), Some(-0.5));
    trade.initial_risk = 0.0;
    assert_eq!(trade.r_multiple(), None);
}

#[test]
fn rejects_invalid_calendar_date() {
    assert!(check_date("2026-02-29", "Date", false).is_err());
    assert!(check_date("2024-02-29", "Date", false).is_ok());
}

#[test]
fn loads_latest_pool_value() {
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
fn persists_relations_and_rejects_unknown_trade() {
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
            buy_date: "2026-09-28".into(),
            sell_date: Some("2026-09-29".into()),
            is_long: true,
            quantity: 2.0,
            buy_price: 10.0,
            sell_price: 12.0,
            commission_buy: 0.5,
            tax_buy: 0.25,
            commission_sell: 1.5,
            tax_sell: 0.35,
            other_costs: 0.25,
            initial_risk: 2.0,
        })
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades[0].r_multiple(), Some(2.0));
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
fn closes_an_existing_trade_without_changing_its_id_or_financing() {
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let open_trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        buy_date: "2026-09-28".into(),
        sell_date: None,
        is_long: true,
        quantity: 2.0,
        buy_price: 10.0,
        sell_price: 11.0,
        commission_buy: 0.5,
        tax_buy: 0.25,
        commission_sell: 0.0,
        tax_sell: 0.0,
        other_costs: 0.0,
        initial_risk: 2.0,
    };
    store.add_trade(open_trade).unwrap();
    let id = store.load().unwrap().trades[0].id;
    assert_eq!(store.load().unwrap().trades[0].sell_price, 11.0);
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
                buy_date: "2026-09-28".into(),
                sell_date: Some("2026-09-30".into()),
                is_long: true,
                quantity: 2.0,
                buy_price: 10.0,
                sell_price: 12.0,
                commission_buy: 0.5,
                tax_buy: 0.25,
                commission_sell: 1.0,
                tax_sell: 0.25,
                other_costs: 0.4,
                initial_risk: 2.0,
            },
        )
        .unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades.len(), 1);
    assert_eq!(journal.trades[0].id, id);
    assert_eq!(journal.trades[0].sell_date, "2026-09-30");
    assert_eq!(journal.trades[0].sell_price, 12.0);
    assert_eq!(journal.trades[0].other_costs, 0.4);
    assert_eq!(journal.trades[0].profit_loss, Some(4.0));
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
}

#[test]
fn calculates_fractional_short_trade_with_absolute_costs() {
    let mut store = Store::open(Path::new(":memory:")).unwrap();
    let trade = NewTrade {
        product: ".MGOLD.cfd".into(),
        buy_date: "2026-09-28".into(),
        sell_date: Some("2026-09-29".into()),
        is_long: false,
        quantity: 1.5,
        buy_price: 12.0,
        sell_price: 10.0,
        commission_buy: 0.2,
        tax_buy: 0.1,
        commission_sell: 0.3,
        tax_sell: 0.1,
        other_costs: 0.2,
        initial_risk: 1.5,
    };
    store.add_trade(trade).unwrap();
    let journal = store.load().unwrap();
    assert_eq!(journal.trades[0].profit_loss, Some(3.0));
    assert_eq!(journal.trades[0].r_multiple(), Some(2.0));
    let net: f64 = store
        .connection
        .query_row(
            "SELECT profit_loss_total FROM t_trade_calculated",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!((net - 2.1).abs() < 1e-10);

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
    assert!((net - 1.7).abs() < 1e-10);
}

#[test]
fn reopens_a_file_database_without_reseeding() {
    let file = std::env::temp_dir().join(format!(
        "capitalcommander-test-{}-{}.sqlite3",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let mut store = Store::open(&file).unwrap();
        store
            .add_product("PERSIST.cfd", "Persistent", "EUR", "cfd other non-share")
            .unwrap();
    }
    let reopened = Store::open(&file).unwrap();
    let products = reopened.load().unwrap().products;
    assert_eq!(
        products.iter().filter(|p| p.name == "PERSIST.cfd").count(),
        1
    );
    drop(reopened);
    std::fs::remove_file(file).unwrap();
}
