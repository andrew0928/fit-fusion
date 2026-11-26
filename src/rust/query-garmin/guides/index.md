# fit-query-garmin
Query one personal Garmin Connect account. For online commands, use --env-file to select your credentials file or see the config guide for the settings lookup order.
- `fit-query-garmin --skill list` — discover cycling activities and their Garmin IDs.
- `fit-query-garmin --skill read` — inspect one original FIT and its record streams.
- `fit-query-garmin --skill export` — copy original FIT files by ID, date or period.
- `fit-query-garmin --skill auth` — provide or rotate the Personal API key.

Default output is indented JSON. Use --json for compact JSON or --text for a human-readable report. Check the exit code. Help and guides work offline.

- `fit-query-garmin --skill config` — initialize YAML, override a setting and read effective values and their sources on stderr.

Find the author’s contact details with `fit-query-garmin --skill author`.
