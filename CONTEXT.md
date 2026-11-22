# FIT Sensor Data Fusion

## Goal
Develop a Python CLI to merge simultaneous activity recordings
from multiple devices into one unified sensor-data timeline.

This is stream merging, NOT concatenating routes or activities.

## Devices
- Bryton Rider 460: GPS, barometric altitude, temperature,
  ANT+ chest heart rate monitor, cadence, speed
- Apple Watch Series 9: Apple Health / workout data
- Garmin CIRQA: activity and wellness data
- iPhone 16 Pro: HealthKit data source and export interface

## Requirements
1. Use Bryton FIT as primary activity and timeline.
2. Preserve chest strap HR as the authoritative heart rate.
3. Merge additional sensor streams by timestamp.
4. Preserve original sampling rates and data provenance.
5. Support FIT developer fields where appropriate.
6. Never silently discard unknown sensor fields.
7. Export merged.fit for Strava.
8. Export merged.parquet for full-fidelity analytics.
9. Include a validation and merge report.
10. Never modify source files.

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
