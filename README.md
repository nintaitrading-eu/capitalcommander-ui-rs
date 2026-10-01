# CapitalCommander trade journal

A Slint desktop CFD trading journal using a local SQLite database. The UI keeps the dark Gruvbox palette and menu layout of `capitalcommander-slint-rs`.

## Run

```sh
cargo run
```

The app creates `capitalcommander.sqlite3` in the current directory on first run. Pass a database path as the first argument to use another file. The spreadsheet is an example only and is never opened. The database begins with a small CFD product catalog adapted from `capitalcommander-api-rs`; add more products under **Admin → Products** and markets under **Admin → Markets**.

The SQLite migration in `migrations/0001_initial.sql` adapts the API's Diesel tables for product types, currencies, markets, products, trade costs, trades, and calculated trade values. It adds financing entries linked to trades. Foreign keys and schema versioning are enabled. New trades and financing entries are saved immediately.

The product selector filters trades, financing, and statistics by product name. In the journal, enter the buy details and leave the sell date empty to save an open trade. Click its journal row later to load it into the same form, enter the sell date, price, commission, tax, and other costs, then choose **Save changes**. **New trade** clears the selection and form. Editing keeps the trade ID and linked financing entries.

**Trade → Trade costs** shows buy and sell commissions, taxes, other costs, and their totals in EUR for each trade. On the Journal and Trade costs pages, choose either end of the inclusive buy date range from its calendar, then select **Apply**. Use the × beside a date to leave that end open, or **Clear** to restore all trades. Open trades are included, and financing entries are shown separately under **Finance → Financing**.

Enter buy and sell exchange rates as EUR per unit of the product currency (use 1 for EUR). The app stores the original prices and rates, then converts each price to EUR for P/L, R, and pool calculations. Enter initial risk, commission, tax, and other costs in EUR. Existing databases gain exchange columns with a default rate of 1.

Expectancy is the average of each closed trade's gross P/L divided by its positive initial risk. Trades without positive initial risk are omitted. Commission, tax, and other costs are stored in separate `t_trade_cost` columns and deducted for net P/L in the calculated table. Financing value can be entered or calculated from quantity, price, rate, exchange rate, and days on a 360 day basis.

## Verify

```sh
cargo test
```
