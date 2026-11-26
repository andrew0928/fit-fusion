# Export original FIT

Replace `rides` with your FIT directory and set `ride_date` to the desired departure date (YYYY-MM-DD).

    fit-query-files --dir rides export --date "$ride_date" --out selected --json

Use --id with a list result for one exact recording, or use the same date, period and device filters as list. With no filters all cycling activities are selected. The directory is created on the first copy. exports[] contains id, source_id, path, sha256, format (original) and status. Use the returned path. Source FIT files are opened read-only. An existing output subdirectory is excluded from this export's search.

Per-file status exported means created; reused means identical bytes already exist. A conflict preserves the destination. Choose a separate --out for conflicting names if both files are needed. Batch processing continues and reports each result. A retry reuses completed files. Exit 0 means success (including zero matches), 2 invalid arguments, 3 source or export errors. Check both exports and errors on exit 3.

Use --text for a readable report. JSON includes activity_name and device_name as strings or null when unknown.
