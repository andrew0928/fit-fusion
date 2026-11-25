# Read one ride

Get an ID from list. From the repository root, this example selects the first fixture:

    rides=src/rust/merger/tests/fixtures
    id=$(fit-query-files --dir "$rides" list --json | jq -er '.activities[0].id')
    fit-query-files --dir "$rides" read --id "$id" --json

Read is offline and read-only. activity contains the source, path, recorder and UTC session bounds. streams lists record fields, their FIT numbers, developer indexes, valid sample counts and first/last valid UTC sample times. Missing times are null. The record timestamp is the clock. If the contents change, the ID changes. On exit 3 check errors and list the directory again. Exit 2 indicates invalid syntax.

Use --text for a readable report. JSON includes activity_name and device_name as strings or null when unknown.
