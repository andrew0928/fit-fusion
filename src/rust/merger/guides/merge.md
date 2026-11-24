# Merge a ride

Choose one input mode. Substitute the rider's file paths, departure date
or confirmed activity ID. Pass `--json` to capture one JSON object on stdout and
capture stderr and the exit status separately.

## Date with configured sources

```sh
fit-fusion --config rides.json merge --date 2026-11-21 --json
```

Supply a checked config and accessible sources. The day uses its timezone.
Each primary ride gets its own result. A companion must satisfy the configured
overlap in both directions. Read `rides[].artifacts.fit` for exported rides.

## Two local recordings of the same ride

```sh
fit-fusion merge bike.fit band.fit --out merged-ride --json
```

This form uses embedded defaults when no config is supplied. It works offline.
For custom priorities, pass `--config rides.json`. Explicit
files must map uniquely to roles. If a config uses two unfiltered directories,
use date mode or add device filters. Read `artifacts.fit` and `artifacts.report`.

## One local primary with configured sources

```sh
fit-fusion --config rides.json merge bike.fit --auto-match --out auto-ride --json
```

`--auto-match` takes no value. The supplied FIT represents `merge.primary_source`.
For each other source ID referenced by `merge.streams`, the CLI looks for the
same ride using the configured time-overlap threshold. The source's `kind`
determines whether to search a local FIT directory or Garmin Connect.

## One local primary with a known Garmin activity

```sh
fit-fusion --config rides.json merge bike.fit --garmin-id 12345 --out chosen-ride --json
```

Use this mode when you have a confirmed Garmin activity ID. Replace 12345 with the
candidate's `activity_id` from the ambiguity diagnostic. Explicit ID mode needs
one selected Garmin source with a Garmin device filter. Connect uses a platform
ID. A configured local Garmin directory uses the FIT's SHA-256.

For any mode, add `--parquet` to request the source and selection audit file.
Manual mode requires `--out`. Date mode uses `output_dir` from config.
Use `fit-fusion --skill results` to interpret success, partial results and retries.
