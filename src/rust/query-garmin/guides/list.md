# List cycling activities
Provide a Personal API key (see --skill auth). Dates are local departure dates, default Asia/Taipei. Use --timezone for another IANA zone. --until is exclusive. Omit dates for the full account history. All pages are read and duplicate IDs removed.

    fit-query-garmin --env-file .env list --date 2026-11-21 --json
    fit-query-garmin --env-file .env list --from 2026-11-01 --until 2026-12-01 --json

Select activities[].id (a decimal Garmin ID). An empty list is success. UTC timestamps come from Connect; activity_name is the Connect title, when supplied. A verified cached original can resolve device and device_name without a download. Unknown values are null. Listing remembers the observed title when the cache is writable. Optional --device 1/5019 downloads candidate originals into --cache-dir to filter by manufacturer/product. A failed page is an error. Exit 4 indicates credentials, 5 rate limit (honor error.retry_after), 6 an unavailable service or invalid response.

To browse the results, use `--text` instead of `--json`. Use the activity ID to distinguish rides with the same title.
