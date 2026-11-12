CREATE TABLE t_trade_premarket (
    trade_premarket_id INTEGER PRIMARY KEY,
    trade_id INTEGER NOT NULL UNIQUE REFERENCES t_trade(trade_id),
    analysis TEXT,
    image BLOB CHECK (image IS NULL OR length(image) <= 2097152),
    is_deleted INTEGER NOT NULL DEFAULT 0 CHECK (is_deleted IN (0, 1)),
    date_created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    date_modified TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

INSERT INTO t_trade_premarket(trade_id) SELECT trade_id FROM t_trade;
