# Inspect a recording

{launch_note}
Supply one readable FIT file. {decoder_note}

```sh
{cli} inspect ride.fit
```

On exit 0 parse one JSON object from stdout. For each entry in `members[]`,
find the `messages[]` entry with `number: 20` (record). Its `message_count`
is the number of recorded samples. Inspect each entry in `fields[]` for
`number`, `name`, `sample_count`, `coverage_pct` and `timing`. Use
`developer_data_index` to distinguish native fields from developer fields.

A field with zero valid samples was defined in the file. An absent field
has no entry there. Coverage measures the presence of values. Detecting a
stale heart rate requires inspecting the values over time. On exit 1 read
stderr for the input or decoding error. Exit 2 indicates invalid arguments.

Use repeated `--stream MESSAGE.FIELD` to select coverage rows, for example
`{cli} inspect ride.fit --stream record.heart_rate --stream record.power`.
Without selectors, `members[].sampling[]` lists every actual field. Each row gives
message_count, sample_count, missing_count, missing_pct and state (complete,
partial, absent, no_valid_samples or no_messages). Zero messages gives null
missing_pct. Valid zero counts as a sample. The original inventory and timing
remain available in messages[].
