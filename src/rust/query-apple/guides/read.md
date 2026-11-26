# Inspect one workout

Set `ride_date` to the desired departure date (YYYY-MM-DD) and `source_name` to the recording source identified by list.

    id=$(fit-query-apple --archive export.zip list --date "$ride_date" --source-name "$source_name" --json | jq -er '.activities[0].id')
    fit-query-apple --archive export.zip read --id "$id" --json

Inspect activity.source_name, device_name, streams[].sample_count and the UTC ranges. coverage.missing reports missing route or heart rate. Workout statistics, samples from the same source within the selected workout, and referenced GPX form the conversion. Read leaves the input unchanged and creates no FIT. Exit 3 reports a missing workout or conversion error.
