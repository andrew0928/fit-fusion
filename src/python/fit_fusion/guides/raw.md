# Inspect wire frames

{launch_note} Supply one readable FIT file.

```sh
{cli} raw ride.fit
```

On exit 0 parse stdout one JSON line at a time. Use each frame's `kind` and
`offset`. For a data frame, resolve its local message number against the
most recent matching definition in the same FIT member. Preserve the original
order. To check exact preservation, decode each frame's `hex`, concatenate
the bytes and compare their SHA-256 with the original FIT.

On exit 1 discard the whole frame sequence, including any lines emitted
before failure, and inspect stderr. Exit 2 indicates invalid arguments.
The command reads the original file without modifying it.
