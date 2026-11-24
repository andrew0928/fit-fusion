# Hand off a FIT for manual Strava upload

Start with the merge result and the requested ride. For a manual merge, use
`artifacts.fit`. For date mode, use the matching exported row in `rides[]`,
then its `artifacts.fit`. A partial day can contain successful rows.

1. Confirm the row has `status: exported`, the FIT exists, and its SHA-256
   matches `output_sha256` in that row's merge report. For example:

```sh
shasum -a 256 merged-ride/merged.fit
```

2. Give the rider the FIT path for Strava's file upload page. Keep the report
   and optional Parquet available for auditing. If the ride already synced
   from the bike computer, ask the rider to check the existing activity.
3. Report the handoff as exported and ready for manual upload. Confirmation
   that Strava accepted the activity comes from the rider or platform.

Strava API publishing is pending. This workflow requires no Strava token.
