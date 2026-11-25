# Find rides

Set rides to the directory holding the FIT files. For example, from the repository root:

    fit-query-files --dir src/rust/merger/tests/fixtures list --json
    fit-query-files --dir data/sources/bryton/fit list --date 2026-11-21 --json
    fit-query-files --dir data/sources/bryton/fit list --from 2026-11-01 --until 2026-12-01 --device 267/2205 --json

Search is recursive and read-only. Each activity has one cycling session. Date filters use the departure instant in Asia/Taipei by default; set --timezone to another IANA zone. The period includes --from and excludes --until. Omit filters to list all rides. Device IDs are manufacturer/product from the FIT.

Use activities[].id for read or export with the same --dir. This SHA-256 survives renaming and duplicate copies. source_id is the absolute directory. path is an absolute path to an original. An empty activities array with status ok is a successful query. Exit 2 means invalid arguments. Exit 3 includes errors with paths; partial results can contain usable activities.
