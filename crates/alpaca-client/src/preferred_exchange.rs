use chrono::NaiveDate;
use serde_json::Value;

pub fn exchanges<'a>(
    actions: &'a [Value],
    symbol: &'a str,
    start: NaiveDate,
    end: NaiveDate,
) -> impl Iterator<Item = (&'a Value, NaiveDate)> {
    actions.iter().filter_map(move |action| {
        if action["type"] != "stock_mergers" || action["acquirer_symbol"] != symbol {
            return None;
        }
        let date = action["effective_date"].as_str()?.parse().ok()?;
        (start < date
            && date <= end
            && !actions.iter().any(|other| {
                other["type"] == "cash_dividends"
                    && other["symbol"] == symbol
                    && other["cusip"] == action["acquirer_cusip"]
                    && other["ex_date"]
                        .as_str()
                        .and_then(|d| d.parse::<NaiveDate>().ok())
                        .is_some_and(|ex| ex < date)
            }))
        .then_some((action, date))
    })
}

pub const CADENCES: [(std::ops::RangeInclusive<i64>, i64); 5] = [
    (5..=9, 52),
    (24..=38, 12),
    (75..=110, 4),
    (150..=215, 2),
    (330..=400, 1),
];
