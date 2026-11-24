# FIT Sensor Data Fusion

## Goal
Maintain two CLI families with independent acceptance criteria.

- The [inspector](spec/inspector.md) inventories a single FIT and compares
  independent decoding paths across Python, C#, Rust, and Node.js, with room
  for additional languages. See the [inspector cases](spec/testcases/inspector.md).
- Python, C# and Rust inspectors also serialize decoded FIT data into a new file. Verify cross-language read-back before starting the M2 merger. See the [roundtrip acceptance](inspectors.md#三種檢查器的-fit-往返驗證).
- The [merger](spec/merger.md) has one Rust implementation for macOS using
  RustyFIT (Cargo package `rustyfit`). It merges simultaneous device
  recordings into one sensor timeline. See the [merger cases](spec/testcases/merger.md)
  and [production CLI decision](decisions/2026-11-21-production-fit-sdk.md).

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
   [heart-rate decision](decisions/2026-11-23-heart-rate-quality.md).
3. Preserve the primary FIT by default. Configured streams override its fields using the [selection specification](specification.md).
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
    activity summary. For a comparable ride, publish CIRQA's original
    `session.total_calories`. Preserve Garmin's original total and metabolic
    calorie values in the merged FIT. Track derived active calories separately
    and keep the Bryton value and the correction reason in the report.

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

## Milestone 2 — usable macOS CLI

Deliver a Rust CLI with human-readable `--help` and self-contained agent
instructions via `--skill <topic>`. Use the preset source and stream priorities
to select same-name fields and export a validated FIT for manual upload to Strava.
Optional `merge --parquet` preserves the complete input files and selected values.
Automated Strava publishing (M2-4) is deferred by the [scope decision](decisions/2026-11-24-manual-strava-export.md). The [field-by-field specification](specification.md#strava-fit-逐欄選源表)
lists the source for each entry in Strava's public FIT table. Mapping a
different source field into a destination field is planned for a later release.
Keep personal Garmin and Strava credentials together in a local config file.
Configure each data source independently and list the ordered sources for
each selected stream. A date-only merge discovers all rides that started on
that day in the primary source and matches secondary recordings per ride.
See the [v1 configuration contract and example](config/README.md).
The [M2 design](milestones/m2-cli.md) describes the commands, inputs,
outputs, recovery behavior, and end-to-end acceptance cases.

## Milestone 3 — credential storage, distribution, and other integrations

Research macOS Keychain and hardware-backed key protection, formal release
channels including Homebrew and GitHub, and further integrations. The
[M3 backlog](milestones/m2-cli.md#m3-需求與研究待辦) records the questions
and proposed proofs of concept.
