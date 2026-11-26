# Personal settings

Run this tool's `config init` once (for example, `fit-query-files config init`).
All three query tools share ~/.fit-fusion/config.yaml. Repeating init preserves
the existing file. Uncomment version: 1 and the parent sections of any settings
you enable. An entirely commented file uses built-in defaults.

Each explicit flag overrides its YAML key for this invocation. All query tools
read common.timezone. The built-in default is `Asia/Taipei` (臺北). Under tools.<executable>,
fit-query-files reads dir, fit-query-apple reads archive, and fit-query-garmin
reads env_file and cache_dir. YAML paths resolve beside config.yaml; CLI paths
resolve in the working directory. Dates, activity IDs and output destinations
are selected on the command line. For these examples, set `ride_date` to your
YYYY-MM-DD departure date and `activity_id` to a previously cached activity ID.

    fit-query-files --timezone UTC list --date "$ride_date" > activities.json
    fit-query-garmin read --id "$activity_id" --offline --json > activity.json 2> settings.log

Before data access, stderr lists each effective key, value and source. Explicit
flags are labeled cli, personal settings config, and built-in values cli (default).
With --json the first stderr line for a valid data command is a JSON summary.
Parse command results from stdout and check the exit code. Offline Garmin reads
use the verified cache and require no token.
