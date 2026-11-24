# Select streams

Use the existing policy unless the rider asks to change it. In config v1,
`merge.primary_source` supplies the timeline. Unlisted fields inherit the
primary FIT. Omit `merge.streams` or set it to `{}` to preserve the entire
primary FIT. Read the explicit overrides with the following command.

```sh
fit-fusion --config rides.json config check --json
```

To fill missing power samples, add the following property to `merge.streams`.
Create the object if absent and keep any existing properties.

```json
{"record.power": ["bike_meter", "health_band"]}
```

- Use configured source IDs in priority order. For a native record field,
  select the first valid value at that timestamp. Zero watts is valid.
  Leave the value missing when all candidates lack it. GPS latitude and
  longitude require matching priorities.
- The template selects bike meter heart rate with the band as backup. Short
  gaps can use the band. If whole-ride quality fails (including confirmed
  stale readings), the entire stream switches to a qualifying backup. Inspect `heart_rate` and
  `heart_rate_sources` in the report.
- The template selects the CIRQA band's `session.total_calories` and
  `session.metabolic_calories`. To override only total calories, keep just
  the first of these two rules. The report records the original values and
  derived active calories. A required
  but unavailable total can produce `quality_review`.
- A primary power meter is preserved automatically. Additional native
  record fields known to the FIT Profile can be selected by name.
  For an unprofiled native field, declare its verified numeric record ID in
  `merge.field_ids`. Cross-field renaming is outside config v1.

After changing the policy, run the date merge and inspect its new report.
For a single legacy FIT without a band, use a separate primary-only config
with `merge.streams` omitted when the intended task is preservation.
