"""Compare both CLIs on fresh input and retain their results in reports/milestone-1-1."""

import argparse
import hashlib
import json
from itertools import zip_longest
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
PROJECT = ROOT / "csharp/FitFusion"
REPORTS = ROOT / "reports/milestone-1-1"


def run_to(command, output):
    with output.open("wb") as stream:
        subprocess.run(command, cwd=ROOT, stdout=stream, check=True)


def difference(left, right, path="$"):
    if left == right:
        return None
    if isinstance(left, dict) and isinstance(right, dict):
        for key in left.keys() | right.keys():
            if key not in left or key not in right:
                return f"{path}.{key} missing on one side"
            mismatch = difference(left[key], right[key], f"{path}.{key}")
            if mismatch:
                return mismatch
    if isinstance(left, list) and isinstance(right, list) and len(left) == len(right):
        for i, (a, b) in enumerate(zip(left, right)):
            mismatch = difference(a, b, f"{path}[{i}]")
            if mismatch:
                return mismatch
    return f"{path} differs"


def compare(path, dll):
    name = path.stem
    source = ROOT / path
    source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
    command = [sys.executable, "-m", "fit_fusion"]
    csharp = ["dotnet", str(dll)]
    with tempfile.TemporaryDirectory(dir=REPORTS) as directory:
        work = Path(directory)
        py_raw = work / "python.wire.jsonl"
        cs_raw = work / f"{name}.csharp.wire.jsonl"
        py_report = work / f"{name}.python.inspect.json"
        cs_report = work / f"{name}.csharp.inspect.json"
        run_to([*command, "raw", str(path)], py_raw)
        run_to([*csharp, "raw", str(path)], cs_raw)
        count = 0
        digest = hashlib.sha256()
        with py_raw.open() as a, cs_raw.open() as b:
            for count, (left, right) in enumerate(zip_longest(a, b), 1):
                if left is None or right is None:
                    raise ValueError(f"{name} raw frame count differs at {count}")
                expected, actual = json.loads(left), json.loads(right)
                mismatch = difference(expected, actual)
                if mismatch:
                    raise ValueError(f"{name} frame {count}, byte {expected['offset']}: {mismatch}")
                digest.update(bytes.fromhex(actual["hex"]))
        if digest.hexdigest() != source_hash:
            raise ValueError(f"{name} raw frames do not reproduce input SHA-256")
        run_to([*command, "inspect", "--decoder", "wire", str(path)], py_report)
        run_to([*csharp, "inspect", str(path)], cs_report)
        reference = json.loads(py_report.read_bytes())
        mismatch = difference(reference, json.loads(cs_report.read_bytes()))
        if mismatch:
            raise ValueError(f"{name} inspect: {mismatch}")
        if hashlib.sha256(source.read_bytes()).hexdigest() != source_hash:
            raise ValueError(f"{name} input changed during verification")
        for artifact in (cs_raw, py_report, cs_report):
            artifact.replace(REPORTS / artifact.name)
    messages = [m for member in reference["members"] for m in member["messages"]]
    return {"file": str(path), "sha256": source_hash, "raw_frames_compared": count,
            "data_messages": sum(m["message_count"] for m in messages),
            "field_summaries_compared": sum(len(m["fields"]) for m in messages),
            "raw_equal": True, "reconstructed_sha256_equal": True, "inspect_equal": True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", type=Path, nargs="+")
    args = parser.parse_args()
    paths = [p.resolve() for p in args.files]
    if len({p.stem for p in paths}) != len(paths):
        parser.error("input filenames must have distinct stems for report naming")
    REPORTS.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as build:
        subprocess.run(["dotnet", "build", str(PROJECT), "-o", build, "--nologo", "-v:q"], check=True)
        results = [compare(path.relative_to(ROOT) if ROOT in path.parents else path,
                           Path(build) / "FitFusion.dll") for path in paths]
    payload = json.dumps({"python_decoder": "wire", "csharp_decoder": "FitFusion",
                          "results": results}, indent=2) + "\n"
    (REPORTS / "comparison.json").write_text(payload)
    print(payload, end="")


if __name__ == "__main__":
    main()
