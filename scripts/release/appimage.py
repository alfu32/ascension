#!/usr/bin/env python3
"""Build an AppImage (+ zsync) from the release binaries.

    (cd app && cargo build --release --locked -p rom-app -p rom-cli)
    python3 scripts/release/appimage.py      # -> dist/ascension-X.Y.Z-linux-<arch>.AppImage

appimagetool and the type 2 runtime are pinned by SHA-256 in
packaging/appimage-inputs.json, as are the runtime's licence notices, which are
fetched at build time into usr/share/doc. appimagetool runs with
APPIMAGE_EXTRACT_AND_RUN=1, so FUSE is not required.

Everything the game reads at run time (rules, schema, shaders, fonts, HUD
artwork, scripts) is compiled into the binaries, so the AppImage starts from
any working directory.
"""

import argparse
import datetime
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

ROOT = common.ROOT
APP = common.APP
METAINFO = "io.github.alfu32.ascension.metainfo.xml"


def pins() -> dict:
    return json.loads((ROOT / "packaging/appimage-inputs.json").read_text())


def tools(arch: str, cache: Path):
    p = pins()
    # appimagetool runs on the build host; the runtime matches the target.
    selected = p.get(arch, p) if arch != "x86_64" else p
    host = common.host_arch()
    host_pins = p.get(host, p) if host != "x86_64" else p
    tool = common.download(host_pins["tool"]["url"], host_pins["tool"]["sha256"], cache / f"appimagetool-{host}.AppImage")
    tool.chmod(tool.stat().st_mode | stat.S_IEXEC)
    runtime = common.download(selected["runtime"]["url"], selected["runtime"]["sha256"], cache / f"runtime-{arch}")
    return tool, runtime


def metainfo(version: str, date: str) -> str:
    """The AppStream file with this release as the newest entry."""
    text = (ROOT / "packaging/linux" / METAINFO).read_text()
    return re.sub(r"<releases>.*?</releases>",
                  f'<releases>\n    <release version="{version}" date="{date}"/>\n  </releases>',
                  text, flags=re.S)


def build_appdir(appdir: Path, bin_dir: Path, version: str, cache: Path):
    if appdir.exists():
        shutil.rmtree(appdir)
    usr = appdir / "usr"
    (usr / "bin").mkdir(parents=True)
    for name in (common.GAME_BIN, common.CLI_BIN):
        shutil.copy2(bin_dir / name, usr / "bin" / name)
    desktop = common.render_desktop(version)
    for d in (appdir, usr / "share/applications"):
        d.mkdir(parents=True, exist_ok=True)
        (d / f"{APP}.desktop").write_text(desktop)
    icon = ROOT / f"packaging/{APP}.png"
    shutil.copy2(icon, appdir / f"{APP}.png")
    shutil.copy2(icon, appdir / ".DirIcon")
    hicolor = usr / "share/icons/hicolor/512x512/apps"
    hicolor.mkdir(parents=True)
    shutil.copy2(icon, hicolor / f"{APP}.png")
    (usr / "share/metainfo").mkdir(parents=True)
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH") or datetime.datetime.now().timestamp())
    date = datetime.datetime.fromtimestamp(epoch, datetime.timezone.utc).strftime("%Y-%m-%d")
    (usr / "share/metainfo" / METAINFO).write_text(metainfo(version, date))
    doc = usr / f"share/doc/{APP}"
    doc.mkdir(parents=True)
    for src, name in common.payload_files():
        (doc / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, doc / name)
    for notice in pins().get("notices", []):
        dest = doc / "appimage-runtime" / notice["file"]
        common.download(notice["url"], notice["sha256"], cache / "notices" / notice["file"])
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(cache / "notices" / notice["file"], dest)
    shutil.copy2(ROOT / "packaging/linux/AppRun", appdir / "AppRun")
    (appdir / "AppRun").chmod(0o755)


def verify(image: Path, version: str, arch: str):
    """Check what an AppImage manager will read from the finished AppImage."""
    problems = []
    if image.name != common.appimage_name(version, arch):
        problems.append(f"file name {image.name} is not the expected pattern")
    with tempfile.TemporaryDirectory() as tmp:
        env = dict(os.environ, APPIMAGE_EXTRACT_AND_RUN="1")
        for pattern in (f"{APP}.desktop", f"{APP}.png", ".DirIcon", "usr/bin/*"):
            subprocess.run([str(image.resolve()), "--appimage-extract", pattern], cwd=tmp, env=env,
                           check=True, stdout=subprocess.DEVNULL)
        root = Path(tmp) / "squashfs-root"
        groups = common.parse_desktop((root / f"{APP}.desktop").read_text())
        problems += common.check_desktop(groups, version)
        for f in (f"{APP}.png", ".DirIcon", f"usr/bin/{common.GAME_BIN}", f"usr/bin/{common.CLI_BIN}"):
            if not (root / f).exists():
                problems.append(f"{f} missing in the AppImage")
    out = subprocess.run([str(image.resolve()), "--version"], capture_output=True, text=True,
                         env=dict(os.environ, APPIMAGE_EXTRACT_AND_RUN="1"), cwd="/")
    if out.stdout.strip() != f"{common.GAME_BIN} {version}":
        problems.append(f"--version printed {out.stdout.strip()!r} {out.stderr.strip()!r}")
    if problems:
        raise SystemExit("AppImage check failed:\n  " + "\n  ".join(problems))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bin-dir", type=Path, default=common.WORKSPACE / "target/release")
    ap.add_argument("--arch", default=common.host_arch())
    ap.add_argument("--output", type=Path, default=ROOT / "dist")
    ap.add_argument("--no-update-info", action="store_true", help="do not embed zsync update information")
    a = ap.parse_args()
    version = common.app_version()
    if not (a.bin_dir / common.GAME_BIN).is_file():
        raise SystemExit(f"{a.bin_dir / common.GAME_BIN} not found; run cargo build --release first")
    cache = common.WORKSPACE / "target/appimage-tools"
    tool, runtime = tools(a.arch, cache)
    appdir = common.WORKSPACE / "target/appimage/Ascension.AppDir"
    build_appdir(appdir, a.bin_dir, version, cache)
    a.output.mkdir(parents=True, exist_ok=True)
    image = a.output / common.appimage_name(version, a.arch)
    cmd = [str(tool), "--no-appstream", "--runtime-file", str(runtime)]
    if not a.no_update_info:
        cmd += ["-u", common.update_information(a.arch)]
    cmd += [str(appdir), str(image.resolve())]
    env = dict(os.environ, APPIMAGE_EXTRACT_AND_RUN="1", ARCH=a.arch, VERSION=version)
    subprocess.run(cmd, check=True, env=env, cwd=a.output)
    verify(image, version, a.arch)
    for f in [image, *a.output.glob(image.name + ".zsync")]:
        common.write_sidecar(f)
    print(image)


if __name__ == "__main__":
    main()
