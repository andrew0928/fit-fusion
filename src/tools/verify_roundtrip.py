"""Compare four independent FIT inventories and verify each encoder with another reader."""
import argparse
import hashlib
from itertools import zip_longest
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from compare_inspectors import PROJECT, ROOT, SRC, difference, run_to

MANIFEST = SRC / "rust/inspector/Cargo.toml"
RESULTS = ROOT / "runtime/reports/inspector-roundtrip"
WIDTHS = {3: 2, 4: 2, 5: 4, 6: 4, 8: 4, 9: 8, 11: 2, 12: 4, 14: 8, 15: 8, 16: 8}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def lines(path):
    with path.open() as handle:
        for line in handle:
            yield json.loads(line)


def observations(frames):
    """Canonical wire values. Keep member boundaries, field identity, invalid values and arrays."""
    for frame in frames:
        kind = frame["kind"]
        if kind == "header":
            layouts, developers, clock = {}, {}, None
            yield ("member", frame["protocol"], frame["profile"])
        elif kind == "crc":
            yield ("end",)
        elif kind == "definition":
            layouts[frame["local"]] = frame["endian"], frame["fields"]
        else:
            endian, fields = layouts[frame["local"]]
            values, native, start = [], {}, 1
            data = bytes.fromhex(frame["hex"])
            timestamp = None
            for field, value in zip(fields, frame["values"]):
                fid, dev = field["id"], field.get("developer_index", -1)
                base = field.get("base", developers.get((dev, fid)))
                raw = data[start:start + field["size"]]
                start += len(raw)
                if dev == -1:
                    native[fid] = value
                if dev == -1 and fid == 253 and base == 0x86 and len(raw) == 4 and value != 0xffffffff:
                    timestamp = value
                    continue
                code = base & 31 if base is not None else None
                width = WIDTHS.get(code, 1)
                if code == 7:
                    raw = raw.rstrip(b"\0")
                elif endian == ">" and len(raw) % width == 0:
                    raw = b"".join(raw[i:i+width][::-1] for i in range(0, len(raw), width))
                values.append((dev, fid, base, raw))
            if "timestamp_offset" in frame and clock is not None:
                timestamp = clock + (frame["timestamp_offset"] - clock) % 32
            if timestamp is not None:
                clock = timestamp
            if frame["global_id"] == 206 and all(isinstance(native.get(i), int) for i in (0, 1, 2)):
                developers[native[0], native[1]] = native[2]
            yield ("message", frame["global_id"], timestamp, sorted(values, key=lambda v: v[:2]))


def compare_file(source, directory, work, commands):
    directory.mkdir()
    source_hash = digest(source)
    baseline = work / "reference.raw.jsonl"
    run_to([*commands["python"], "raw", str(source)], baseline)
    reference = work / "reference.inspect.json"
    run_to([*commands["python"], "inspect", "--decoder", "wire", str(source)], reference)
    inventory = json.loads(reference.read_text())
    count = 0
    recovered = hashlib.sha256()
    for count, frame in enumerate(lines(baseline), 1):
        recovered.update(bytes.fromhex(frame["hex"]))
    if recovered.hexdigest() != source_hash:
        raise ValueError("raw frames did not reproduce input")
    scratch = work / "actual.jsonl"
    for language in ("csharp", "rust", "node"):
        run_to([*commands[language], "raw", str(source)], scratch)
        for n, (p, q) in enumerate(zip_longest(lines(baseline), lines(scratch)), 1):
            mismatch = difference(p, q)
            if mismatch:
                raise ValueError(f"{language} raw frame {n}: {mismatch}")
        modes = [["--decoder", "wire"], ["--decoder", "rustyfit"]] if language == "rust" else [[]]
        for mode in modes:
            run_to([*commands[language], "inspect", str(source), *mode], scratch)
            mismatch = difference(inventory, json.loads(scratch.read_text()))
            if mismatch:
                raise ValueError(f"{language} inspect {mode}: {mismatch}")

    run_to([*commands["node"], "sdk-check", str(source)], scratch)
    sdk = json.loads(scratch.read_text())
    receipts = []
    for language, command in commands.items():
        output = directory / f"{language}.fit"
        run_to([*command, "roundtrip", str(source), "--out", str(output)], scratch)
        receipt = json.loads(scratch.read_text())
        reader = "csharp" if language == "python" else "python"
        run_to([*commands[reader], "raw", str(output)], scratch)
        for n, (a,b) in enumerate(zip_longest(observations(lines(baseline)), observations(lines(scratch))), 1):
            if a != b:
                raise ValueError(f"{language} export, independent {reader} read-back: item {n} differs")
        if (source_hash != receipt["input_sha256"] or digest(source) != source_hash
                or digest(output) != receipt["output_sha256"]):
            raise ValueError(f"{language}: hash verification failed")
        receipts.append({**receipt, "verified_by": reader})
    return {"source": str(source), "sha256": source_hash, "frames": count,
            "raw_equal": True, "inspect_equal": True, "exports": receipts, "node_sdk": sdk}


def build_commands(work):
    subprocess.run(["dotnet","build",str(PROJECT),"-o",str(work/"dotnet"),"--nologo","-v:q"], check=True)
    build = subprocess.check_output(["cargo","build","--locked","--release","--manifest-path",str(MANIFEST),
                                     "--message-format=json"], cwd=ROOT, text=True)
    binary = next(m["executable"] for m in map(json.loads,build.splitlines()) if m.get("executable"))
    return {"python": [sys.executable, "-m", "fit_fusion"], "csharp": ["dotnet", str(work/"dotnet/FitFusion.dll")],
                "node": ["node", str(SRC/"node/inspector/cli.mjs")], "rust": [binary]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path, help="FIT sources; default uses a generated control")
    args = parser.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="run-", dir=RESULTS))
    inputs = [p.resolve() for p in args.files]
    if not inputs:
        sys.path.insert(0, str(SRC / "python"))
        sys.path.insert(0, str(SRC / "python/tests"))
        from test_inspect import recording
        control = run / "control.fit"
        control.write_bytes(recording())
        inputs = [control]
    with tempfile.TemporaryDirectory(dir=run) as tmp:
        work = Path(tmp)
        commands = build_commands(work)
        results = [compare_file(p, run/f"{i}-{p.stem}", work, commands) for i,p in enumerate(inputs,1)]
    result = run/"comparison.json"
    result.write_text(json.dumps({"node_version":subprocess.check_output(["node","--version"],text=True).strip(),
                                  "inspect_backends":{"node":"wire + separate Garmin SDK capability report","python":"wire","csharp":"WireReader","rust":["wire","RustyFIT 0.11.2 + wire definitions"]},
                                  "results":results}, indent=2)+"\n")
    print(result)


if __name__ == "__main__":
    main()
