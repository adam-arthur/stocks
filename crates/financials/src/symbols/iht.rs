use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    vec![Rule {
        from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
        ..Rule::tags(
            "cash_and_equivalents",
            "USD",
            false,
            &["CashAndCashEquivalentsAtCarryingValue", "Cash"],
        )
    }]
}
