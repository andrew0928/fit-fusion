# fit-fusion agent guide

Choose the row that matches the task and run its command.
Examples assume the binaries are on PATH. If you launched a binary by
path, use the same path for subsequent commands.

| Task | Read |
|---|---|
| First run, offline setup or invalid settings | `fit-fusion --skill config` |
| Find rides, source IDs, recorder models or companion IDs | `fit-fusion --skill sources` |
| Choose streams and priorities, including heart rate and calories | `fit-fusion --skill selection` |
| Export by date, from two FITs, or from one FIT and configured sources | `fit-fusion --skill merge` |
| Locate artifacts, handle an empty/partial day or retry a failure | `fit-fusion --skill results` |
| Hand a finished FIT to the rider for Strava | `fit-fusion --skill publish` |

For two local FITs, start with **merge**. For a date, start with **config**,
then **sources** and **merge**. Read **results** before reporting success.
For a single FIT field inventory, use the separate `fit-inspector --skill index`.
