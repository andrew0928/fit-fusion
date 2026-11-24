# Configure sources

For the personal settings file, run `fit-fusion config init --json`.
The default path is `~/.config/fit-fusion/config.json`. To use a separate
file, pass `--config FILE` on every call. Both `--config` and `--json` are
global options and can appear before or after the subcommand.

```sh
fit-fusion --config rides.json config init --json
fit-fusion --config rides.json config check --json
```

Init returns `code: created` or `already_exists` and the path in
`artifacts.config`. Preserve existing credentials when editing. In the
generated template, set the bike directory and configure the Garmin source
with the rider's local credentials. Keep credential values out of transcripts.

For Garmin Connect, the generated `sources.health_band` uses
`kind: garmin_connect`. Set `auth.personal_api_key` for the connector to
obtain or renew its access token, or supply a valid `auth.access_token`.
It caches downloaded originals in `cache_dir`.

For an entirely local setup, this is a complete v1 config. Place bike FITs
in `inbox/bike` and matching band FITs in `inbox/band`. Paths are
relative to the config file. Use separate input and output directories.
The template lists three overrides. The primary FIT supplies all other
fields, including new sensor data. Omit `merge.streams` entirely to
preserve just the primary FIT.

```json
{
  "version": 1,
  "timezone": "Asia/Taipei",
  "output_dir": "exports",
  "sources": {
    "bike_meter": {"kind": "fit_directory", "directory": "inbox/bike"},
    "health_band": {
      "kind": "fit_directory",
      "directory": "inbox/band",
      "device": {"manufacturer": 1, "product": 5019}
    }
  },
  "merge": {
    "primary_source": "bike_meter",
    "min_overlap_ratio": 0.90,
    "streams": {
      "record.heart_rate": ["bike_meter", "health_band"],
      "session.total_calories": ["health_band"],
      "session.metabolic_calories": ["health_band"]
    }
  }
}
```

Run `config check` after editing. On exit 0 inspect `sources[].id`,
`primary_source` and `streams`. The latter lists only explicit overrides.
Check validates settings offline. To test
access to the actual files or account, follow `fit-fusion --skill sources`.
On `invalid_config` (exit 2), correct the location in `details.path` and retry.
