use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Product {
    pub name: String,
    pub description: String,
    pub currency: String,
    pub market: String,
}

#[derive(Clone, Debug)]
pub struct Trade {
    pub id: i64,
    pub product: String,
    pub buy_date: String,
    pub sell_date: String,
    pub is_long: bool,
    pub quantity: f64,
    pub initial_risk: f64,
    pub profit_loss: Option<f64>,
}

impl Trade {
    pub fn r_multiple(&self) -> Option<f64> {
        self.profit_loss
            .filter(|_| self.initial_risk > 0.0)
            .map(|pl| pl / self.initial_risk)
    }
}

#[derive(Clone, Debug)]
pub struct Financing {
    pub id: i64,
    pub trade_id: i64,
    pub date: String,
    pub quantity: f64,
    pub price: f64,
    pub rate: f64,
    pub days: i64,
    pub value: f64,
    pub note: String,
}

pub struct NewTrade {
    pub product: String,
    pub buy_date: String,
    pub sell_date: Option<String>,
    pub is_long: bool,
    pub quantity: f64,
    pub buy_price: f64,
    pub sell_price: f64,
    pub commission: f64,
    pub tax: f64,
    pub other_costs: f64,
    pub initial_risk: f64,
}

pub struct NewFinancing {
    pub trade_id: i64,
    pub date: String,
    pub quantity: f64,
    pub price: f64,
    pub rate: f64,
    pub exchange_rate: f64,
    pub days: i64,
    pub value: f64,
    pub note: String,
}

pub struct Journal {
    pub products: Vec<Product>,
    pub trades: Vec<Trade>,
    pub financing: Vec<Financing>,
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut connection = Connection::open(path).map_err(db_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)?;
        match version {
            0 => {
                let transaction = connection.transaction().map_err(db_error)?;
                transaction
                    .execute_batch(include_str!("../migrations/0001_initial.sql"))
                    .map_err(db_error)?;
                transaction
                    .pragma_update(None, "user_version", 1)
                    .map_err(db_error)?;
                transaction.commit().map_err(db_error)?;
            }
            1 => {}
            other => return Err(format!("Unsupported database schema version {other}")),
        }
        Ok(Self { connection })
    }

    pub fn load(&self) -> Result<Journal, String> {
        let mut products = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT p.name, p.description, c.code, m.code FROM t_product p
             JOIN t_currency c ON c.currency_id = p.currency_id
             JOIN t_market m ON m.market_id = p.market_id
             WHERE p.is_deleted = 0 ORDER BY p.name COLLATE NOCASE",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Product {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    currency: row.get(2)?,
                    market: row.get(3)?,
                })
            })
            .map_err(db_error)?;
        for row in rows {
            products.push(row.map_err(db_error)?);
        }

        let mut trades = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT t.trade_id, p.name, t.date_buy, COALESCE(t.date_sell, ''),
                    t.is_long, t.shares_buy, calc.risk_initial, calc.profit_loss
             FROM t_trade t JOIN t_product p ON p.product_id = t.product_id
             JOIN t_trade_calculated calc ON calc.trade_calculated_id = t.trade_calculated_id
             WHERE t.is_deleted = 0 ORDER BY t.trade_id",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Trade {
                    id: row.get(0)?,
                    product: row.get(1)?,
                    buy_date: row.get(2)?,
                    sell_date: row.get(3)?,
                    is_long: row.get(4)?,
                    quantity: row.get(5)?,
                    initial_risk: row.get(6)?,
                    profit_loss: row.get(7)?,
                })
            })
            .map_err(db_error)?;
        for row in rows {
            trades.push(row.map_err(db_error)?);
        }

        let mut financing = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT f.financing_id, f.trade_id, f.date, f.quantity, f.price,
                    f.rate, f.days, f.value, f.note
             FROM t_financing f JOIN t_trade t ON t.trade_id = f.trade_id
             WHERE f.is_deleted = 0 AND t.is_deleted = 0 ORDER BY f.financing_id",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Financing {
                    id: row.get(0)?,
                    trade_id: row.get(1)?,
                    date: row.get(2)?,
                    quantity: row.get(3)?,
                    price: row.get(4)?,
                    rate: row.get(5)?,
                    days: row.get(6)?,
                    value: row.get(7)?,
                    note: row.get(8)?,
                })
            })
            .map_err(db_error)?;
        for row in rows {
            financing.push(row.map_err(db_error)?);
        }
        Ok(Journal {
            products,
            trades,
            financing,
        })
    }

    pub fn add_product(
        &mut self,
        name: &str,
        description: &str,
        currency: &str,
        market: &str,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Product name is required".into());
        }
        let currency_id: i64 = self
            .connection
            .query_row(
                "SELECT currency_id FROM t_currency WHERE code = ?1 AND is_deleted = 0",
                [currency],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?
            .ok_or("Unknown currency")?;
        let market_id: i64 = self
            .connection
            .query_row(
                "SELECT market_id FROM t_market WHERE code = ?1 AND is_deleted = 0",
                [market],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?
            .ok_or("Unknown market")?;
        self.connection
            .execute(
                "INSERT INTO t_product(name, description, product_type_id, currency_id, market_id)
             VALUES (?1, ?2, 1, ?3, ?4)",
                params![name, description.trim(), currency_id, market_id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn add_trade(&mut self, trade: NewTrade) -> Result<(), String> {
        let product_id: i64 = self
            .connection
            .query_row(
                "SELECT product_id FROM t_product WHERE name = ?1 AND is_deleted = 0",
                [&trade.product],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?
            .ok_or("Select a product by name")?;
        let profit_loss = trade.sell_date.as_ref().map(|_| {
            (if trade.is_long {
                trade.sell_price - trade.buy_price
            } else {
                trade.buy_price - trade.sell_price
            }) * trade.quantity
        });
        if profit_loss.is_some_and(|value| !value.is_finite()) {
            return Err("Calculated P/L is too large".into());
        }
        let r_multiple = profit_loss
            .filter(|_| trade.initial_risk > 0.0)
            .map(|pl| pl / trade.initial_risk);
        if r_multiple.is_some_and(|value| !value.is_finite()) {
            return Err("Calculated R is too large".into());
        }
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction
            .execute(
                "INSERT INTO t_trade_cost(commission, tax, other) VALUES (?1, ?2, ?3)",
                params![trade.commission, trade.tax, trade.other_costs],
            )
            .map_err(db_error)?;
        let cost_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO t_trade_calculated(risk_initial, profit_loss, profit_loss_total, r_multiple)
             VALUES (?1, ?2, ?3, ?4)",
            params![trade.initial_risk, profit_loss,
                    profit_loss.map(|pl| pl - trade.commission - trade.tax - trade.other_costs),
                    r_multiple]
        ).map_err(db_error)?;
        let calculated_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO t_trade(trade_calculated_id, product_id, trade_cost_id, date_buy, date_sell,
                                 is_long, shares_buy, shares_sell, price_buy, price_sell)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![calculated_id, product_id, cost_id, trade.buy_date, trade.sell_date,
                    trade.is_long, trade.quantity, if profit_loss.is_some() { trade.quantity } else { 0.0 },
                    trade.buy_price, trade.sell_price]
        ).map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn add_financing(&mut self, entry: NewFinancing) -> Result<(), String> {
        let calculated_id: i64 = self
            .connection
            .query_row(
                "SELECT trade_calculated_id FROM t_trade WHERE trade_id = ?1 AND is_deleted = 0",
                [entry.trade_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?
            .ok_or_else(|| format!("Trade #{} does not exist", entry.trade_id))?;
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction.execute(
            "INSERT INTO t_financing(trade_id, date, quantity, price, rate, exchange_rate, days, value, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![entry.trade_id, entry.date, entry.quantity, entry.price, entry.rate,
                    entry.exchange_rate, entry.days, entry.value, entry.note]
        ).map_err(db_error)?;
        transaction.execute(
            "UPDATE t_trade_calculated SET
                profit_loss_total = profit_loss - (
                    SELECT commission + tax + other FROM t_trade_cost
                    WHERE trade_cost_id = (SELECT trade_cost_id FROM t_trade WHERE trade_calculated_id = ?1)
                ) - (
                    SELECT COALESCE(SUM(value), 0) FROM t_financing
                    WHERE trade_id = ?2 AND is_deleted = 0
                ),
                date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE trade_calculated_id = ?1",
            params![calculated_id, entry.trade_id]
        ).map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(())
    }
}

fn db_error(error: rusqlite::Error) -> String {
    match error {
        rusqlite::Error::SqliteFailure(_, Some(message)) => format!("Database: {message}"),
        error => format!("Database: {error}"),
    }
}

pub fn database_path() -> PathBuf {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("capitalcommander.sqlite3"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                commission: 0.5,
                tax: 0.25,
                other_costs: 0.25,
                initial_risk: 2.0,
            })
            .unwrap();
        let journal = store.load().unwrap();
        assert_eq!(journal.trades[0].r_multiple(), Some(2.0));
        let saved_costs: (f64, f64, f64) = store
            .connection
            .query_row(
                "SELECT commission, tax, other FROM t_trade_cost",
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
        assert!((net - 2.9).abs() < 1e-10);
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
}
