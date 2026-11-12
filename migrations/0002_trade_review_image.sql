ALTER TABLE t_trade_review ADD COLUMN image BLOB CHECK (image IS NULL OR length(image) <= 2097152);
