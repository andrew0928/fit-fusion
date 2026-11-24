# FIT inspector agent guide

{launch_note}

Choose a task and run the corresponding command.

- Identify streams and check sample coverage: `{cli} --skill inspect`
- Examine encoded fields or reconstruct original bytes: `{cli} --skill raw`
- Verify parsing and re-encoding into a fresh FIT: `{cli} --skill roundtrip`
{sdk_topic}
For each task capture stdout, stderr and the exit code separately.
For multi-device merging, use the separate `fit-fusion --skill index`.
