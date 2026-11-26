#!/usr/bin/env python3
"""Generate installable Homebrew formulae from FIT Fusion's public stable releases."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
from string import Template
import tarfile
from urllib.request import Request, urlopen

REPO = "andrew0928/fit-fusion"
HOMEPAGE = f"https://github.com/{REPO}"
VERSION = r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)"
TOOLS = ("fit-merge", "fit-inspector", "fit-query-files", "fit-query-apple", "fit-query-garmin")
LEGACY_TOOLS = {
    "0.1.0": ("fit-inspector",),
    "0.2.0": ("fit-merge", "fit-inspector", "fit-query-files"),
    "0.2.1": ("fit-merge", "fit-inspector", "fit-query-files", "fit-query-garmin"),
}
NOTICES = ("THIRD-PARTY-NOTICES.txt", "RUST-LIBRARY-COPYRIGHT.html")

FORMULA = Template('''# Generated from FIT Fusion releases by scripts/sync-fit-fusion.py.
class $classname < Formula
  desc "Merge, inspect and query cycling FIT activities"
  homepage "$homepage"
$license  url "$url"
  version "$version"
  sha256 "$sha"
$revision$keg
  depends_on arch: :arm64
  depends_on :macos

  TOOLS = %w[$tools].freeze

  def install
    bin.install(*TOOLS)
$notices  end
$caveats
  test do
    assert_equal TOOLS.sort, bin.children.map { |tool| tool.basename.to_s }.sort
    TOOLS.each do |tool|
      assert_match tool, shell_output("#{bin}/#{tool} --help")
    end
$query_test  end
end
''')
CONFIG_CAVEATS = """
  def caveats
    "To create a fully commented personal config, run: #{opt_bin}/fit-query-files config init"
  end
"""
QUERY_TEST = '''
    mkdir "rides" do
      result = JSON.parse(shell_output("#{bin}/fit-query-files --dir . list --json"))
      assert_equal "ok", result.fetch("status")
      assert_empty result.fetch("activities")
    end
'''


def releases():
    headers = {"Accept": "application/vnd.github+json"}
    token = os.getenv("GH_TOKEN") or os.getenv("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    result = {}
    page = 1
    while True:
        url = f"https://api.github.com/repos/{REPO}/releases?per_page=100&page={page}"
        with urlopen(Request(url, headers=headers), timeout=60) as response:
            batch = json.load(response)
        for release in batch:
            tag = release["tag_name"]
            if not release["draft"] and not release["prerelease"] and re.fullmatch("v" + VERSION, tag):
                result[tag[1:]] = release["assets"]
        if len(batch) < 100:
            return result
        page += 1


def asset_bytes(assets, tag, name):
    url = f"{HOMEPAGE}/releases/download/{tag}/{name}"
    if name not in assets or assets[name]["browser_download_url"] != url:
        raise ValueError(f"{tag}: missing expected published asset {name}")
    with urlopen(url, timeout=120) as response:
        data = response.read()
    digest = assets[name].get("digest")
    if digest and digest != "sha256:" + hashlib.sha256(data).hexdigest():
        raise ValueError(f"{tag}: GitHub digest mismatch for {name}")
    return data


def package(version, assets):
    tag = f"v{version}"
    stem = f"fit-fusion-{tag}-macos-arm64"
    # The original v0.2.2 archive predates the distribution of third-party notices.
    name = stem + ("-homebrew.tar.gz" if version == "0.2.2" else ".tar.gz")
    sums = name + ".sha256" if version == "0.2.2" else "SHA256SUMS"
    assets = {asset["name"]: asset for asset in assets}
    manifest = asset_bytes(assets, tag, sums).decode()
    archive = asset_bytes(assets, tag, name)
    sha = hashlib.sha256(archive).hexdigest()
    entries = re.findall(r"^([0-9a-fA-F]{64})[ \t]+\*?" + re.escape(name) + r"\r?$", manifest, re.M)
    if [s.lower() for s in entries] != [sha]:
        raise ValueError(f"{tag}: incorrect or ambiguous checksum for {name}")

    tools = LEGACY_TOOLS.get(version, TOOLS)
    notices = () if version in LEGACY_TOOLS else NOTICES
    release_version = tuple(map(int, version.split(".")))
    # 0.2.3 introduced the shared YAML template.
    if release_version >= (0, 2, 3):
        notices = (*notices, "config.example.yaml")
    # The MIT decision followed the v0.2.4 prerelease.
    if release_version > (0, 2, 4):
        notices = (*notices, "LICENSE")
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as tar:
        members = tar.getmembers()
        if release_version >= (0, 2, 4) and any(PurePosixPath(m.name).name == "fit-fusion" for m in members):
            raise ValueError(f"{tag}: archive contains the retired fit-fusion executable")
        files = {PurePosixPath(m.name): m for m in members if m.isfile()}
    # Homebrew removes a single top-level wrapper directory when it unpacks.
    nested = all(len(path.parts) > 1 for path in files)
    roots = {path.parts[0] for path in files}
    root = PurePosixPath(next(iter(roots))) if nested and len(roots) == 1 else PurePosixPath(".")
    for tool in tools:
        member = files.get(root / tool)
        if not member or not member.mode & 0o111:
            raise ValueError(f"{tag}: missing executable {tool}")
    for notice in notices:
        if root / notice not in files:
            raise ValueError(f"{tag}: missing {notice}")
    return assets[name]["browser_download_url"], sha, tools, notices


def revision_for(paths, version, sha):
    for path in paths:
        if path.exists():
            previous = path.read_text()
            if f'  version "{version}"\n' in previous:
                old = re.search(r"^  revision (\d+)$", previous, re.M)
                revision = int(old.group(1)) if old else 0
                return revision + int(f'  sha256 "{sha}"\n' not in previous)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tap", type=Path, help="checkout of andrew0928/homebrew-tools")
    folder = parser.parse_args().tap / "Formula"
    if not folder.is_dir():
        parser.error("the tap checkout must have a Formula directory")
    available = releases()
    if not available:
        raise ValueError("No published stable vMAJOR.MINOR.PATCH releases found")
    latest = max(available, key=lambda v: tuple(map(int, v.split("."))))
    stable = folder / "fit-fusion.rb"
    generated = {}
    for version, assets in available.items():
        url, sha, tools, notices = package(version, assets)
        versioned = folder / f"fit-fusion@{version}.rb"
        revision = revision_for((versioned, stable), version, sha)
        notice_files = ", ".join(f'"{notice}"' for notice in notices)
        fields = {
            "homepage": HOMEPAGE,
            "license": '  license "MIT"\n' if "LICENSE" in notices else "",
            "version": version,
            "url": url,
            "sha": sha,
            "tools": " ".join(tools),
            "revision": f"  revision {revision}\n" if revision else "",
            "notices": f"    pkgshare.install {notice_files}\n" if notices else "",
            "caveats": CONFIG_CAVEATS if "config.example.yaml" in notices else "",
            "query_test": QUERY_TEST if "fit-query-files" in tools else "",
        }
        generated[versioned] = FORMULA.substitute(
            fields, classname="FitFusionAT" + version.replace(".", ""), keg="\n  keg_only :versioned_formula\n")
        if version == latest:
            generated[stable] = FORMULA.substitute(fields, classname="FitFusion", keg="")
    # Validate every release before changing the tap.
    for path, contents in generated.items():
        if not path.exists() or path.read_text() != contents:
            path.write_text(contents)
    for path in folder.glob("fit-fusion@*.rb"):
        if re.fullmatch("fit-fusion@" + VERSION + r"\.rb", path.name) and path not in generated:
            path.unlink()
    print(f"{len(available)} versioned formulae; current fit-fusion {latest}")


if __name__ == "__main__":
    main()
