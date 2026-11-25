"""Command line entry point for FIT Fusion."""

import json
from pathlib import Path
import platform
import sys

from . import __version__
from .inspect import inspect_file
from .wire import wire_frames


IDENTITY = f"fit-inspector {__version__} (Python)"


def guide(file):
    text = (Path(__file__).with_name("guides") / file).read_text(encoding="utf-8")
    return (text.replace("{cli}", "PYTHONPATH=src/python python3 -m fit_fusion")
            .replace("{identity}", IDENTITY)
            .replace("{launch_note}", "Run the Python inspector from the project root.")
            .replace("{decoder_args}", " [--decoder auto|fitdecode|wire]")
            .replace("{decoder_note}", "Auto uses fitdecode if installed. Use --decoder wire for the offline reader.")
            .replace("{sdk_row}", "").replace("{sdk_topic}", ""))


def main():
    req = json.loads(sys.argv[1])
    if req["action"] == "version":
        from importlib import metadata
        try:
            fitdecode = metadata.version("fitdecode")
        except metadata.PackageNotFoundError:
            fitdecode = "not installed"
        print(IDENTITY)
        print(f"Runtime   {platform.python_implementation()} {platform.python_version()}")
        print(f"Platform  {platform.system()} {platform.machine()}")
        print(f"FIT       Wire reader, fitdecode ({fitdecode})")
        return 0
    if req["action"] == "skill":
        sys.stdout.write(guide(req["topic"] + ".md"))
        return 0
    file = Path(req["file"])
    try:
        if req["action"] == "raw":
            for frame in wire_frames(file.read_bytes()):
                print(json.dumps(frame, ensure_ascii=False, separators=(",", ":"), allow_nan=False))
            return 0
        if req["action"] == "roundtrip":
            from .roundtrip import roundtrip
            report = roundtrip(file, Path(req["out"]))
        elif req["decoder"] == "wire":
            from .offline import inspect_wire
            report = inspect_wire(file)
        else:
            try:
                report = inspect_file(file)
            except ModuleNotFoundError as error:
                if req["decoder"] != "auto" or error.name != "fitdecode":
                    raise
                from .offline import inspect_wire
                print("fit-fusion: using offline FIT profile (fitdecode unavailable)", file=sys.stderr)
                report = inspect_wire(file)
    except ModuleNotFoundError as error:
        if error.name != "fitdecode":
            raise
        requirements = Path(__file__).resolve().parents[1] / "requirements.txt"
        sys.exit(f'fit-fusion: install the decoder with python -m pip install -r "{requirements}"\n')
    except Exception as error:
        # A failed decode must never produce a successful, partial inventory.
        sys.exit(f"fit-fusion: {file}: {error or type(error).__name__}\n")

    print(json.dumps(report, indent=2, allow_nan=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
