use serde_json::{Value, json};

pub fn merge(quarters: &mut Vec<Value>, reit: Vec<Value>) {
    let latest = quarters
        .iter()
        .filter_map(|quarter| quarter["end"].as_str())
        .max()
        .map(str::to_owned);
    for quarter in reit.into_iter().filter(|quarter| {
        quarter["end"]
            .as_str()
            .zip(latest.as_deref())
            .is_some_and(|(end, latest)| end <= latest)
    }) {
        let per_share = quarter["ffo_per_share"].as_f64().or_else(|| {
            let shares = quarter["ownership"]["shares_outstanding"]
                .as_f64()
                .filter(|shares| *shares > 0.0)?;
            let units = quarter["ownership"]["outside_op_unit_share_equivalents"]
                .as_f64()
                .unwrap_or(0.0);
            (units >= 0.0)
                .then(|| {
                    quarter["ffo_total"]
                        .as_f64()
                        .map(|ffo| ffo / (shares + units))
                })
                .flatten()
                .filter(|value| value.is_finite())
        });
        let index = quarters
            .iter()
            .position(|record| {
                record["start"] == quarter["start"] && record["end"] == quarter["end"]
            })
            .unwrap_or_else(|| {
                quarters.push(json!({"start": quarter["start"], "end": quarter["end"]}));
                quarters.len() - 1
            });
        quarters[index]["ffo_total"] = quarter["ffo_total"].clone();
        quarters[index]["ffo_per_share"] = json!(per_share);
    }
    quarters.sort_by(|a, b| a["end"].as_str().cmp(&b["end"].as_str()));
}
