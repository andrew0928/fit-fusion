"""Launch the shared clap frontend with the independent Python decoder."""
import os
from pathlib import Path
import shutil
import sys

from . import __version__


def main():
    local = Path(__file__).resolve().parents[3] / "runtime/build/rust/release/fit-inspector-bridge"
    bridge = local if local.is_file() else shutil.which("fit-inspector-bridge")
    if not bridge:
        sys.exit("Build the frontend with cargo build --release --manifest-path src/rust/inspector/Cargo.toml")
    os.execv(bridge, [str(bridge), "python", f"fit-inspector {__version__} (Python)",
                      sys.executable, "-m", "fit_fusion.worker", "--", *sys.argv[1:]])


if __name__ == "__main__":
    main()
