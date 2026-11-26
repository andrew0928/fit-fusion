# FIT Sensor Data Fusion

## Goal
Version 0.1.0 delivers the two CLI families below. [M4](milestones/m4-cli.md) delivers independent tools starting at 0.2.0. The 0.2.x Homebrew packages are for testing. [M5](milestones/m5-release.md) owns the first production release at 0.3.0 with the legacy CLI retirement previewed in v0.2.4. All other acceptance work belongs to 0.2.x.

- The [inspector](spec/inspector.md) inventories a single FIT and compares
  independent decoding paths across Python, C#, Rust, and Node.js, with room
  for additional languages. See the [inspector cases](spec/testcases/inspector.md).
- Python, C#, Rust and Node.js inspectors also serialize decoded FIT data into a new file. Verify cross-language read-back before starting the M2 merger. See the [roundtrip acceptance](inspectors.md#四種檢查器的-fit-往返驗證).
- The [merger](spec/merger.md) has one Rust implementation for macOS using
  RustyFIT (Cargo package `rustyfit`). It merges simultaneous device
  recordings into one sensor timeline. See the [merger cases](spec/testcases/merger.md)
  and [production CLI decision](decisions/2026-11-21-production-fit-sdk.md).

This is stream merging, NOT concatenating routes or activities.

## Roadmap

| Milestone | Status as of 2026-11-26 | Delivery |
|---|---|---|
| M1 | Complete | Python, C#, Rust and Node.js inspectors validated FIT decoding and round trips. [Inspector verification](inspectors.md) |
| M2 | Released in 0.1.0 | The legacy Rust merger supports date-based matching, FIT export and optional Parquet. M2-4 was retired. Riders upload the FIT to Strava manually. [M2 plan](milestones/m2-cli.md) |
| M3 | Retired | Its requirements moved to named tasks in M4 and M5. [Task mapping](milestones/m4-cli.md#原-m3-工作移轉) |
| M4 | Closed in 0.2.2 | M4-1 through M4-5 delivered the five independent CLIs. M4-7 passed the R14 clean-install review. M4-6 and M4-8 were canceled. [M4 plan](milestones/m4-cli.md) |
| M5 | In progress for 0.3.0 | Source and binaries are public. The three query CLIs now share YAML settings and all five CLIs report effective arguments. The owner accepted M5-5, including configuration and terminal colors. v0.2.3 is public. M5-4 passed a public Homebrew upgrade from 0.2.2 with the existing personal configuration preserved. Further 0.2.x patches cover remaining acceptance. v0.2.4 previews the removal of the legacy source and executable ahead of 0.3.0. [M5 tasks and acceptance](milestones/m5-release.md) |

## Devices
- Bryton Rider 15 Neo: historical cycling FIT, manufacturer 267 and product 2004.
- Bryton Rider 460: GPS, barometric altitude, temperature,
  ANT+ chest heart rate monitor, cadence, speed
- Apple Watch Series 9: Apple Health / workout data
- Garmin CIRQA: activity and wellness data
- iPhone 16 Pro: HealthKit data source and export interface

## Requirements for the 0.1.0 fusion workflow

These source priorities and quality rules describe the legacy merger. The [0.2.x pair-merge contract](spec/m4-cli.md) uses explicit whole-stream replacement.
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
M2-4 was **retired** on 2026-11-25. Riders upload the exported FIT to Strava manually. [Scope decision](decisions/2026-11-25-cli-split.md). The [field-by-field specification](specification.md#strava-fit-逐欄選源表)
lists the source for each entry in Strava's public FIT table. Mapping a
different source field into a destination field is planned for a later release.
Config v1 supports local Garmin credentials. Manual Strava upload requires only the exported FIT.
Configure each data source independently and list the ordered sources for
each selected stream. A date-only merge discovers all rides that started on
that day in the primary source and matches secondary recordings per ride.
See the [v1 configuration contract and example](config/README.md).
The [M2 design](milestones/m2-cli.md) describes the commands, inputs,
outputs, recovery behavior, and end-to-end acceptance cases.

## Milestone 3 — retired

Its credential storage, distribution, integration and field-mapping tasks are assigned to specific M4 work items. The [v0.1.0 release decision](decisions/2026-11-24-v0.1.0-release.md) records the earlier scope, including M15 and M18 as the legacy acceptance cases and M16 and M17 as skipped.

## Milestone 4 — independent CLIs by purpose

0.2.0 starts with `fit-merge`: one local primary FIT, one overlay FIT, explicit stream names and a new output path, all supplied on the command line. Selected streams retain gaps in the overlay. Repeat the command for further inputs. `fit-inspector` owns sampling and missing-data analysis. The three independent query CLIs handle local FIT directories, Apple Health exports and Garmin Connect. New tools run independently of `fit-fusion`, whose source and executable were removed for the v0.2.4 pre-release. See the [development plan](milestones/m4-cli.md), [CLI contract](spec/m4-cli.md) and [acceptance cases](spec/testcases/m4-cli.md).

## Milestone 5 — 0.3.0 production release

`andrew0928/fit-fusion` is public. Each binary and checksum is published once in its GitHub Release. A release workflow updates the stable and versioned formulas in `andrew0928/homebrew-tools` with the asset URL and checksum. See the [distribution decision](decisions/2026-11-26-public-source-and-release.md).

M4-7 closed with the [R14 clean-install review](spec/testcases/m4-cli.md#r14review) of 0.2.2. M5-1 is complete. M5-2 was canceled. The maintainer still builds the binaries and publishes the release. The owner completed M5-5 review of the shared settings and their [usage guide](config/README.md). The [M5 tasks](milestones/m5-release.md) record the public 0.2.3 release, its verified upgrade and outstanding 0.2.x license reviews before the first production release in 0.3.0. Developer ID signing and notarization are skipped for this release.
