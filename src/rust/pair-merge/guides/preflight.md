# Compare two recordings before choosing streams

Use this optional guide when a rider asks you to merge two local FIT files. You can inspect both originals offline with `fit-inspector`. Reading this topic only prints instructions. Substitute the actual file paths below.

## Find comparable fields

```sh
fit-inspector inspect primary.fit
fit-inspector inspect overlay.fit
```

1. Read each JSON report from stdout. Check timestamp spans in `messages[].fields[].timing`. Use the raw session values below for sport (field 5), start time (2) and elapsed time (7, milliseconds) to verify the ride pairing. For record streams compare the common recording interval. Compare whole-session totals only when each file has one corresponding session and both cover similar start and end times.
2. In each member's `sampling[]`, match native rows (`developer_data_index` is null) by the full `name`, such as `record.heart_rate` or `session.total_calories`. Check units in `messages[].fields[]`. Review shared ride fields even when the rider only named one for replacement. Identify a requested field found on only one side separately. Zero valid samples or an absent field means the value is unavailable.
3. Compare `sample_count / message_count` and `missing_pct` for each side. Give the counts and denominators. These measure field presence in each file's messages. A slower recorder can have 100% coverage yet supply few matches on the primary timeline. For a proposed record overwrite, count valid overlay samples at exact primary timestamps and report gaps it would introduce.

## Check values on shared fields

```sh
fit-inspector raw primary.fit
fit-inspector raw overlay.fit
```

Parse stdout as JSON Lines. Pair each data frame's `values` with the latest definition for its `local` number in that FIT member. Use native field IDs, honor FIT invalid values and convert to the same units. Resolve timestamps before comparing records, including compressed timestamps when present. For frame details see `fit-inspector --skill raw`. If either inspection fails, discard its partial output and tell the rider which comparison is unavailable.

| Shared field | What to look for and report |
|---|---|
| `record.heart_rate` (record 20, field 3, bpm) | Flag substantially lower coverage, prolonged gaps, or one sensor repeating a value while the other changes. Compare at matching timestamps (field 253), in both directions. Give the interval, duration, fixed bpm and the other device's range. Split runs at pauses, missing samples and timestamp gaps. Treat 0 bpm as missing for this check. |
| `session.total_calories` (session 18, field 11, kcal) | Read the two actual totals even if both fields show 100% coverage. Report primary and overlay kcal, the absolute difference and, when meaningful, the ratio with its direction. A severalfold difference over the same ride deserves a warning. Compare total with total. `session.metabolic_calories` (field 196) is a separate component. |
| Other shared ride fields | Compare the same quantity over the common interval. Look for persistent disagreement or long dropouts in cadence, speed or measured power. Zero watts while coasting and steady values confirmed by both devices are plausible. |

For continuous 1 Hz heart rate, flag a constant run of at least 60 seconds for comparison. Stronger evidence of a stale reading requires at least 95% valid reference coverage, a reference P90−P10 spread of at least 5 bpm and a difference of at least 5 bpm for 20 consecutive seconds. On sparse or poorly aligned data, describe the observed plateau and uncertainty. These are advisory checks for the agent.

## Tell the rider what the merge would do

Report material findings in the rider's language. Name the field, the two sources, the measured difference and its consequence. Keep the requested field list explicit. Suggest an additional `--stream` when warranted and leave that change to the rider. If the overlapping data agrees, say so briefly. If there is too little overlap, state what you could verify.

- **Conflicting calorie totals.** Suppose the primary reports 6,186 kcal and the overlay reports 1,897 kcal for the same session. The primary is 4,289 kcal higher (3.26× the overlay). A heart-rate-only merge keeps 6,186 kcal. If the rider chooses the overlay total, suggest `--stream session.total_calories`. Select `--stream session.metabolic_calories` as well if the rider wants its metabolic component.
- **Low heart-rate coverage.** The primary has valid heart rate on 3,906 of 17,190 records (77.28% missing). The overlay has a valid sample at all 17,190 primary timestamps. A whole-stream switch would fill the gaps and also replace the 3,906 existing readings.
- **Suspected frozen heart rate.** The primary has 99.54% coverage but holds 102 bpm for 68 seconds while the overlay ranges from 102 to 112 bpm. During a separate 90-second plateau the two sources agree closely. Flag the first interval for review and check overlay coverage before suggesting a whole-stream replacement.

Use `fit-merge --skill merge` for the actual merge command once the desired fields are known.
