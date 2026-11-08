use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    vec![Rule {
        from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
        formulas: vec![vec![("SecuredDebt", 1.0), ("LineOfCredit", 1.0)]],
        ..Rule::tags("total_debt", "USD", false, &[])
    }]
}
