# Select workouts

    fit-query-apple --archive export.zip list --date 2026-11-21 --text
    fit-query-apple --archive export.zip list --from 2026-11-01 --until 2026-12-01 --source-name AndrewWatch --json

Use activities[].id for read or export. Source names come from the Workout, and may identify apps importing from other devices. Equal times or names can identify different recordings. The date is the departure day in --timezone (default Asia/Taipei); --until is exclusive. An empty list is successful. Syntax errors exit 2. An unreadable ZIP or an ambiguous identity exits 3.
