# Discover recordings

`activities` by itself displays the list syntax and examples.
Use `config check` to get the configured source IDs. Substitute those IDs
in the list command. The examples use the default role names.

```sh
fit-fusion --config rides.json config check --json
fit-fusion --config rides.json activities list --source bike_meter --json
fit-fusion --config rides.json activities list --source health_band --json
```

On exit 0 read `activities[]`. Each row has `id`, `source_id`, `start_utc`
and `end_utc` (Unix seconds). An empty array is a successful empty search.

- A `fit_directory` scans cycling FITs recursively. The ID is the SHA-256 of
  the original bytes. Read `device.name` and `device.name_source` per activity.
  A role can contain several recorder models. A malformed FIT can fail the
  whole listing. Inspect the failing file and use a separate directory of
  validated copies to continue.
- A `garmin_connect` source queries the account. The row ID is the Garmin
  activity ID. List results have `device: null`. Merging downloads or reuses
  the original FIT and checks the recorder. Credentials belong in that
  source's `auth` settings.
- `apple_health_export` is reserved for analysis. The M2 merger reads FITs
  from local directories or Garmin Connect.

For date pairing, use the departure day in the config timezone. Run
`fit-fusion --skill merge` for the command and `fit-fusion --skill results`
for missing directories, authorization failures or ambiguous candidates.
