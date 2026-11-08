use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    vec![Rule {
        from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
        formulas: vec![vec![
            ("Liabilities", 1.0),
            (
                "AccountsPayableAndAccruedLiabilitiesCurrentAndNoncurrent",
                -1.0,
            ),
            ("DividendsPayableCurrentAndNoncurrent", -1.0),
            ("DeferredRevenue", -1.0),
            ("FinanceLeaseLiability", -1.0),
        ]],
        ..Rule::tags("total_debt", "USD", false, &[])
    }]
}
