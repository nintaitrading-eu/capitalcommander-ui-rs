-- Adapted from capitalcommander-api-rs Diesel migrations on the develop branch.
-- SQLite keeps the product/reference, trade cost, calculated trade, and trade
-- relationships; financing is a new trade-linked table for this application.
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

CREATE TABLE t_market (
    market_id INTEGER PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    country TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_product_tick_info (
    product_tick_info_id INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    tick REAL NOT NULL CHECK (tick > 0),
    tick_value REAL NOT NULL CHECK (tick_value > 0),
    order_min REAL NOT NULL CHECK (order_min > 0),
    order_max REAL NOT NULL CHECK (order_max >= order_min),
    margin_day_proc REAL NOT NULL CHECK (margin_day_proc >= 0),
    margin_night_proc REAL NOT NULL CHECK (margin_night_proc >= 0),
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
    product_tick_info_id INTEGER REFERENCES t_product_tick_info(product_tick_info_id),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_trade_cost (
    trade_cost_id INTEGER PRIMARY KEY,
    commission REAL NOT NULL DEFAULT 0 CHECK (commission >= 0),
    tax REAL NOT NULL DEFAULT 0 CHECK (tax >= 0),
    other REAL NOT NULL DEFAULT 0 CHECK (other >= 0),
    is_manually_added INTEGER NOT NULL DEFAULT 1 CHECK (is_manually_added IN (0, 1)),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE t_trade_calculated (
    trade_calculated_id INTEGER PRIMARY KEY,
    risk_initial REAL NOT NULL DEFAULT 0 CHECK (risk_initial >= 0),
    profit_loss REAL,
    profit_loss_total REAL,
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
    date_buy TEXT NOT NULL,
    date_sell TEXT,
    is_long INTEGER NOT NULL CHECK (is_long IN (0, 1)),
    shares_buy REAL NOT NULL CHECK (shares_buy > 0),
    shares_sell REAL NOT NULL DEFAULT 0 CHECK (shares_sell >= 0),
    price_buy REAL NOT NULL CHECK (price_buy >= 0),
    price_sell REAL NOT NULL DEFAULT 0 CHECK (price_sell >= 0),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_trade_product ON t_trade(product_id, date_buy);

CREATE TABLE t_financing (
    financing_id INTEGER PRIMARY KEY,
    trade_id INTEGER NOT NULL REFERENCES t_trade(trade_id),
    date TEXT NOT NULL,
    quantity REAL NOT NULL CHECK (quantity > 0),
    price REAL NOT NULL CHECK (price >= 0),
    rate REAL NOT NULL CHECK (rate >= 0),
    exchange_rate REAL NOT NULL CHECK (exchange_rate > 0),
    days INTEGER NOT NULL CHECK (days > 0),
    value REAL NOT NULL CHECK (value >= 0),
    note TEXT NOT NULL DEFAULT '',
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_financing_trade ON t_financing(trade_id, date);

INSERT INTO t_product_type(name, description) VALUES ('cfd', 'Contracts for difference');
INSERT INTO t_currency(code, description) VALUES
    ('EUR', 'Euro'), ('USD', 'United States Dollar'), ('GBP', 'British Pound');
INSERT INTO t_market(code, name, country) VALUES
    ('cfd other non-share', 'CFD - other non-share', ''),
    ('cfd .gold', 'CFD - World Spot Gold', 'US'),
    ('cfd .silver', 'CFD - World Spot Silver', 'US'),
    ('cfd oil', 'CFD - Brent and WTI oil', 'US');

-- Starter catalog from the API product seed, with trailing spaces normalized.
INSERT INTO t_product(name, description, product_type_id, currency_id, market_id) VALUES
    ('.MGOLD.cfd', 'MINI Spot Gold, US Dollar', 1, 2, 2),
    ('.GOLD.cfd', 'Spot Gold, US Dollar', 1, 2, 2),
    ('.MSILVER.cfd', 'Spot Mini Silver, US Dollar/100', 1, 2, 3),
    ('.SILVER.cfd', 'Spot Silver, US Dollar/100', 1, 2, 3),
    ('.BRENT.cfd', 'SPOT Brent Crude Oil, US Dollar/100', 1, 2, 4),
    ('.WTI.cfd', 'SPOT WTI Light Crude Oil, US Dollar', 1, 2, 4),
    ('.N25.cfd', 'Netherlands 25 cash, Euro', 1, 1, 1),
    ('.DE30.cfd', 'Germany 30 cash, Euro', 1, 1, 1),
    ('.ES35.cfd', 'Spain 35 cash, Euro', 1, 1, 1),
    ('.F40.cfd', 'France 40 cash, Euro', 1, 1, 1);
