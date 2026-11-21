#!/usr/bin/env python3
"""Portable archive: ascension-X.Y.Z-<target>.tar.gz (or .zip on Windows) with
both binaries (rise-of-man, rom), the README, licences, the icon (and on Linux
the desktop entry), and a file manifest. The binaries carry everything the game
reads at run time, so the archive runs from any folder."""

import argparse
import hashlib
import json
import shutil
import sys
import tarfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

ROOT = common.ROOT


def stage(target: str, bin_dir: Path, version: str) -> Path:
    exe = ".exe" if "windows" in target else ""
    stem = f"{common.APP}-{version}-{target}"
    out = common.WORKSPACE / "target/portable" / stem
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)
    for b in (common.GAME_BIN, common.CLI_BIN):
        shutil.copy2(bin_dir / (b + exe), out / (b + exe))
    for src, name in common.payload_files():
        (out / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, out / name)
    shutil.copy2(ROOT / f"packaging/{common.APP}.png", out / f"{common.APP}.png")
    if "linux" in target:
        (out / f"{common.APP}.desktop").write_text(common.render_desktop(version))
    if "windows" in target:
        shutil.copy2(ROOT / f"packaging/windows/{common.APP}.ico", out / f"{common.APP}.ico")
    manifest = {
        "name": common.APP,
        "version": version,
        "target": target,
        "files": {
            p.relative_to(out).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(out.rglob("*")) if p.is_file()
        },
    }
    (out / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--target", required=True, help="e.g. x86_64-unknown-linux-gnu")
    ap.add_argument("--bin-dir", type=Path, default=common.WORKSPACE / "target/release")
    ap.add_argument("--output", type=Path, default=ROOT / "dist")
    a = ap.parse_args()
    version = common.app_version()
    staged = stage(a.target, a.bin_dir, version)
    stem = staged.name
    a.output.mkdir(parents=True, exist_ok=True)
    if "windows" in a.target:
        out = a.output / f"{stem}.zip"
        with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
            for p in sorted(staged.rglob("*")):
                z.write(p, (Path(stem) / p.relative_to(staged)).as_posix())
    else:
        out = a.output / f"{stem}.tar.gz"
        with tarfile.open(out, "w:gz") as t:
            t.add(staged, arcname=stem)
    common.write_sidecar(out)
    print(out)


if __name__ == "__main__":
    main()
