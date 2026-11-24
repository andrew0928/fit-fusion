# Verify a FIT roundtrip

{launch_note} Supply a readable FIT and a fresh output path in an existing
writable directory. Preserve the original for the independent comparison.

```sh
{cli} roundtrip ride.fit --out copy.fit
```

On exit 0 parse one receipt. Check `semantic_equal`, `output`,
`input_sha256` and `output_sha256`. `byte_equal` reports exact byte identity.
Equivalent FIT encodings can use different bytes. For a cross-SDK check,
feed the output to another inspector's `raw` or `inspect` command.

Exit 1 can indicate invalid input, a read-back difference or a destination
that already exists. Inspect stderr. For a path collision, retry with a new
name. A semantic failure needs investigation before using the output.
Exit 2 indicates invalid arguments.
