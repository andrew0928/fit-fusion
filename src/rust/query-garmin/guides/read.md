# Read one ride
Set `activity_id` to an activities[].id returned by list.

    fit-query-garmin --env-file .env read --id "$activity_id" --json
    fit-query-garmin read --offline --id "$activity_id" --json

The first read downloads the original FIT to the local cache when necessary. The result contains activity metadata, exact FIT SHA-256, cache_hit, recording device and UTC bounds, plus streams with valid sample counts and UTC ranges. Cache location defaults to ~/Library/Caches/fit-query-garmin. Use the same --cache-dir for offline retries. Exit 3 can mean cache_miss or activity_not_found. For 4/5/6 inspect the error and follow --skill auth or list.

Use --text for a readable report. JSON includes activity_name and device_name as strings or null when unknown.
