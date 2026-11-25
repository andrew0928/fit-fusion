# Inspect one workout

    id=$(fit-query-apple --archive export.zip list --date 2026-11-21 --source-name MyWatch --json | jq -er '.activities[0].id')
    fit-query-apple --archive export.zip read --id "$id" --json

Inspect activity.source_name, device_name, streams[].sample_count and the UTC ranges. coverage.missing reports missing route or heart rate. Workout statistics, samples from the same source within the selected workout, and referenced GPX form the conversion. Read leaves the input unchanged and creates no FIT. Exit 3 reports a missing workout or conversion error.
