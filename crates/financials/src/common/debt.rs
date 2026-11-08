use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    let mut rules = [
        (
            "debt_due_within_one_year",
            "USD",
            false,
            &["DebtCurrent"][..],
        ),
        (
            "finance_leases_due_within_one_year",
            "USD",
            false,
            &[
                "FinanceLeaseLiabilityCurrent",
                "CapitalLeaseObligationsCurrent",
            ][..],
        ),
        ("long_term_debt", "USD", false, &["LongTermDebt"][..]),
        (
            "long_term_debt_due_within_one_year",
            "USD",
            false,
            &["LongTermDebtCurrent"][..],
        ),
        (
            "debt_due_after_one_year",
            "USD",
            false,
            &["LongTermDebtNoncurrent"][..],
        ),
        ("commercial_paper", "USD", false, &["CommercialPaper"][..]),
        (
            "other_short_term_debt",
            "USD",
            false,
            &["OtherShortTermBorrowings"][..],
        ),
    ]
    .into_iter()
    .map(|(metric, unit, duration, tags)| Rule::tags(metric, unit, duration, tags))
    .collect::<Vec<_>>();
    rules.extend([
        Rule {
            formulas: vec![
                vec![("ShortTermBorrowings", 1.0)],
                vec![("CommercialPaper", 1.0), ("OtherShortTermBorrowings", 1.0)],
            ],
            ..Rule::tags("short_term_debt", "USD", false, &[])
        },
        Rule {
            formulas: vec![
                vec![("DebtLongtermAndShorttermCombinedAmount", 1.0)],
                vec![
                    ("DebtCurrent", 1.0),
                    ("LongTermDebtNoncurrent", 1.0),
                    ("FinanceLeaseLiabilityCurrent", -1.0),
                ],
                vec![
                    ("DebtCurrent", 1.0),
                    ("LongTermDebtNoncurrent", 1.0),
                    ("CapitalLeaseObligationsCurrent", -1.0),
                ],
                vec![("LongTermDebt", 1.0), ("ShortTermBorrowings", 1.0)],
                vec![
                    ("LongTermDebt", 1.0),
                    ("CommercialPaper", 1.0),
                    ("OtherShortTermBorrowings", 1.0),
                ],
                vec![
                    ("LongTermDebtCurrent", 1.0),
                    ("LongTermDebtNoncurrent", 1.0),
                    ("ShortTermBorrowings", 1.0),
                ],
                vec![
                    ("LongTermDebtCurrent", 1.0),
                    ("LongTermDebtNoncurrent", 1.0),
                    ("CommercialPaper", 1.0),
                    ("OtherShortTermBorrowings", 1.0),
                ],
                vec![("LongTermDebt", 1.0)],
            ],
            ..Rule::tags("total_debt", "USD", false, &[])
        },
    ]);
    rules.push(Rule {
        from: chrono::NaiveDate::from_ymd_opt(2026, 1, 1),
        formulas: rules
            .last()
            .unwrap()
            .formulas
            .iter()
            .cloned()
            .chain([
                vec![
                    ("LongTermDebtCurrent", 1.0),
                    ("LongTermDebtNoncurrent", 1.0),
                ],
                vec![
                    ("DebtAndCapitalLeaseObligations", 1.0),
                    ("FinanceLeaseLiability", -1.0),
                ],
                vec![
                    (
                        "LongTermDebtAndCapitalLeaseObligationsIncludingCurrentMaturities",
                        1.0,
                    ),
                    ("FinanceLeaseLiability", -1.0),
                ],
            ])
            .collect(),
        ..Rule::tags("total_debt", "USD", false, &[])
    });
    rules
}
