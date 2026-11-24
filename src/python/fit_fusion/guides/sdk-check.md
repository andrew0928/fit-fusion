# Check Garmin JavaScript SDK compatibility

{launch_note} Install the locked SDK with
`npm ci --prefix src/node/inspector` before the first SDK check.

```sh
{cli} sdk-check ride.fit
```

Exit 0 means a comparison report was produced. To judge support, read each
entry in `members[]`. Check `native_values_equal` for decoding and
`encode_field_values_equal` for re-encoding. On a mismatch, capture
`first_native_difference`, `first_encode_difference`, `decode_error` or
`encode_error`. SDK capability gaps can appear in a successful JSON report.

For an independent output FIT, use the wire roundtrip topic. Exit 1 reports
an input, dependency or runtime failure on stderr. Exit 2 indicates invalid
arguments.
