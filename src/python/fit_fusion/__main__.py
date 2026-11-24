"""Command line entry point for FIT Fusion."""

import argparse
import json
from pathlib import Path
import platform
import sys

from . import __version__
from .inspect import inspect_file
from .wire import wire_frames


COMMANDS = ("inspect", "raw", "roundtrip")
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
    argv = sys.argv[1:]
    if argv in (["--version"], ["-V"]):
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
    if argv and argv[0] == "--skill":
        topic = argv[1] if len(argv) == 2 else "index"
        if len(argv) > 2 or topic not in ("index", *COMMANDS):
            print("Use --skill index for available topics (inspect, raw, roundtrip).", file=sys.stderr)
            return 2
        sys.stdout.write(guide(topic + ".md"))
        return 0
    if argv and argv[-1] in ("--help", "-h"):
        if len(argv) == 1:
            sys.stdout.write(guide("help.txt"))
            return 0
        if len(argv) == 2 and argv[0] in COMMANDS:
            sys.stdout.write(guide(argv[0] + ".txt"))
            return 0
    parser = argparse.ArgumentParser(prog="python3 -m fit_fusion")
    commands = parser.add_subparsers(dest="command", required=True)
    inspect = commands.add_parser("inspect", help="report FIT fields, coverage and timing as JSON")
    inspect.add_argument("file", type=Path, help="input FIT recording")
    inspect.add_argument("--decoder", choices=("auto", "fitdecode", "wire"), default="auto",
                         help="auto uses fitdecode when installed, otherwise the offline wire reader")
    raw = commands.add_parser("raw", help="validate CRCs and dump every FIT frame as JSON Lines")
    raw.add_argument("file", type=Path, help="input FIT recording")
    export = commands.add_parser("roundtrip", help="parse and serialize a verified new FIT offline")
    export.add_argument("file", type=Path)
    export.add_argument("--out", type=Path, required=True, help="new FIT path in an existing directory")
    args = parser.parse_args()
    try:
        if args.command == "raw":
            for frame in wire_frames(args.file.read_bytes()):
                print(json.dumps(frame, ensure_ascii=False, separators=(",", ":"), allow_nan=False))
            return 0
        if args.command == "roundtrip":
            from .roundtrip import roundtrip
            report = roundtrip(args.file, args.out)
        elif args.decoder == "wire":
            from .offline import inspect_wire
            report = inspect_wire(args.file)
        else:
            try:
                report = inspect_file(args.file)
            except ModuleNotFoundError as error:
                if args.decoder != "auto" or error.name != "fitdecode":
                    raise
                from .offline import inspect_wire
                print("fit-fusion: using offline FIT profile (fitdecode unavailable)", file=sys.stderr)
                report = inspect_wire(args.file)
    except ModuleNotFoundError as error:
        if error.name != "fitdecode":
            raise
        requirements = Path(__file__).resolve().parents[1] / "requirements.txt"
        parser.exit(1, f'fit-fusion: install the decoder with python -m pip install -r "{requirements}"\n')
    except Exception as error:
        # A failed decode must never produce a successful, partial inventory.
        parser.exit(1, f"fit-fusion: {args.file}: {error or type(error).__name__}\n")

    print(json.dumps(report, indent=2, allow_nan=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
