CREATE TABLE t_product_type (
    product_type_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_currency (
    currency_id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE CHECK (length(code) = 3),
    description TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_parameter (
    parameter_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE CHECK (length(name) <= 50),
    value TEXT NOT NULL CHECK (length(value) <= 512),
    value_default TEXT NOT NULL CHECK (length(value_default) <= 512),
    description TEXT NOT NULL CHECK (length(description) <= 4000),
    datatype TEXT NOT NULL CHECK (length(datatype) <= 50),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_market (
    market_id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    country TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_product (
    product_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK (length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    product_type_id INTEGER NOT NULL REFERENCES t_product_type(product_type_id),
    currency_id INTEGER NOT NULL REFERENCES t_currency(currency_id),
    market_id INTEGER NOT NULL REFERENCES t_market(market_id),
    tick REAL NOT NULL CHECK (tick > 0),
    tick_value REAL NOT NULL CHECK (tick_value > 0),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_trade_cost (
    trade_cost_id INTEGER PRIMARY KEY,
    commission_buy REAL NOT NULL DEFAULT 0 CHECK (commission_buy >= 0),
    tax_buy REAL NOT NULL DEFAULT 0 CHECK (tax_buy >= 0),
    commission_sell REAL NOT NULL DEFAULT 0 CHECK (commission_sell >= 0),
    tax_sell REAL NOT NULL DEFAULT 0 CHECK (tax_sell >= 0),
    other REAL NOT NULL DEFAULT 0 CHECK (other >= 0),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_pool(
    pool_id INTEGER PRIMARY KEY,
    pool_value REAL NOT NULL DEFAULT 0 CHECK (pool_value >= 0),
    pool_value_converted REAL NOT NULL DEFAULT 0 CHECK (pool_value_converted >= 0),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_trade_calculated (
    trade_calculated_id INTEGER PRIMARY KEY,
    risk_initial REAL NOT NULL DEFAULT 0 CHECK (risk_initial >= 0),
    risk_initial_converted REAL NOT NULL DEFAULT 0 CHECK (risk_initial_converted >= 0),
    risk_actual REAL CHECK (risk_actual >= 0),
    risk_actual_converted REAL CHECK (risk_actual_converted >= 0),
    risk_percent_initial REAL NOT NULL DEFAULT 0 CHECK (risk_percent_initial >= 0),
    risk_percent_actual REAL CHECK (risk_percent_actual >= 0),
    stoploss_initial REAL NOT NULL DEFAULT 0 CHECK (stoploss_initial >= 0),
    stoploss_actual REAL CHECK (stoploss_actual >= 0),
    profit_loss REAL,
    profit_loss_converted REAL,
    profit_loss_total REAL,
    profit_loss_total_converted REAL,
    r_multiple REAL,
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_trade (
    trade_id INTEGER PRIMARY KEY,
    trade_calculated_id INTEGER NOT NULL UNIQUE REFERENCES t_trade_calculated(trade_calculated_id),
    product_id INTEGER NOT NULL REFERENCES t_product(product_id),
    trade_cost_id INTEGER NOT NULL UNIQUE REFERENCES t_trade_cost(trade_cost_id),
    pool_id INTEGER NOT NULL UNIQUE REFERENCES t_pool(pool_id),
    date_buy TEXT NOT NULL,
    date_sell TEXT,
    is_long INTEGER NOT NULL CHECK (is_long IN (0, 1)),
    shares_buy INTEGER NOT NULL CHECK (shares_buy > 0),
    shares_sell INTEGER NOT NULL DEFAULT 0 CHECK (shares_sell >= 0),
    price_buy REAL NOT NULL CHECK (price_buy >= 0),
    price_sell REAL NOT NULL DEFAULT 0 CHECK (price_sell >= 0),
    exchange_rate_buy REAL NOT NULL DEFAULT 1 CHECK (exchange_rate_buy > 0),
    exchange_rate_sell REAL NOT NULL DEFAULT 1 CHECK (exchange_rate_sell > 0),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_trade_product ON t_trade(product_id, date_buy);

CREATE TABLE t_financing (
    financing_id INTEGER PRIMARY KEY,
    trade_id INTEGER NOT NULL REFERENCES t_trade(trade_id),
    date TEXT NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    price REAL NOT NULL CHECK (price >= 0),
    rate REAL NOT NULL CHECK (rate >= 0),
    exchange_rate REAL NOT NULL CHECK (exchange_rate > 0),
    days INTEGER NOT NULL CHECK (days > 0),
    value REAL NOT NULL CHECK (value >= 0),
    comment TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_financing_trade ON t_financing(trade_id, date);

----------------------------------------------------------------
-- Add initial values.
----------------------------------------------------------------
INSERT INTO t_pool(pool_value, pool_value_converted) VALUES
    (75000.0, 75000.0);
INSERT INTO t_product_type(name, description) VALUES
    ('futures', 'Futures contracts');
INSERT INTO t_currency(code, description) VALUES
    ('EUR', 'Euro'),
    ('USD', 'United States Dollar'),
    ('GBP', 'British Pound');
INSERT INTO t_parameter(name, value, value_default, description, datatype, date_created) VALUES
    ('base_currency', 'EUR', 'EUR', 'Base currency', 'string', strftime('%Y-%m-%dT%H:%M:%fZ','now'));
INSERT INTO t_market(code, name, country) VALUES
    ('EUREX', 'EUREX - Indices, equities, ...', ''),
    ('COMEX', 'COMEX - Precious metals, commodities, ...', '');

-- Tick = basis point or move
-- It is usually 1.0, unless we are dealing with Forex or something.
INSERT INTO t_product(name, description, product_type_id, currency_id, market_id, tick, tick_value) VALUES
    ('FDX', 'Micro-DAX', 1, 1, 1, 1.0, 1.0),
    ('FDXM', 'Mini-DAX', 1, 1, 1, 1.0, 5.0),
    ('MGC', 'Micro GOLD', 1, 1, 2, 1.0, 10.0),
    ('QO', 'E-mini GOLD', 1, 1, 2, 1.0, 50.0);
