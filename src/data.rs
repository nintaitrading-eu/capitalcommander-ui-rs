use crate::calculatorfinance_lib::trade_calculations;
use libcalculatorfinance::convert_to_orig;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Product
{
    pub name: String,
    pub description: String,
    pub currency: String,
    pub market: String,
}

#[derive(Clone, Debug)]
pub struct Market
{
    pub code: String,
    pub name: String,
    pub country: String,
}

#[derive(Clone, Debug)]
pub struct Parameter
{
    pub id: i64,
    pub name: String,
    pub value: String,
    pub value_default: String,
    pub description: String,
    pub datatype: String,
}

#[derive(Clone, Debug)]
pub struct Trade
{
    pub id: i64,
    pub product: String,
    pub currency: String,
    pub date_buy: String,
    pub date_sell: String,
    pub is_long: bool,
    pub shares_buy: i32,
    pub shares_sell: i32,
    pub price_buy: f64,
    pub price_sell: f64,
    pub exchange_rate_buy: f64,
    pub exchange_rate_sell: f64,
    pub commission_buy: f64,
    pub tax_buy: f64,
    pub commission_sell: f64,
    pub tax_sell: f64,
    pub cost_other: f64,
    pub risk_initial: f64,
    pub risk_initial_converted: f64,
    pub risk_actual: Option<f64>,
    pub risk_actual_converted: Option<f64>,
    pub risk_percent_initial: Option<f64>,
    pub risk_percent_actual: Option<f64>,
    pub trade_pool: Option<f64>,
    pub trade_pool_converted: Option<f64>,
    pub stoploss_initial: Option<f64>,
    pub stoploss_actual: Option<f64>,
    pub target_minimum: f64,
    pub target_preferred: Option<f64>,
    pub profit_loss: Option<f64>,
    pub profit_loss_converted: Option<f64>,
    pub profit_loss_total: Option<f64>,
    pub profit_loss_total_converted: Option<f64>,
}

#[derive(Clone)]
pub struct NewTrade
{
    pub product: String,
    pub date_buy: String,
    pub date_sell: Option<String>,
    pub is_long: bool,
    pub shares_buy: i32,
    pub shares_sell: i32,
    pub price_buy: f64,
    pub price_sell: f64,
    pub exchange_rate_buy: f64,
    pub exchange_rate_sell: f64,
    pub commission_buy: f64,
    pub tax_buy: f64,
    pub commission_sell: f64,
    pub tax_sell: f64,
    pub cost_other: f64,
    pub risk_initial: f64,
    pub risk_percent_initial: Option<f64>,
    pub risk_percent_actual: Option<f64>,
    pub trade_pool: Option<f64>,
    pub stoploss_initial: Option<f64>,
}

pub struct Journal
{
    pub products: Vec<Product>,
    pub markets: Vec<Market>,
    pub parameters: Vec<Parameter>,
    pub trades: Vec<Trade>,
    pub pool_value_converted: Option<f64>,
}

pub struct Store
{
    pub(crate) connection: Connection,
}

impl Store
{
    pub fn open(path: &Path) -> Result<Self, String>
    {
        let mut connection = Connection::open(path).map_err(db_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(db_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(db_error)?;
        match version
        {
            0 =>
            {
                let transaction = connection.transaction().map_err(db_error)?;
                transaction
                    .execute_batch(include_str!("../migrations/0001_initial.sql"))
                    .map_err(db_error)?;
                transaction
                    .pragma_update(None, "user_version", 1)
                    .map_err(db_error)?;
                transaction.commit().map_err(db_error)?;
            },
            1 => {},
            other => return Err(format!("Unsupported database schema version {other}")),
        }
        Ok(Self { connection })
    }

    pub fn load(&self) -> Result<Journal, String>
    {
        let pool_value_converted = self
            .connection
            .query_row(
                "SELECT pool_value_converted FROM t_pool ORDER BY pool_id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
        let mut markets = Vec::new();
        let mut statement = self
            .connection
            .prepare("SELECT code, name, country FROM t_market WHERE is_deleted = 0 ORDER BY code COLLATE NOCASE")
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Market {
                    code: row.get(0)?,
                    name: row.get(1)?,
                    country: row.get(2)?,
                })
            })
            .map_err(db_error)?;
        for row in rows
        {
            markets.push(row.map_err(db_error)?);
        }
        let mut parameters = Vec::new();
        let mut statement = self
            .connection
            .prepare("SELECT parameter_id, name, value, value_default, description, datatype FROM t_parameter WHERE is_deleted = 0 ORDER BY name COLLATE NOCASE")
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Parameter {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    value: row.get(2)?,
                    value_default: row.get(3)?,
                    description: row.get(4)?,
                    datatype: row.get(5)?,
                })
            })
            .map_err(db_error)?;
        for row in rows
        {
            parameters.push(row.map_err(db_error)?);
        }
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
        for row in rows
        {
            products.push(row.map_err(db_error)?);
        }

        let mut trades = Vec::new();
        let mut statement = self
            .connection
            .prepare(
                "SELECT t.trade_id, p.name, t.date_buy, COALESCE(t.date_sell, ''),
                    t.is_long, t.shares_buy, t.shares_sell, t.price_buy, t.price_sell,
                    cost.commission_buy, cost.tax_buy, cost.commission_sell,
                    cost.tax_sell, cost.other, calc.risk_initial, calc.risk_initial_converted, calc.profit_loss, calc.profit_loss_converted,
                    calc.profit_loss_total, calc.profit_loss_total_converted,
                    t.exchange_rate_buy, t.exchange_rate_sell, calc.risk_actual, calc.risk_actual_converted,
                    calc.risk_percent_initial, calc.risk_percent_actual, pool.pool_value, pool.pool_value_converted, calc.stoploss_initial, calc.stoploss_actual,
                    c.code, calc.target_minimum, calc.target_preferred
             FROM t_trade t JOIN t_product p ON p.product_id = t.product_id
             JOIN t_currency c ON c.currency_id = p.currency_id
             JOIN t_trade_cost cost ON cost.trade_cost_id = t.trade_cost_id
             JOIN t_trade_calculated calc ON calc.trade_calculated_id = t.trade_calculated_id
             JOIN t_pool pool ON pool.pool_id = t.pool_id
             WHERE t.is_deleted = 0 ORDER BY t.trade_id",
            )
            .map_err(db_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Trade {
                    id: row.get(0)?,
                    product: row.get(1)?,
                    currency: row.get(30)?,
                    date_buy: row.get(2)?,
                    date_sell: row.get(3)?,
                    is_long: row.get(4)?,
                    shares_buy: row.get(5)?,
                    shares_sell: row.get(6)?,
                    price_buy: row.get(7)?,
                    price_sell: row.get(8)?,
                    commission_buy: row.get(9)?,
                    tax_buy: row.get(10)?,
                    commission_sell: row.get(11)?,
                    tax_sell: row.get(12)?,
                    cost_other: row.get(13)?,
                    risk_initial: row.get(14)?,
                    risk_initial_converted: row.get(15)?,
                    profit_loss: row.get(16)?,
                    profit_loss_converted: row.get(17)?,
                    profit_loss_total: row.get(18)?,
                    profit_loss_total_converted: row.get(19)?,
                    exchange_rate_buy: row.get(20)?,
                    exchange_rate_sell: row.get(21)?,
                    risk_actual: row.get(22)?,
                    risk_actual_converted: row.get(23)?,
                    risk_percent_initial: row.get(24)?,
                    risk_percent_actual: row.get(25)?,
                    trade_pool: row.get(26)?,
                    trade_pool_converted: row.get(27)?,
                    stoploss_initial: row.get(28)?,
                    stoploss_actual: row.get(29)?,
                    target_minimum: row.get(31)?,
                    target_preferred: row.get(32)?,
                })
            })
            .map_err(db_error)?;
        for row in rows
        {
            trades.push(row.map_err(db_error)?);
        }

        Ok(Journal {
            products,
            markets,
            parameters,
            trades,
            pool_value_converted,
        })
    }

    pub fn save_parameter(&mut self, id: i64, name: &str, value: &str, value_default: &str, description: &str, datatype: &str) -> Result<(), String>
    {
        let name = name.trim();
        let datatype = datatype.trim();
        if name.is_empty()
        {
            return Err("Parameter name is required".into());
        }
        if datatype.is_empty()
        {
            return Err("Datatype is required".into());
        }
        for (label, value, limit) in [
            ("Name", name, 50),
            ("Value", value, 512),
            ("Default value", value_default, 512),
            ("Description", description, 4000),
            ("Datatype", datatype, 50),
        ]
        {
            if value.chars().count() > limit
            {
                return Err(format!("{label} must be at most {limit} characters"));
            }
        }
        if id == 0
        {
            self.connection
                .execute(
                    "INSERT INTO t_parameter(name, value, value_default, description, datatype) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, value, value_default, description, datatype],
                )
                .map_err(db_error)?;
        }
        else
        {
            let changed = self.connection
                .execute(
                    "UPDATE t_parameter SET name = ?1, value = ?2, value_default = ?3, description = ?4, datatype = ?5, date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE parameter_id = ?6 AND is_deleted = 0",
                    params![name, value, value_default, description, datatype, id],
                )
                .map_err(db_error)?;
            if changed == 0
            {
                return Err(format!("Parameter #{id} does not exist"));
            }
        }
        Ok(())
    }

    pub fn add_market(&mut self, code: &str, name: &str, country: &str) -> Result<(), String>
    {
        let code = code.trim();
        let name = name.trim();
        if code.is_empty()
        {
            return Err("Market code is required".into());
        }
        if name.is_empty()
        {
            return Err("Market name is required".into());
        }
        self.connection
            .execute(
                "INSERT INTO t_market(code, name, country) VALUES (?1, ?2, ?3)",
                params![code, name, country.trim()],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn add_product(
        &mut self,
        name: &str,
        description: &str,
        currency: &str,
        market: &str,
    ) -> Result<(), String>
    {
        let name = name.trim();
        if name.is_empty()
        {
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
                "INSERT INTO t_product(name, description, product_type_id, currency_id, market_id, tick, tick_value)
             VALUES (?1, ?2, 1, ?3, ?4, 1, 1)",
                params![name, description.trim(), currency_id, market_id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn add_trade(&mut self, trade: NewTrade) -> Result<(), String>
    {
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
        let transaction = self.connection.transaction().map_err(db_error)?;
        let pool_value = match trade.trade_pool
        {
            Some(value) => value,
            None => transaction
                .query_row(
                    "SELECT pool_value FROM t_pool ORDER BY pool_id DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()
                .map_err(db_error)?
                .ok_or("No pool balance exists")?,
        };
        if !pool_value.is_finite() || pool_value < 0.0
        {
            return Err("Pool balance must be finite and nonnegative".into());
        }
        transaction
            .execute("INSERT INTO t_pool(pool_value, pool_value_converted) VALUES (?1, ?2)", [pool_value, convert_to_orig(pool_value, trade.exchange_rate_buy)])
            .map_err(db_error)?;
        let pool_id = transaction.last_insert_rowid();
        transaction
            .execute(
                "INSERT INTO t_trade_cost(commission_buy, tax_buy, commission_sell, tax_sell, other)
                      VALUES (?1, ?2, ?3, ?4, ?5)",
                params![trade.commission_buy, trade.tax_buy, trade.commission_sell, trade.tax_sell, trade.cost_other],
            )
            .map_err(db_error)?;
        let cost_id = transaction.last_insert_rowid();
        let (stoploss_initial, stoploss_actual, target_minimum, target_preferred, risk_initial, risk_initial_converted, risk_actual, risk_actual_converted, profit_loss, profit_loss_converted, profit_loss_total, profit_loss_total_converted, r_multiple) = trade_calculations(&trade, pool_value)?;
        transaction
            .execute(
                "INSERT INTO t_trade_calculated(risk_initial, risk_initial_converted, risk_actual, risk_actual_converted, risk_percent_initial, risk_percent_actual, stoploss_initial, stoploss_actual, target_minimum, target_preferred, profit_loss, profit_loss_converted, profit_loss_total, profit_loss_total_converted, r_multiple)
                      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    risk_initial,
                    risk_initial_converted,
                    risk_actual,
                    risk_actual_converted,
                    trade.risk_percent_initial,
                    trade.risk_percent_actual,
                    stoploss_initial,
                    stoploss_actual,
                    target_minimum,
                    target_preferred,
                    profit_loss,
                    profit_loss_converted,
                    profit_loss_total,
                    profit_loss_total_converted,
                    r_multiple
                ],
            )
            .map_err(db_error)?;
        let calculated_id = transaction.last_insert_rowid();
        transaction
            .execute("INSERT INTO t_trade_review DEFAULT VALUES", [])
            .map_err(db_error)?;
        let review_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO t_trade(trade_calculated_id, product_id, trade_cost_id, trade_review_id, pool_id, date_buy, date_sell,
                                 is_long, shares_buy, shares_sell, price_buy, price_sell, exchange_rate_buy, exchange_rate_sell)
                  VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![calculated_id, product_id, cost_id, review_id, pool_id, trade.date_buy, trade.date_sell,
                    trade.is_long, trade.shares_buy, if profit_loss.is_some() { trade.shares_sell } else { 0 },
                    trade.price_buy, trade.price_sell, trade.exchange_rate_buy, trade.exchange_rate_sell]
        ).map_err(db_error)?;
        if let Some(profit_loss_total_converted) = profit_loss_total_converted
        {
            record_pool_profit_loss_total_converted(&transaction, profit_loss_total_converted)?;
        }
        transaction.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn update_trade(&mut self, id: i64, trade: NewTrade) -> Result<(), String>
    {
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
        let (cost_id, calculated_id, pool_value): (i64, i64, f64) = self
            .connection
            .query_row(
                "SELECT t.trade_cost_id, t.trade_calculated_id, pool.pool_value FROM t_trade t
             JOIN t_pool pool ON pool.pool_id = t.pool_id
             WHERE t.trade_id = ?1 AND t.is_deleted = 0 AND t.date_sell IS NULL",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(db_error)?
            .ok_or_else(|| format!("Trade #{id} does not exist or is already closed"))?;
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction
            .execute(
                "UPDATE t_trade_cost SET commission_buy = ?1, tax_buy = ?2, commission_sell = ?3,
             tax_sell = ?4, other = ?5, date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE trade_cost_id = ?6",
                params![
                    trade.commission_buy,
                    trade.tax_buy,
                    trade.commission_sell,
                    trade.tax_sell,
                    trade.cost_other,
                    cost_id
                ],
            )
            .map_err(db_error)?;
        let (stoploss_initial, stoploss_actual, target_minimum, target_preferred, risk_initial, risk_initial_converted, risk_actual, risk_actual_converted, profit_loss, profit_loss_converted, profit_loss_total, profit_loss_total_converted, r_multiple) = trade_calculations(&trade, pool_value)?;
        transaction
            .execute(
                "UPDATE t_trade SET product_id = ?1, date_buy = ?2, date_sell = ?3, is_long = ?4,
             shares_buy = ?5, shares_sell = ?6, price_buy = ?7, price_sell = ?8,
             exchange_rate_buy = ?9, exchange_rate_sell = ?10,
             date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE trade_id = ?11",
                params![
                    product_id,
                    trade.date_buy,
                    trade.date_sell,
                    trade.is_long,
                    trade.shares_buy,
                    trade.shares_sell,
                    trade.price_buy,
                    trade.price_sell,
                    trade.exchange_rate_buy,
                    trade.exchange_rate_sell,
                    id
                ],
            )
            .map_err(db_error)?;
        transaction
            .execute(
                "UPDATE t_trade_calculated
                    SET risk_initial = ?1,
                        risk_initial_converted = ?2,
                        risk_actual = ?3,
                        risk_actual_converted = ?4,
                        risk_percent_initial = ?5,
                        risk_percent_actual = ?6,
                        stoploss_initial = ?7,
                        stoploss_actual = ?8,
                        target_minimum = ?9,
                        target_preferred = ?10,
                        profit_loss = ?11,
                        profit_loss_converted = ?12,
                        profit_loss_total = ?13,
                        profit_loss_total_converted = ?14,
                        r_multiple = ?15,
             date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE trade_calculated_id = ?16",
                params![
                    risk_initial,
                    risk_initial_converted,
                    risk_actual,
                    risk_actual_converted,
                    trade.risk_percent_initial,
                    trade.risk_percent_actual,
                    stoploss_initial,
                    stoploss_actual,
                    target_minimum,
                    target_preferred,
                    profit_loss,
                    profit_loss_converted,
                    profit_loss_total,
                    profit_loss_total_converted,
                    r_multiple,
                    calculated_id
                ],
            )
            .map_err(db_error)?;
        if let Some(profit_loss_total) = profit_loss_total
        {
            record_pool_profit_loss_total(&transaction, profit_loss_total)?;
        }
        if let Some(profit_loss_total_converted) = profit_loss_total_converted
        {
            record_pool_profit_loss_total_converted(&transaction, profit_loss_total_converted)?;
        }
        transaction.commit().map_err(db_error)?;
        Ok(())
    }
}

fn record_pool_profit_loss_total(transaction: &Transaction<'_>, profit_loss_total: f64) -> Result<(), String>
{
    let current: f64 = transaction
        .query_row(
            "SELECT pool_value FROM t_pool ORDER BY pool_id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?
        .ok_or("No pool balance exists")?;
    let updated = current + profit_loss_total;
    if !updated.is_finite() || updated < 0.0
    {
        return Err("Closing this trade would make the pool balance invalid".into());
    }
    transaction
        .execute(
            "INSERT INTO t_pool(pool_value, pool_value_converted)
             SELECT ?1, pool_value_converted FROM t_pool ORDER BY pool_id DESC LIMIT 1",
            [updated],
        )
        .map_err(db_error)?;
    Ok(())
}

fn record_pool_profit_loss_total_converted(transaction: &Transaction<'_>, profit_loss_total_converted: f64) -> Result<(), String>
{
    let current: f64 = transaction
        .query_row(
            "SELECT pool_value_converted FROM t_pool ORDER BY pool_id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(db_error)?
        .ok_or("No pool balance exists")?;
    let updated = current + profit_loss_total_converted;
    if !updated.is_finite() || updated < 0.0
    {
        return Err("Closing this trade would make the pool balance invalid".into());
    }
    transaction
        .execute(
            "INSERT INTO t_pool(pool_value, pool_value_converted)
             SELECT pool_value, ?1 FROM t_pool ORDER BY pool_id DESC LIMIT 1",
            [updated],
        )
        .map_err(db_error)?;
    Ok(())
}

fn db_error(error: rusqlite::Error) -> String
{
    match error
    {
        rusqlite::Error::SqliteFailure(_, Some(message)) => format!("Database: {message}"),
        error => format!("Database: {error}"),
    }
}

pub fn database_path() -> PathBuf
{
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("capitalcommander.sqlite3"))
}
