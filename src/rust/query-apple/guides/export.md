# Export converted cycling FIT files

    fit-query-apple --archive export.zip export --date 2026-11-21 --source-name MyWatch --out selected/apple --json

For one entry use export --id WORKOUT_ID --out DIR. Each exports[] row gives the workout ID, source, path, format converted, SHA-256, coverage and status. Identical content is reused. When a newer ZIP supplies route or sample changes, the activity ID stays stable and the output hash selects a new filename. Never replace the earlier revision. Unavailable routes or heart rate are reported and retain gaps. A conflicting file or bad workout returns 3; inspect per-item results before retrying.
