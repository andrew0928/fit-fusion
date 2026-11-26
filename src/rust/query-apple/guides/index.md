# Apple Health workouts
Use a local Apple Health export ZIP. Commands run offline.
- `fit-query-apple --skill list` — select cycling workouts by local departure day or exact recording source.
- `fit-query-apple --skill read` — inspect one workout's metadata, streams and route coverage.
- `fit-query-apple --skill export` — create or reuse a source-identified converted FIT per workout.

Default output is indented JSON. Choose --json for compact output or --text for people.

- `fit-query-apple --skill config` — initialize YAML, override a setting and read effective values and their sources on stderr.

Find the author’s contact details with `fit-query-apple --skill author`.
