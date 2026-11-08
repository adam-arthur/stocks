use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule::tags(
            "cash_and_equivalents",
            "USD",
            false,
            &["CashAndCashEquivalentsAtCarryingValue"],
        ),
        Rule {
            from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
            formulas: vec![
                vec![("CashAndCashEquivalentsAtCarryingValue", 1.0)],
                vec![
                    (
                        "CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents",
                        1.0,
                    ),
                    ("RestrictedCashAndCashEquivalents", -1.0),
                ],
                vec![
                    (
                        "CashCashEquivalentsRestrictedCashAndRestrictedCashEquivalents",
                        1.0,
                    ),
                    ("RestrictedCash", -1.0),
                ],
            ],
            ..Rule::tags("cash_and_equivalents", "USD", false, &[])
        },
    ]
}
