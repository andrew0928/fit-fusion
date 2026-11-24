"""Exercise the offline Rust date workflow on one actual FIT or a pair from the same ride."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from zoneinfo import ZoneInfo

from compare_inspectors import ROOT, SRC, run_to
from verify_roundtrip import digest, lines, observations

# Scalar native record fields whose FIT units and invalid codes are unambiguous.
FIELDS = {"cadence": (4, 0x02, 255), "power": (7, 0x84, 65535), "temperature": (13, 0x01, 127)}
RESULTS = ROOT / "runtime/reports/device-e2e"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def inspect(path, scratch, selected):
    """Use the independent Python inspector CLI, including its CRC and compressed-clock checks."""
    run_to([sys.executable, "-m", "fit_fusion", "raw", str(path)], scratch)
    device = start = None
    records = {}
    for row in observations(lines(scratch)):
        if row[0] != "message":
            continue
        _, number, timestamp, fields = row
        native = {fid: raw for dev, fid, _, raw in fields if dev == -1}
        if number == 0 and native.get(0) == b"\x04" and 1 in native and 2 in native:
            device = {key: int.from_bytes(native[fid], "little")
                      for key, fid in (("manufacturer", 1), ("product", 2))}
        if number == 18 and 2 in native:
            require(start is None, "expected a single activity session")
            start = int.from_bytes(native[2], "little")
        if number == 20:
            require(timestamp is not None and timestamp not in records, "missing or duplicate record timestamp")
            value, other = None, []
            for field in fields:
                dev, fid, base, raw = field
                if selected and dev == -1 and fid == selected[0]:
                    require(base == selected[1] and len(raw) == (2 if base == 0x84 else 1),
                            "selected stream has nonstandard type or size")
                    value = int.from_bytes(raw, "little", signed=base == 1)
                    if value == selected[2]:
                        value = None
                else:
                    other.append(field)
            records[timestamp] = (value, other)
    require(device and start is not None and records, "missing activity file_id, session start or records")
    return device, start, records


def check_stream(name, order, primary, secondary, output, report):
    require(list(output) == list(primary), "output timestamps differ from primary")
    ranges = iter(report["stream_sources"]["record." + name])
    interval = next(ranges, None)
    counts = Counter()
    changed = 0
    for t, (primary_value, other) in primary.items():
        backup = secondary.get(t)
        values = {"primary": primary_value, "secondary": backup[0] if backup else None}
        source = next((role for role in order if values[role] is not None), None)
        expected = values.get(source)
        require(output[t] == (expected, other), f"record values differ at FIT second {t}")
        if interval and interval["end_fit_seconds"] < t:
            interval = next(ranges, None)
        require(interval and interval["start_fit_seconds"] <= t <= interval["end_fit_seconds"]
                and interval["source"] == source, f"wrong provenance at FIT second {t}")
        counts[source or "missing"] += 1
        changed += source == "secondary" and expected != primary_value
    require(changed, f"fixture has no observable {name} change from secondary; try another stream or priority")
    return dict(counts)


def run_case(args, run, sources, hashes):
    build = subprocess.check_output(["cargo", "build", "--locked", "--release", "--message-format=json",
                                     "--manifest-path", str(SRC / "rust/merger/Cargo.toml")], cwd=ROOT, text=True)
    cli = next(m["executable"] for m in map(json.loads, build.splitlines()) if m.get("executable"))
    config_path = run / "config.json"

    def command(*args):
        result = subprocess.run([cli, "--config", str(config_path), *args, "--json"],
                                cwd=ROOT, capture_output=True, text=True)
        if result.returncode:
            raise ValueError(f"{args[0]} exited {result.returncode}: {result.stdout.strip() or result.stderr.strip()}")
        return json.loads(result.stdout)

    with tempfile.TemporaryDirectory(dir=run) as tmp:
        scratch = Path(tmp) / "frames.jsonl"
        stream = args.stream or ("cadence" if args.unrelated else None)
        order = ["secondary", "primary"] if args.prefer == "secondary" else ["primary", "secondary"]
        selected = FIELDS.get(stream)
        originals = {role: inspect(path, scratch, selected) for role, path in sources.items()}
        device, fit_start, records = originals["primary"]
        day = datetime.fromtimestamp(fit_start + 631065600, timezone.utc).astimezone(ZoneInfo(args.timezone)).date()
        streams = {"record." + stream: order} if stream else {}
        config = {"version": 1, "timezone": args.timezone, "output_dir": str(run / "exports"), "sources": {},
                  "merge": {"primary_source": "primary", "min_overlap_ratio": 0.90, "streams": streams}}
        for role, path in sources.items():
            directory = run / role
            directory.mkdir()
            copy = directory / "activity.FIT"
            shutil.copyfile(path, copy)
            require(digest(copy) == hashes[role], f"{role} staging copy differs")
            config["sources"][role] = {"kind": "fit_directory", "directory": str(directory),
                                       "device": originals[role][0]}
        config_path.write_text(json.dumps(config, indent=2) + "\n")
        command("config", "check")
        for role in sources:
            activities = command("activities", "list", "--source", role)["activities"]
            require([a["id"] for a in activities] == [hashes[role]], f"{role} discovery differs from input")
        args_date = ("merge", "--date", str(day))
        ride, = command(*args_date)["rides"]
        artifacts = ride["artifacts"]
        fit = Path(artifacts["fit"])
        report = json.loads(Path(artifacts["report"]).read_text())
        require(ride["activity_id"] == hashes["primary"] and not ride["reused"], "wrong primary or stale result")
        expected_inputs = {"primary": hashes["primary"]} if args.unrelated else hashes
        require({s: i["sha256"] for s, i in report["inputs"].items()} == expected_inputs, "matched inputs differ")
        require(digest(fit) == report["output_sha256"], "output hash differs from merge report")
        out_device, out_start, output = inspect(fit, scratch, selected)
        require((out_device, out_start) == (device, fit_start), "output device or session start changed")
        if args.secondary:
            counts = check_stream(stream, order, records, originals["secondary"][2], output, report)
        else:
            require(digest(fit) == hashes["primary"], "primary-only merge changed FIT bytes")
            counts = {}
        retry, = command(*args_date)["rides"]
        require(retry["reused"] and retry["artifacts"] == artifacts and digest(fit) == report["output_sha256"],
                "rerun did not reuse identical output")
        return {"devices": {k: v[0] for k, v in originals.items()}, "ride_date": str(day),
                "records": len(records), "streams": streams, "selected_samples": counts,
                "artifacts": artifacts, "output_sha256": digest(fit), "reused": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("primary", type=Path)
    pair = parser.add_mutually_exclusive_group()
    pair.add_argument("--secondary", type=Path, help="confirmed recording of the same ride")
    pair.add_argument("--unrelated", type=Path, help="known different ride to exclude in date mode")
    parser.add_argument("--stream", choices=FIELDS, help="stream with a measurable secondary contribution")
    parser.add_argument("--prefer", choices=("primary", "secondary"), help="preferred stream source (default primary)")
    parser.add_argument("--timezone", default="Asia/Taipei")
    args = parser.parse_args()
    if bool(args.secondary) != bool(args.stream):
        parser.error("--secondary and --stream must be supplied together")
    if args.prefer and not args.secondary:
        parser.error("--prefer requires --secondary")
    sources = {"primary": args.primary.resolve()}
    second = args.secondary or args.unrelated
    if second:
        sources["secondary"] = second.resolve()
    RESULTS.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="run-", dir=RESULTS))
    case = "paired" if args.secondary else "unrelated" if args.unrelated else "primary_only"
    receipt = {"case": case, "status": "failed", "input_sha256": {}}
    try:
        hashes = receipt["input_sha256"]
        for role, source in sources.items():
            hashes[role] = digest(source)
        require(len(set(hashes.values())) == len(hashes), "the pair contains identical FIT bytes")
        receipt.update(run_case(args, run, sources, hashes))
        receipt["status"] = "passed"
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as err:
        receipt["error"] = str(err)
        print(err, file=sys.stderr)
    finally:
        for role, original in receipt["input_sha256"].items():
            try:
                require(digest(sources[role]) == original, f"{role} original changed during test")
            except (OSError, ValueError) as err:
                receipt.update(status="failed", error=str(err))
        output = run / "verification.json"
        output.write_text(json.dumps(receipt, indent=2) + "\n")
        print(output)
    return 0 if receipt["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
