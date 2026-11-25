# Export original FIT
Use a Garmin activity ID for one file or a date/period for a batch:

    fit-query-garmin --env-file .env export --id 123456789 --out selected --json
    fit-query-garmin --env-file .env export --date 2026-11-21 --out selected --json
    fit-query-garmin export --offline --id 123456789 --out selected --json

A date query contacts Connect even when all originals are cached. Offline mode uses one verified cached ID. Use separate cache directories for different accounts. Each successful exports[] item includes path, sha256, format original, cache_hit and status exported or reused. The name is <GarminID>.fit. A different existing file is preserved and reported as conflict. Batch errors include their ID; after authorization, service or rate-limit failures, remaining IDs are marked not_attempted. Exit 2 means invalid syntax, 3 an unavailable file or conflict, 4 reauthorize, 5 honor retry_after, 6 service/protocol error. Retry with the same --out to reuse verified files.

Use --text for a readable report. JSON includes activity_name and device_name as strings or null when unknown.
