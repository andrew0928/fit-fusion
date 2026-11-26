# Garmin credentials

Put the Garmin Personal API key in the assignment GARMIN_PERSONAL_ACCESS_TOKEN in a dotenv file. Online queries use --env-file first, then tools.fit-query-garmin.env_file in ~/.fit-fusion/config.yaml, then .env in the working directory. The pre-execution summary identifies the selected path and its source. The file is read as data. Short-lived bearer tokens stay in memory.

    fit-query-garmin --env-file .env list --date 2026-11-21 --json

On exit 4 check the selected file and key expiry. To rotate the key, update the assignment. A verified offline original needs no credentials.
