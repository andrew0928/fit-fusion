# Overwrite fields in a local FIT

Choose the primary file for its timeline. Choose an overlay containing the fields the user wants to replace. To inspect available field names first, use a FIT inspector. No config, account or network access is needed.

```sh
fit-merge --primary bike.fit --overlay band.fit --stream record.heart_rate --stream session.total_calories --out ride.fit --json
fit-merge --primary ride.fit --overlay power.fit --stream record.power --out ride-with-power.fit --json
```

Pass one `--stream` per native record field. The supported session fields are `session.total_calories` and `session.metabolic_calories`; each file must contain one overlapping session when selecting these fields. All other fields retain the primary values, including unselected summaries.

Record values align on exact FIT timestamps. Overlay gaps and invalid samples produce missing values even if the primary recorded a value. A selected field absent from the entire overlay fails the operation. Inspect the overlay and correct the stream name or select another overlay before retrying.

Give a new output filename in an existing directory. Exit 0 indicates success; with `--json`, read `artifacts.fit`. Exit 2 indicates a CLI syntax error; exit 3 indicates an input, merge or file error. Both originals and any existing destination are preserved. For a third recording, run the command again with the preceding output as primary.
