# Choose which FIT fields to replace

If you are helping a rider choose fields, follow `fit-merge --skill preflight` first to inspect the two inputs and surface significant differences.

Start with two recordings of the same ride. The primary FIT supplies the timeline and every field you leave out. Each `--stream` selects one same-name field from the overlay. At least one is required. Before merging, tell the rider which device will supply each selected field and which device's ride calorie total the output will carry.

## Choose which ride calorie total to keep

If the two recordings give substantially different totals for the same ride, show both values and identify their sources. Selecting `record.heart_rate` alone keeps the primary calorie total. Offer to replace calories separately if the rider only asked to change a sensor stream.

Use the file with the desired timeline as primary. To use the overlay's whole-ride total, select `session.total_calories`. To retain its metabolic component too, explicitly select `session.metabolic_calories`. Both files must contain one session each with overlapping time ranges. Check that they cover comparable portions of the ride before copying a whole-session total.

```sh
fit-merge --primary primary.fit --overlay overlay.fit --stream session.total_calories --stream session.metabolic_calories --out ride.fit --json
```

This keeps bike data such as cadence (`record.cadence`), distance (`record.distance`) and GPS (`record.position_lat`, `record.position_long`) wherever the primary recorded them.

For example, the primary reports 6,186 kcal and the overlay reports 1,897 kcal total, including 383 kcal at rest. With both fields selected, the output contains 1,897 kcal total and 383 kcal metabolic. In this example, **active** calories are 1,897 − 383 = 1,514 kcal. Check that the recording source defines the metabolic field as the resting component before using that calculation.

## Keep or replace heart rate

Omit `record.heart_rate` to keep the primary readings when correcting calories. If the signal drops out, a recorder can repeat its last BPM and still show 100% coverage. Run `fit-inspector inspect primary.fit --stream record.heart_rate` and repeat for `overlay.fit`. Check `members[].sampling[]` for missing counts and rates. Compare values at matching timestamps over the suspect interval before recommending a switch.

Use this command when the overlay has the better signal and the rider wants it for the whole ride.

```sh
fit-merge --primary primary.fit --overlay overlay.fit --stream record.heart_rate --out ride-with-hr.fit --json
```

This example keeps the primary calorie total. To switch heart rate **and** correct calories in one pass, add `--stream record.heart_rate` to the calorie command. Replacement uses exact primary timestamps. A missing or invalid overlay sample leaves a gap even when the primary had a value there. Existing lap and session heart-rate summaries retain their primary values.

## Add measured power

If the primary already contains the desired `record.power`, a calorie-only merge preserves it. To add measured watts from a separate recording, use that file as the overlay and the calorie-corrected ride above as primary.

```sh
fit-merge --primary ride.fit --overlay power.fit --stream record.power --out ride-with-power.fit --json
```

Zero watts is a valid sample. If the overlay entirely lacks a selected field, the merge fails. If it defines the field but every sample is invalid, the output contains missing values for that field.

## Verify the result

Use a new filename in an existing directory. On exit 0, `--json` returns the output path in `artifacts.fit`. For a calorie correction, use `fit-inspector raw ride.fit` and read the native fields in the session message (global ID 18, field 11 for total kcal and 196 for metabolic kcal). Match each data frame's values to its local definition as described in `fit-inspector --skill raw`. Compare each selected value with the overlay before reporting it. If a requested overlay value is missing, tell the rider which value is available before proposing a narrower merge.

For a record stream, inspect coverage on the output and compare samples at matching timestamps. Exit 2 indicates invalid CLI syntax. Exit 3 indicates an input, merge or output-file error. Read the diagnostic before retrying with a new destination. Both source files stay intact.
