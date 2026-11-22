"""Command line entry point for FIT Fusion."""

import argparse
import json
from pathlib import Path
import sys

from .inspect import inspect_file
from .wire import wire_frames


def main():
    parser = argparse.ArgumentParser(prog="fit-fusion")
    commands = parser.add_subparsers(dest="command", required=True)
    inspect = commands.add_parser("inspect", help="report FIT fields, coverage and timing as JSON")
    inspect.add_argument("file", type=Path, help="input FIT recording")
    inspect.add_argument("--decoder", choices=("auto", "fitdecode", "wire"), default="auto",
                         help="auto uses fitdecode when installed, otherwise the offline wire reader")
    raw = commands.add_parser("raw", help="validate CRCs and dump every FIT frame as JSON Lines")
    raw.add_argument("file", type=Path, help="input FIT recording")
    args = parser.parse_args()
    try:
        if args.command == "raw":
            for frame in wire_frames(args.file.read_bytes()):
                print(json.dumps(frame, ensure_ascii=False, separators=(",", ":"), allow_nan=False))
            return 0
        if args.decoder == "wire":
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
        parser.exit(1, "fit-fusion: install the decoder with python -m pip install -r requirements.txt\n")
    except Exception as error:
        # A failed decode must never produce a successful, partial inventory.
        parser.exit(1, f"fit-fusion: {args.file}: {error or type(error).__name__}\n")

    print(json.dumps(report, indent=2, allow_nan=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
