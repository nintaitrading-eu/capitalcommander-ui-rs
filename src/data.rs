use libcalculatorfinance::{
    calculate_profit_loss, calculate_profit_loss_total, calculate_r_multiple,
    calculate_risk_actual, calculate_risk_initial, calculate_risk_input, calculate_stoploss,
    convert_from_orig, TradeType,
};
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
pub struct Trade
{
    pub id: i64,
    pub product: String,
    pub buy_date: String,
    pub sell_date: String,
    pub is_long: bool,
    pub quantity: f64,
    pub buy_price: f64,
    pub sell_price: f64,
    pub exchange_rate_buy: f64,
    pub exchange_rate_sell: f64,
    pub commission_buy: f64,
    pub tax_buy: f64,
    pub commission_sell: f64,
    pub tax_sell: f64,
    pub other_costs: f64,
    pub initial_risk: f64,
    pub actual_risk: Option<f64>,
    pub risk_percent: Option<f64>,
    pub risk_pool: Option<f64>,
    pub profit_loss: Option<f64>,
    pub profit_loss_total: Option<f64>,
}

impl Trade
{
    pub fn displayed_actual_risk(&self) -> Option<f64>
    {
        self.actual_risk.or_else(|| {
            self.profit_loss.map(|profit_loss| {
                calculate_risk_actual(
                    convert_from_orig(self.buy_price, self.exchange_rate_buy) * self.quantity,
                    1,
                    0.0,
                    self.commission_buy + self.tax_buy,
                    convert_from_orig(self.sell_price, self.exchange_rate_sell) * self.quantity,
                    1,
                    0.0,
                    self.commission_sell + self.tax_sell,
                    self.initial_risk,
                    profit_loss,
                    trade_type(self.is_long),
                )
            })
        })
    }

    pub fn r_multiple(&self) -> Option<f64>
    {
        self.profit_loss
            .filter(|_| self.initial_risk > 0.0)
            .map(|pl| calculate_r_multiple(pl, self.initial_risk))
    }
}

#[derive(Clone, Debug)]
pub struct Financing
{
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

pub struct NewTrade
{
    pub product: String,
    pub buy_date: String,
    pub sell_date: Option<String>,
    pub is_long: bool,
    pub quantity: f64,
    pub buy_price: f64,
    pub sell_price: f64,
    pub exchange_rate_buy: f64,
    pub exchange_rate_sell: f64,
    pub commission_buy: f64,
    pub tax_buy: f64,
    pub commission_sell: f64,
    pub tax_sell: f64,
    pub other_costs: f64,
    pub initial_risk: f64,
    pub risk_percent: Option<f64>,
    pub risk_pool: Option<f64>,
}

pub struct NewFinancing
{
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

pub struct Journal
{
    pub products: Vec<Product>,
    pub trades: Vec<Trade>,
    pub financing: Vec<Financing>,
    pub pool_value: Option<f64>,
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
                    .pragma_update(None, "user_version", 0)
                    .map_err(db_error)?;
                transaction.commit().map_err(db_error)?;
            }
            other => return Err(format!("Unsupported database schema version {other}")),
        }
        Ok(Self { connection })
    }

    pub fn load(&self) -> Result<Journal, String>
    {
        let pool_value = self
            .connection
            .query_row(
                "SELECT pool_value FROM t_pool ORDER BY pool_id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_error)?;
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
                    t.is_long, t.shares_buy, t.price_buy, t.price_sell,
                    cost.commission_buy, cost.tax_buy, cost.commission_sell,
                    cost.tax_sell, cost.other, calc.risk_initial, calc.profit_loss, calc.profit_loss_total,
                    t.exchange_rate_buy, t.exchange_rate_sell, calc.risk_actual,
                    calc.risk_percent, calc.risk_pool, calc.stoploss
             FROM t_trade t JOIN t_product p ON p.product_id = t.product_id
             JOIN t_trade_cost cost ON cost.trade_cost_id = t.trade_cost_id
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
                    buy_price: row.get(6)?,
                    sell_price: row.get(7)?,
                    commission_buy: row.get(8)?,
                    tax_buy: row.get(9)?,
                    commission_sell: row.get(10)?,
                    tax_sell: row.get(11)?,
                    other_costs: row.get(12)?,
                    initial_risk: row.get(13)?,
                    profit_loss: row.get(14)?,
                    profit_loss_total: row.get(15)?,
                    exchange_rate_buy: row.get(16)?,
                    exchange_rate_sell: row.get(17)?,
                    actual_risk: row.get(18)?,
                    risk_percent: row.get(19)?,
                    risk_pool: row.get(20)?,
                })
            })
            .map_err(db_error)?;
        for row in rows
        {
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
        for row in rows
        {
            financing.push(row.map_err(db_error)?);
        }
        Ok(Journal {
            products,
            trades,
            financing,
            pool_value,
        })
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
        let (stoploss, initial_risk, actual_risk, profit_loss, profit_loss_total, r_multiple) =
            trade_calculations(&trade, 0.0)?;
        let transaction = self.connection.transaction().map_err(db_error)?;
        transaction
            .execute(
                "INSERT INTO t_trade_cost(commission_buy, tax_buy, commission_sell, tax_sell, other) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![trade.commission_buy, trade.tax_buy, trade.commission_sell, trade.tax_sell, trade.other_costs],
            )
            .map_err(db_error)?;
        let cost_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO t_trade_calculated(risk_initial, risk_actual, risk_percent, risk_pool, stoploss,
                                              profit_loss, profit_loss_total, r_multiple)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![initial_risk, actual_risk, trade.risk_percent, trade.risk_pool, stoploss,
                    profit_loss, profit_loss_total, r_multiple]
        ).map_err(db_error)?;
        let calculated_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO t_trade(trade_calculated_id, product_id, trade_cost_id, date_buy, date_sell,
                                 is_long, shares_buy, shares_sell, price_buy, price_sell, exchange_rate_buy, exchange_rate_sell)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![calculated_id, product_id, cost_id, trade.buy_date, trade.sell_date,
                    trade.is_long, trade.quantity, if profit_loss.is_some() { trade.quantity } else { 0.0 },
                    trade.buy_price, trade.sell_price, trade.exchange_rate_buy, trade.exchange_rate_sell]
        ).map_err(db_error)?;
        if let Some(profit_loss_total) = profit_loss_total
        {
            record_pool_profit_loss(&transaction, profit_loss_total)?;
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
        let financing_total: f64 = self.connection.query_row(
            "SELECT COALESCE(SUM(value), 0) FROM t_financing WHERE trade_id = ?1 AND is_deleted = 0",
            [id],
            |row| row.get(0),
        ).map_err(db_error)?;
        let (stoploss, initial_risk, actual_risk, profit_loss, profit_loss_total, r_multiple) =
            trade_calculations(&trade, financing_total)?;
        let (cost_id, calculated_id): (i64, i64) = self
            .connection
            .query_row(
                "SELECT trade_cost_id, trade_calculated_id FROM t_trade
             WHERE trade_id = ?1 AND is_deleted = 0 AND date_sell IS NULL",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
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
                    trade.other_costs,
                    cost_id
                ],
            )
            .map_err(db_error)?;
        transaction
            .execute(
                "UPDATE t_trade SET product_id = ?1, date_buy = ?2, date_sell = ?3, is_long = ?4,
             shares_buy = ?5, shares_sell = ?6, price_buy = ?7, price_sell = ?8,
             exchange_rate_buy = ?9, exchange_rate_sell = ?10,
             date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE trade_id = ?11",
                params![
                    product_id,
                    trade.buy_date,
                    trade.sell_date,
                    trade.is_long,
                    trade.quantity,
                    if profit_loss.is_some()
                    {
                        trade.quantity
                    }
                    else
                    {
                        0.0
                    },
                    trade.buy_price,
                    trade.sell_price,
                    trade.exchange_rate_buy,
                    trade.exchange_rate_sell,
                    id
                ],
            )
            .map_err(db_error)?;
        transaction
            .execute(
                "UPDATE t_trade_calculated SET risk_initial = ?1, risk_actual = ?2,
             risk_percent = ?3, risk_pool = ?4, stoploss = ?5, profit_loss = ?6,
             profit_loss_total = ?7, r_multiple = ?8,
             date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE trade_calculated_id = ?9",
                params![
                    initial_risk,
                    actual_risk,
                    trade.risk_percent,
                    trade.risk_pool,
                    stoploss,
                    profit_loss,
                    profit_loss_total,
                    r_multiple,
                    calculated_id
                ],
            )
            .map_err(db_error)?;
        if let Some(profit_loss_total) = profit_loss_total
        {
            record_pool_profit_loss(&transaction, profit_loss_total)?;
        }
        transaction.commit().map_err(db_error)?;
        Ok(())
    }

    pub fn add_financing(&mut self, entry: NewFinancing) -> Result<(), String>
    {
        let (
            calculated_id,
            is_long,
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
            is_closed,
        ): (i64, bool, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, bool) = self
            .connection
            .query_row(
                "SELECT t.trade_calculated_id, t.is_long, t.shares_buy, t.price_buy, t.price_sell,
                        t.exchange_rate_buy, t.exchange_rate_sell, cost.commission_buy, cost.tax_buy,
                        cost.commission_sell, cost.tax_sell, cost.other, t.date_sell IS NOT NULL
                 FROM t_trade t JOIN t_trade_cost cost ON cost.trade_cost_id = t.trade_cost_id
                 WHERE t.trade_id = ?1 AND t.is_deleted = 0",
                [entry.trade_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                        row.get(12)?,
                    ))
                },
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
        let financing_total: f64 = transaction.query_row(
            "SELECT COALESCE(SUM(value), 0) FROM t_financing WHERE trade_id = ?1 AND is_deleted = 0",
            [entry.trade_id],
            |row| row.get(0),
        ).map_err(db_error)?;
        let profit_loss_total = if is_closed
        {
            Some(net_profit_loss(
                is_long,
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
                financing_total,
            )?)
        }
        else
        {
            None
        };
        transaction
            .execute(
                "UPDATE t_trade_calculated SET profit_loss_total = ?1,
                date_modified = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE trade_calculated_id = ?2",
                params![profit_loss_total, calculated_id],
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(())
    }
}

fn record_pool_profit_loss(transaction: &Transaction<'_>, profit_loss_total: f64) -> Result<(), String>
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
        .execute("INSERT INTO t_pool(pool_value) VALUES (?1)", [updated])
        .map_err(db_error)?;
    Ok(())
}

fn trade_calculations(
    trade: &NewTrade,
    financing_total: f64,
) -> Result<(Option<f64>, f64, Option<f64>, Option<f64>, Option<f64>, Option<f64>), String>
{
    let stoploss = match (trade.risk_percent, trade.risk_pool)
    {
        (Some(percent), Some(pool)) => Some(calculate_trade_stoploss(
            percent, pool, trade.buy_price, trade.quantity, trade.exchange_rate_buy,
            trade.commission_buy + trade.commission_sell + trade.tax_buy + trade.tax_sell,
            trade.is_long,
        )?),
        (None, None) => None,
        _ => return Err("Risk % requires a pool value".into()),
    };
    let initial_risk = if let Some(stoploss) = stoploss
    {
        // The calculator uses integer shares and percentage taxes. A single
        // share at the full position value preserves fractional quantities;
        // the average fixed cost covers both sides of a stopped trade.
        calculate_risk_initial(
            convert_from_orig(trade.buy_price, trade.exchange_rate_buy) * trade.quantity,
            1,
            0.0,
            (trade.commission_buy + trade.commission_sell + trade.tax_buy + trade.tax_sell) / 2.0,
            convert_from_orig(stoploss, trade.exchange_rate_buy) * trade.quantity,
            trade.is_long,
        )
    }
    else
    {
        trade.initial_risk // Existing trades without a recorded risk percentage.
    };
    if !initial_risk.is_finite() || initial_risk < 0.0
    {
        return Err("Initial risk must be finite and nonnegative; check the stop loss".into());
    }
    let profit_loss = trade.sell_date.as_ref().map(|_| {
        let buy_price = convert_from_orig(trade.buy_price, trade.exchange_rate_buy);
        let sell_price = convert_from_orig(trade.sell_price, trade.exchange_rate_sell);
        calculate_profit_loss(
            buy_price * trade.quantity,
            1,
            sell_price * trade.quantity,
            1,
            trade_type(trade.is_long),
        )
    });
    if profit_loss.is_some_and(|value| !value.is_finite())
    {
        return Err("Calculated P/L is too large".into());
    }
    let profit_loss_total = trade
        .sell_date
        .as_ref()
        .map(|_| {
            net_profit_loss(
                trade.is_long,
                trade.quantity,
                trade.buy_price,
                trade.sell_price,
                trade.exchange_rate_buy,
                trade.exchange_rate_sell,
                trade.commission_buy,
                trade.tax_buy,
                trade.commission_sell,
                trade.tax_sell,
                trade.other_costs,
                financing_total,
            )
        })
        .transpose()?;
    let r_multiple = profit_loss
        .filter(|_| initial_risk > 0.0)
        .map(|pl| calculate_r_multiple(pl, initial_risk));
    if r_multiple.is_some_and(|value| !value.is_finite())
    {
        return Err("Calculated R is too large".into());
    }
    let actual_risk = profit_loss.map(|pl| {
        calculate_risk_actual(
            convert_from_orig(trade.buy_price, trade.exchange_rate_buy) * trade.quantity,
            1,
            0.0,
            trade.commission_buy + trade.tax_buy,
            convert_from_orig(trade.sell_price, trade.exchange_rate_sell) * trade.quantity,
            1,
            0.0,
            trade.commission_sell + trade.tax_sell,
            initial_risk,
            pl,
            trade_type(trade.is_long),
        )
    });
    if actual_risk.is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("Calculated actual risk is invalid".into());
    }
    Ok((
        stoploss,
        initial_risk,
        actual_risk,
        profit_loss,
        profit_loss_total,
        r_multiple,
    ))
}

#[allow(clippy::too_many_arguments)]
pub fn calculate_trade_stoploss(
    risk_percent: f64,
    pool: f64,
    buy_price: f64,
    quantity: f64,
    exchange_rate_buy: f64,
    round_trip_cost: f64,
    is_long: bool,
) -> Result<f64, String>
{
    if !risk_percent.is_finite() || risk_percent < 0.0 || !pool.is_finite() || pool < 0.0
        || !buy_price.is_finite() || buy_price < 0.0 || !quantity.is_finite() || quantity <= 0.0
        || !exchange_rate_buy.is_finite() || exchange_rate_buy <= 0.0
        || !round_trip_cost.is_finite() || round_trip_cost < 0.0
    {
        return Err("Invalid input for stoploss calculation".into());
    }
    let risk_input = calculate_risk_input(pool, risk_percent);
    if !risk_input.is_finite()
    {
        return Err("Risk input is too large".into());
    }
    // The calculator takes integer shares. One share at the position's EUR
    // value preserves fractional quantities and absolute transaction costs.
    let price_eur = convert_from_orig(buy_price, exchange_rate_buy) * quantity;
    let stoploss_eur = calculate_stoploss(
        price_eur, 1, 0.0, round_trip_cost / 2.0, risk_percent, pool, is_long,
    );
    let stoploss = stoploss_eur / exchange_rate_buy / quantity;
    if !stoploss.is_finite() || stoploss < 0.0
    {
        return Err("Calculated stoploss is invalid; check the risk, price, and quantity".into());
    }
    Ok(stoploss)
}

fn trade_type(is_long: bool) -> TradeType
{
    if is_long
    {
        TradeType::Long
    }
    else
    {
        TradeType::Short
    }
}

#[allow(clippy::too_many_arguments)]
fn net_profit_loss(
    is_long: bool,
    quantity: f64,
    buy_price: f64,
    sell_price: f64,
    exchange_rate_buy: f64,
    exchange_rate_sell: f64,
    commission_buy: f64,
    tax_buy: f64,
    commission_sell: f64,
    tax_sell: f64,
    other_costs: f64,
    financing_total: f64,
) -> Result<f64, String>
{
    let buy_price = convert_from_orig(buy_price, exchange_rate_buy);
    let sell_price = convert_from_orig(sell_price, exchange_rate_sell);
    // The library accepts integer shares and percentage taxes. Use one share at the
    // full position value, and pass the database's absolute taxes as fixed costs.
    let result = calculate_profit_loss_total(
        buy_price * quantity,
        1,
        0.0,
        commission_buy + tax_buy,
        sell_price * quantity,
        1,
        0.0,
        commission_sell + tax_sell + other_costs + financing_total,
        trade_type(is_long),
    );
    if !result.is_finite()
    {
        return Err("Calculated total P/L is too large".into());
    }
    Ok(result)
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
