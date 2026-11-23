# FIT Sensor Data Fusion

## Goal
Develop a Rust CLI for macOS using RustyFIT (Cargo package `rustyfit`) to merge
simultaneous activity recordings from multiple devices into one unified sensor-data
timeline. See the [production CLI decision](docs/decisions/2026-11-21-production-fit-sdk.md).

This is stream merging, NOT concatenating routes or activities.

## Devices
- Bryton Rider 460: GPS, barometric altitude, temperature,
  ANT+ chest heart rate monitor, cadence, speed
- Apple Watch Series 9: Apple Health / workout data
- Garmin CIRQA: activity and wellness data
- iPhone 16 Pro: HealthKit data source and export interface

## Requirements
1. Use Bryton FIT as primary activity and timeline.
2. Prefer heart rate recorded by Bryton (normally from the chest strap). Use
   Garmin CIRQA to fill small gaps. If the ride has prolonged missing or stale
   Bryton heart rate, select Garmin for the entire heart-rate stream after a
   quality check. Apple Watch heart rate is for analysis. See the
   [heart-rate decision](docs/decisions/2026-11-23-heart-rate-quality.md).
3. Merge the fields listed in the [specification](docs/specification.md).
   Prefer each field's primary source. Use its designated secondary source
   for a missing stream or individual gaps, and retain missing values when
   neither source has a valid observation.
4. Preserve original sampling rates and data provenance.
5. Support FIT developer fields where appropriate.
6. Never silently discard unknown sensor fields.
7. Export merged.fit for Strava.
8. Export merged.parquet for full-fidelity analytics.
9. Include a validation and merge report.
10. Never modify source files.
11. Detect discrepancies in ride calories and support correcting the published
    activity summary, preferring Garmin for comparable ride-level calories.
    Track active calories and total calories separately. Keep the original
    value and the reason for the correction in the report.

## First milestone
Investigate available Python FIT libraries.
Implement a CLI to inspect FIT files and report:
- Message types and fields
- Timestamps and recording intervals
- Sampling frequency and coverage
- Developer fields and unknown fields

Do not implement merging before inspecting real sample files.

## Milestone 1-1 — independent C# inspection

Add a read-only .NET CLI with the same `raw` (JSON Lines) and `inspect` (JSON)
commands as the Python inspector. Decode both original Bryton and Garmin
samples independently in each runtime. Compare every frame, field definition,
field value and inspection statistic. Verify that the raw frame bytes
reconstruct the original files. Keep the commands and comparison results
reproducible. The scope of this milestone is inspection.
