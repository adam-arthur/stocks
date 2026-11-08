import argparse
import csv
import json
import math
import sys
from calendar import monthrange
from collections import Counter
from datetime import date, timedelta
from pathlib import Path
from statistics import fmean


def number(value):
    return isinstance(value, (int, float)) and math.isfinite(value)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--stocks", type=Path, default=Path("data/derived/stocks"))
    parser.add_argument("--financials", type=Path)
    parser.add_argument("--as-of", type=date.fromisoformat, default=date.today())
    parser.add_argument("--exclude", nargs="*", default=[])
    args = parser.parse_args()
    rows, counts = [], Counter()
    for path in sorted(args.stocks.glob("*/meta.json")):
        meta = json.loads(path.read_text())
        if meta["metadata"]["category"] != "EQUITY_REIT":
            continue
        counts["equity_reits"] += 1
        if path.parent.name in args.exclude:
            counts["explicitly_excluded"] += 1
            continue
        prices = json.loads((path.parent / "prices.json").read_text())
        dates = sorted(d for d in prices if d <= args.as_of.isoformat())
        if not dates or (args.as_of - date.fromisoformat(dates[-1])).days > 7:
            counts["missing_or_stale_price"] += 1
            continue
        current = prices[dates[-1]].get("dividend_yield")
        if not number(current) or current <= 0:
            counts["missing_or_zero_yield"] += 1
            continue
        financial_path = (args.financials or args.stocks) / path.parent.name / "financials.json"
        financials = [
            q for q in json.loads(financial_path.read_text())
            if q["end"] <= dates[-1]
        ] if financial_path.exists() else []
        latest = max(financials, key=lambda q: q["end"], default={})
        if not all(number(latest.get(k)) for k in (
            "total_debt", "cash_and_equivalents", "total_assets"
        )) or latest["total_assets"] <= 0:
            counts["missing_latest_balance_sheet"] += 1
            continue
        debt = (latest["total_debt"] - latest["cash_and_equivalents"]) / latest["total_assets"]
        if debt >= 0.45:
            counts["net_debt_at_least_45_percent"] += 1
            continue
        counts["leverage_pass"] += 1
        end = date.fromisoformat(dates[-1])
        start = end.replace(year=end.year - 5, day=min(end.day, monthrange(end.year - 5, end.month)[1]))
        history = [
            (d, prices[d]["dividend_yield"])
            for d in dates if start.isoformat() <= d < dates[-1]
            and number(prices[d].get("dividend_yield"))
            and prices[d]["dividend_yield"] >= 0
        ]
        if len(history) < 1200 or date.fromisoformat(history[0][0]) > start + timedelta(days=7):
            counts["insufficient_five_year_history"] += 1
            continue
        average = fmean(y for _, y in history)
        if average <= 0 or current <= average:
            counts["yield_not_above_average"] += 1
            continue
        rows.append({
            "symbol": path.parent.name,
            "name": meta["metadata"]["name"],
            "price_date": dates[-1],
            "yield_pct": current * 100,
            "five_year_average_yield_pct": average * 100,
            "above_average_pct": (current / average - 1) * 100,
            "yield_gap_percentage_points": (current - average) * 100,
            "net_debt_to_assets_pct": debt * 100,
            "balance_sheet_date": latest["end"],
            "history_days": len(history),
        })
    rows.sort(key=lambda r: r["above_average_pct"], reverse=True)
    if rows:
        writer = csv.DictWriter(sys.stdout, fieldnames=rows[0].keys())
        writer.writeheader()
        writer.writerows({k: round(v, 4) if isinstance(v, float) else v for k, v in r.items()} for r in rows)
    counts["matches"] = len(rows)
    print(dict(counts), file=sys.stderr)


if __name__ == "__main__":
    main()
