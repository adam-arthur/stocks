use crate::Rule;

pub(super) fn rules() -> Vec<Rule> {
    vec![Rule {
        from: chrono::NaiveDate::from_ymd_opt(2025, 12, 31),
        ..Rule::tags("total_debt", "USD", false, &["NotesPayable"])
    }]
}
