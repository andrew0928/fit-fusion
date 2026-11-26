# Select workouts

Supply your export ZIP. Start with the full list. For a filtered search, set `ride_date`, `from_date` or `until_date` to your YYYY-MM-DD dates and `source_name` to a source returned by list.

    fit-query-apple --archive export.zip list --json
    fit-query-apple --archive export.zip list --date "$ride_date" --text
    fit-query-apple --archive export.zip list --from "$from_date" --until "$until_date" --source-name "$source_name" --json

Use activities[].id for read or export. Source names come from the Workout, and may identify apps importing from other devices. Equal times or names can identify different recordings. The date is the departure day in the selected time zone. The built-in default is `Asia/Taipei` (臺北). The --until date is exclusive. An empty list is successful. Syntax errors exit 2. An unreadable ZIP or an ambiguous identity exits 3.
