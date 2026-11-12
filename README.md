# CapitalCommander trade journal

A Slint desktop CFD trading journal using a local SQLite database. The UI keeps the dark Gruvbox palette and menu layout of `capitalcommander-slint-rs`.

## Run

```sh
cargo run
```

The app creates `capitalcommander.sqlite3` in the current directory on first run. Pass a database path as the first argument to use another file. The spreadsheet is an example only and is never opened. The database begins with a small CFD product catalog adapted from `capitalcommander-api-rs`; add more products under **Admin → Products** and markets under **Admin → Markets**.

The SQLite migration in `migrations/0001_initial.sql` adapts the API's Diesel tables for product types, currencies, markets, products, trade costs, trades, and calculated trade values. Foreign keys and schema versioning are enabled. New trades are saved immediately.

The product selector filters trades and statistics by product name. In the journal, choose Long or Short and enter its entry date, shares, commission, and tax. Leave the exit date and unknown exit details empty to save an open trade. Click its journal row later to load it into the same form, enter the exit date, price, shares, commission, tax, and other costs, then choose **Save changes**. **New trade** clears the selection and form. Editing keeps the trade ID.

**Trade → Review** shows the journal results. Select any trade, write its review in the text area above the table, then choose **Save review**. Use **Add image** to attach one image to the selected trade (maximum 2 MB); the image appears below the review and can be removed with **Delete image**. Images are stored in that trade's linked `t_trade_review` record. The buy date filter above the table is shared with the Journal, so applying or clearing it updates both views.

**Report → Trade costs** shows buy and sell commissions, taxes, other costs, and their totals in EUR for each trade. On the Journal and Trade costs pages, choose either end of the inclusive entry date range from its calendar, then select **Apply**. Use the × beside a date to leave that end open, or **Clear** to restore all trades. Open trades are included.

Enter buy and sell exchange rates as EUR per unit of the product currency (use 1 for EUR). The app stores the original prices and rates, then converts each price to EUR for P/L, R, and pool calculations. Enter initial risk, commission, tax, and other costs in EUR.

Expectancy is the average of each closed trade's gross P/L divided by its positive initial risk. Trades without positive initial risk are omitted. Commission, tax, and other costs are stored in separate `t_trade_cost` columns and deducted for net P/L in the calculated table.

**Trade → Pre-market analysis** uses the same trade list and date filter as Review. Select a trade to save analysis text and an image. Right-click a trade in the Journal, Review, or Pre-market results to jump between the two screens. Open trades also have an **Abandon** action; abandoned trades remain visible in faded rows and are excluded from performance and cost totals.

## Verify

```sh
cargo test
```
