# Find rides

Replace `rides` with your FIT directory. List it first to discover the recordings. For the filtered examples, set `ride_date`, `from_date` and `until_date` to the desired YYYY-MM-DD dates and `device_id` to a listed recorder's numeric manufacturer/product pair.

    fit-query-files --dir rides list --json
    fit-query-files --dir rides list --date "$ride_date" --json
    fit-query-files --dir rides list --from "$from_date" --until "$until_date" --device "$device_id" --json

Search is recursive and read-only. Each activity has one cycling session. Date filters use the departure instant in the configured time zone. The built-in default is `Asia/Taipei` (臺北). Use --timezone to select another IANA zone. The period includes --from and excludes --until. Omit filters to list all rides.

Use activities[].id for read or export with the same --dir. This SHA-256 survives renaming and duplicate copies. source_id is the absolute directory. path is an absolute path to an original. An empty activities array with status ok is a successful query. Exit 2 means invalid arguments. Exit 3 includes errors with paths; partial results can contain usable activities.

To browse the results, use `--text` instead of `--json`. The device_name uses the FIT product name or model lookup. Generic activity FIT files have activity_name null. Use path to find the original.
