# Interpret the merge result

Use the exit code and the JSON from the same invocation. Diagnostics also
appear on stderr. Date output has a batch status and per-ride results.

| Mode and status | Action |
|---|---|
| Manual, `exported` | Read `artifacts.fit` and `artifacts.report`. |
| Date, `exported` | For each exported entry in `rides[]`, read its `artifacts.fit` and `artifacts.report`. |
| Date, `no_activities` | Exit 0 with an empty `rides` array. Report zero rides for that day. |
| Date, `partial` | The process exits nonzero. Keep successful artifacts and read each failed ride's `code`, `message` and `details`. |
| `error` | Inspect individual rides when present. A config or discovery failure can return a top-level error before any ride is available. |

The optional analysis path is `artifacts.parquet` on each exported result.
Open the merge report to check `inputs`, `primary_timeline`, `heart_rate`,
`stream_sources`, `calories` and `output_sha256`. To read the exported FIT
independently, pass its actual path to the inspector.

```sh
fit-inspector inspect merged-ride/merged.fit
```

## Recovery

- Exit 2 means usage or config. For `invalid_config`, inspect `details.path`.
- Exit 3 requires action on inputs or output. For `ambiguous_match`, inspect
  `details.candidates[]`, compare times and IDs, then choose a confirmed
  companion using the **merge** guide. For `quality_review`, inspect the
  reason and coverage, obtain usable data or send the ride for review.
- On `output_conflict`, preserve existing artifacts and choose a fresh manual
  destination. Identical valid manual output is reusable. Date mode uses
  revisions. An identical retry can return `reused: true`.
- Garmin exit 4 requires valid credentials. On exit 5, honor
  `details.retry_after` if present before retrying. On exit 6 inspect the
  Garmin response or connection before retrying.

Read `fit-fusion --skill publish` before handing an exported FIT to the rider.
