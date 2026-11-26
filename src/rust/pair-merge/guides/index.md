# fit-merge

Start with `fit-merge --skill preflight` to compare the primary and overlay with `fit-inspector`. It explains how to warn the rider about low coverage, frozen heart rate or conflicting ride totals before choosing streams.

Use `fit-merge --skill merge` to look up stream names for replacing a ride calorie total, handling missing or stale heart rate, or adding measured power. The guide includes commands and read-back checks. Use `fit-merge --help` for flag syntax. Guides are available offline.

Before merging, stderr lists the effective keys, values and CLI sources. --json sends the summary to stderr and the merge result to stdout.

Find the author’s contact details with `fit-merge --skill author`.
