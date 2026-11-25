# Authenticate
Use your Garmin Connect Personal API key. Supply --env-file explicitly, set GARMIN_PERSONAL_ACCESS_TOKEN in the process environment, or import the key into the local macOS Keychain. Lookup follows that order. The dotenv reader reads the named assignment; its contents are never executed. Do not put the key in a CLI argument.

    fit-query-garmin --env-file .env list --date 2026-11-21 --json
    fit-query-garmin --env-file .env auth import --json
    fit-query-garmin auth status --json

Import creates or replaces the generic password for service fit-query-garmin and account personal-api-key. Status checks the Keychain entry, even when an environment key is present. Both use the default user keychain, or an existing keychain selected by --keychain FILE. A locked/inaccessible keychain needs unlocking or access approval in macOS. Reimport a renewed key for rotation or recovery. Import leaves the original credential file untouched. Only these explicit imports write to Keychain. Short-lived bearer tokens are held in memory. With exit 4 check the error code, unlock or renew as directed. A verified cache hit by ID needs no key.
