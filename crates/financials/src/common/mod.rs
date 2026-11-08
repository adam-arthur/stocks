mod cash;
mod debt;

use crate::Rule;

pub(crate) fn rules() -> Vec<Rule> {
    let mut rules = [
        (
            "revenue",
            "USD",
            true,
            &[
                "RevenueFromContractWithCustomerExcludingAssessedTax",
                "RevenueFromContractWithCustomerIncludingAssessedTax",
                "Revenues",
                "SalesRevenueNet",
            ][..],
        ),
        (
            "earnings",
            "USD",
            true,
            &["NetIncomeLoss", "ProfitLoss"][..],
        ),
        (
            "common_shareholder_earnings",
            "USD",
            true,
            &["NetIncomeLossAvailableToCommonStockholdersBasic"][..],
        ),
        (
            "diluted_earnings_per_share",
            "USD/shares",
            true,
            &["EarningsPerShareDiluted"][..],
        ),
        ("total_assets", "USD", false, &["Assets"][..]),
        (
            "shareholder_equity",
            "USD",
            false,
            &["StockholdersEquity"][..],
        ),
        (
            "preferred_stock_equity",
            "USD",
            false,
            &[
                "PreferredStockIncludingAdditionalPaidInCapitalNetOfDiscount",
                "PreferredStockIncludingAdditionalPaidInCapital",
            ][..],
        ),
        (
            "temporary_shareholder_equity",
            "USD",
            false,
            &["TemporaryEquityCarryingAmountAttributableToParent"][..],
        ),
        (
            "preferred_shares_outstanding",
            "shares",
            false,
            &["PreferredStockSharesOutstanding"][..],
        ),
        (
            "shares_outstanding",
            "shares",
            false,
            &[
                "dei:EntityCommonStockSharesOutstanding",
                "CommonStockSharesOutstanding",
            ][..],
        ),
    ]
    .into_iter()
    .map(|(metric, unit, duration, tags)| Rule::tags(metric, unit, duration, tags))
    .collect::<Vec<_>>();
    rules.extend(cash::rules());
    rules.extend(debt::rules());
    rules
}
