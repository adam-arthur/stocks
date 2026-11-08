# REIT metrics

Extracts REIT metrics from company filings and supplemental reports using company-specific rules.

The `ffo` metric typically represents NAREIT FFO. Core FFO and adjusted FFO can include additional company-specific adjustments and should not be silently substituted for NAREIT FFO. Missing observations indicate unavailable data, not zero.

## TODO

- Pull all GAAP metrics directly from company filings eventually: the SEC bulk company-facts download sometimes omits the latest quarters well past their report dates.
- Collect both adjusted and NAREIT FFO when available. Most existing FFO metrics use nareit
- Add AFFO.
- Audit existing scripts for which form of ffo they are pulling. Then recategorize as adjusted vs nareit
- Use bulk download instead of sec fetching?
- Store only total FFO?
- Report whether total FFO is attributable to common shareholders only or common shareholders plus OP unitholders, so per-share calculations can use the matching share count or share-plus-OP-unit count.
