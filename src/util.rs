pub fn format_number(value: f64) -> String
{
    if value.fract() == 0.0
    {
        format!("{value:.0}")
    }
    else
    {
        format!("{value:.2}")
    }
}

pub fn validated_as_positive(input: &str, label: &str) -> Result<f64, String>
{
    let value: f64 = input
        .trim()
        .parse()
        .map_err(|_| format!("{label} must be a number"))?;
    if !value.is_finite() || value < 0.0
    {
        return Err(format!("{label} must be a nonnegative finite number"));
    }
    Ok(value)
}

pub fn check_date(input: &str, label: &str, optional: bool) -> Result<(), String>
{
    if optional && input.is_empty()
    {
        return Ok(());
    }
    let bytes = input.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err(format!("{label} must use YYYY-MM-DD"));
    }
    let year: u16 = input[0..4].parse().unwrap_or(0);
    let month: u8 = input[5..7].parse().unwrap_or(0);
    let day: u8 = input[8..10].parse().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month
    {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day
    {
        return Err(format!("{label} is not a valid date"));
    }
    Ok(())
}

pub fn date_in_range(date: &str, from: &str, to: &str) -> bool
{
    (from.is_empty() || date >= from) && (to.is_empty() || date <= to)
}
