# Stock Pipeline

Scans all tradable stocks, researches their income categories, and refreshes prices for categorized securities. Successful and failed categorization attempts are skipped for 365 days; prices refresh each run. Failed categorization refreshes preserve existing metadata.

Run from the project root, with Alpaca credentials configured in `.env` and Codex installed and signed in:

```bash
cargo run
```

The scan takes no arguments and saves data under `data/` relative to the working directory. Repeated runs update the same records and resume categorization where previous runs left off.
