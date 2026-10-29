use crate::data::{NewTrade, Trade};
use libcalculatorfinance::{
    convert_from_orig,
    calculate_close_at_breakeven, calculate_close_at_preferred,
    calculate_profit_loss, calculate_profit_loss_total, calculate_r_multiple,
    calculate_risk, calculate_risk_initial,
    calculate_stoploss, calculate_stoploss_initial, TradeType,
};

impl Trade
{
    pub fn r_multiple(&self) -> Option<f64>
    {
        self.profit_loss
            .filter(|_| self.risk_initial > 0.0)
            .map(|pl| calculate_r_multiple(pl, self.risk_initial))
    }
}

pub(crate) fn trade_calculations(trade: &NewTrade, pool_value: f64, financing_total: f64) -> Result<
    (
        Option<f64>,
        Option<f64>,
        f64,
        Option<f64>,
        f64,
        f64,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    ),
    String,
>
{
    let stoploss_initial = match (trade.stoploss_initial, trade.risk_percent_initial)
    {
        (Some(stoploss_initial), _) => Some(stoploss_initial),
        (None, Some(risk_percent_initial)) => Some(calculate_stoploss_initial(
            trade.price_buy,
            trade.shares_buy,
            trade.tax_buy,
            trade.commission_buy,
            trade.price_sell,
            trade.shares_sell,
            trade.tax_sell,
            trade.commission_sell,
            trade.risk_initial,
            pool_value,
            trade_type(trade.is_long),
        )),
        (None, None) => None,
    };
    let risk_initial = if let Some(stoploss_initial) = stoploss_initial
    {
        calculate_risk_initial(
            trade.price_buy,
            trade.shares_buy,
            trade.tax_buy,
            trade.commission_buy,
            trade.price_sell,
            trade.shares_sell,
            trade.tax_sell,
            trade.commission_sell,
            stoploss_initial,
            trade_type(trade.is_long),
        )
    }
    else
    {
        trade.risk_initial
    };
    if !risk_initial.is_finite() || risk_initial < 0.0
    {
        return Err("Initial risk must be finite and nonnegative. Check the stoploss value.".into());
    }
    let (entry_buy_price, entry_buy_shares, entry_buy_tax, entry_buy_commission,
         entry_sell_price, entry_sell_shares, entry_sell_tax, entry_sell_commission) = if trade.is_long
    {
        (trade.price_buy, trade.shares_buy, trade.tax_buy, trade.commission_buy,
         0.0, 0, 0.0, 0.0)
    }
    else
    {
        (0.0, 0, 0.0, 0.0,
         trade.price_sell, trade.shares_sell, trade.tax_sell, trade.commission_sell)
    };
    let target_minimum = calculate_close_at_breakeven(
        entry_buy_price, entry_buy_shares, entry_buy_tax, entry_buy_commission,
        entry_sell_price, entry_sell_shares, entry_sell_tax, entry_sell_commission,
        trade_type(trade.is_long),
    );
    if !target_minimum.is_finite() || target_minimum < 0.0
    {
        return Err("Minimum target must be finite and nonnegative. Check the entry price and tax.".into());
    }
    let target_preferred = stoploss_initial.map(|_| calculate_close_at_preferred(
        entry_buy_price, entry_buy_shares, entry_buy_tax, entry_buy_commission,
        entry_sell_price, entry_sell_shares, entry_sell_tax, entry_sell_commission,
        risk_initial, trade_type(trade.is_long),
    ));
    if target_preferred.is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("Preferred target must be finite and nonnegative. Check the entry price and tax.".into());
    }
    let profit_loss = trade.date_sell.as_ref().map(|_| {
        calculate_profit_loss(
            trade.price_buy,
            trade.shares_buy,
            trade.price_sell,
            trade.shares_sell,
            trade_type(trade.is_long),
        )
    });
    let profit_loss_total = trade.date_sell.as_ref().map(|_| {
        calculate_profit_loss_total(
            trade.price_buy,
            trade.shares_buy,
            trade.tax_buy,
            trade.commission_buy,
            trade.price_sell,
            trade.shares_sell,
            trade.tax_sell,
            trade.commission_sell,
            trade_type(trade.is_long),
        ) + trade.cost_other
    });
    let r_multiple = profit_loss
        .filter(|_| risk_initial > 0.0)
        .map(|pl| calculate_r_multiple(pl, risk_initial));
    let risk_actual = profit_loss.map(|pl| {
        calculate_risk(
            trade.price_buy,
            trade.shares_buy,
            trade.tax_buy,
            trade.commission_buy,
            trade.price_sell,
            trade.shares_sell,
            trade.tax_sell,
            trade.commission_sell,
            risk_initial,
            pl,
            trade_type(trade.is_long),
        )
    });
    if risk_actual.is_some() && (!pool_value.is_finite() || pool_value <= 0.0)
    {
        return Err("The pool value must be positive to calculate the actual stoploss".into());
    }
    let stoploss_actual = risk_actual.map(|risk| {
        calculate_stoploss(
            trade.price_buy,
            trade.shares_buy,
            trade.tax_buy,
            trade.commission_buy,
            trade.price_sell,
            trade.shares_sell,
            trade.tax_sell,
            trade.commission_sell,
            risk / pool_value * 100.0,
            pool_value,
            trade_type(trade.is_long),
        )
    });
    if stoploss_actual.is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err("Actual stoploss must be finite and nonnegative. Check the closing quantity and tax.".into());
    }


    let risk_initial_converted = convert_from_orig(risk_initial, trade.exchange_rate_buy);
    let risk_actual_converted = if risk_actual.is_some()
    {
        Some(convert_from_orig(risk_actual.unwrap(), if trade.is_long { trade.exchange_rate_sell } else { trade.exchange_rate_buy }))
    }
    else
    {
        None
    };

    // TODO: Only do this when the trade is closed? Refactor this.
    let profit_loss_converted = if profit_loss.is_some()
    {
        Some(convert_from_orig(profit_loss.unwrap(), if trade.is_long { trade.exchange_rate_sell } else { trade.exchange_rate_buy }))
    }
    else
    {
        None
    };

    let profit_loss_total_converted = if profit_loss_total.is_some()
    {
        Some(convert_from_orig(profit_loss_total.unwrap(), if trade.is_long { trade.exchange_rate_sell } else { trade.exchange_rate_buy }))
    }
    else
    {
        None
    };

    Ok((
        stoploss_initial,
        stoploss_actual,
        target_minimum,
        target_preferred,
        risk_initial,
        risk_initial_converted,
        risk_actual,
        risk_actual_converted,
        profit_loss,
        profit_loss_converted,
        profit_loss_total,
        profit_loss_total_converted,
        r_multiple,
    ))
}

pub fn trade_type(is_long: bool) -> TradeType
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
