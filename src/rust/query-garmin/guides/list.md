# List cycling activities
Provide a Personal API key (see --skill auth). Set `ride_date`, `from_date` and `until_date` to the desired local departure dates (YYYY-MM-DD). The built-in time zone is `Asia/Taipei` (臺北). Use --timezone for another IANA zone. --until is exclusive. Omit dates for the full account history. All pages are read and duplicate IDs removed.

    fit-query-garmin --env-file .env list --date "$ride_date" --json
    fit-query-garmin --env-file .env list --from "$from_date" --until "$until_date" --json

Select activities[].id (a decimal Garmin ID). An empty list is success. UTC timestamps come from Connect; activity_name is the Connect title, when supplied. A verified cached original can resolve device and device_name without a download. Unknown values are null. Listing remembers the observed title when the cache is writable. To filter by recorder, pass its numeric manufacturer/product pair to --device. This downloads candidate originals into --cache-dir. A failed page is an error. Exit 4 indicates credentials, 5 rate limit (honor error.retry_after), 6 an unavailable service or invalid response.

To browse the results, use `--text` instead of `--json`. Use the activity ID to distinguish rides with the same title.
