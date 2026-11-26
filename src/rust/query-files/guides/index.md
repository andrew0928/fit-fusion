# fit-query-files

Use a local FIT directory. Commands run offline. Start with list for activity IDs.

- `fit-query-files --skill list` — search by departure date, period or recorder.
- `fit-query-files --skill read` — inspect one activity's metadata and available record streams.
- `fit-query-files --skill export` — copy original FIT files and verify or reuse the copies.

Default output is indented JSON. Use `--json` for compact JSON or `--text` for a human-readable report. Check status and the process exit code.

- `fit-query-files --skill config` — initialize YAML, override a setting and read effective values and their sources on stderr.

Find the author’s contact details with `fit-query-files --skill author`.
