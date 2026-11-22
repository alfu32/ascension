#!/usr/bin/env python3
"""Pack a strict-confinement Snap (core24 base, mesa-2404 GPU content):
ascension_<version>_<arch>.snap plus a .sha256 sidecar. Snap Store name:
packaging/snapstore.

The game window is GPUI (Vulkan) showing raylib's offscreen image (OpenGL).
core24 provides libc, libstdc++ and xkbcommon; mesa-2404 provides OpenGL/EGL,
the Vulkan drivers, libX11, libxcb and the Wayland client libraries.
Everything else the binaries load is copied from the build host into
$SNAP/usr/lib with its own dependencies, together with the XKB keyboard data.
Sound (task 125): the game plays through ALSA (libasound, bundled), whose pulse
plugin (bundled with libpulse) reaches the desktop's sound server, PulseAudio
or PipeWire's pulse socket, through the audio-playback interface. Without it,
or without a server, the game is silent and plays on.
Requires `snap` (snapd) and mksquashfs.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

ROOT = common.ROOT
NAME = (ROOT / "packaging/snapstore").read_text().strip()
BINARIES = (common.GAME_BIN, common.CLI_BIN)

# Loaded by GPUI (X11, keyboard, Vulkan loader) and raylib's GLFW window, and
# not in core24 or mesa-2404.
BUNDLE = (
    "libxkbcommon-x11.so.0",
    "libxcb-xkb.so.1",
    "libXcursor.so.1",
    "libXrandr.so.2",
    "libXi.so.6",
    "libXinerama.so.1",
    "libXrender.so.1",
    "libvulkan.so.1",
    # Sound (task 125): ALSA, and libpulse for its pulse plugin (ALSA_PLUGINS).
    "libasound.so.2",
    "libpulse.so.0",
)
# The ALSA plugins that send the game's sound to the desktop's sound server; copied into
# $SNAP/usr/lib/alsa-lib and named in the ALSA configuration the launcher writes.
ALSA_PLUGINS = ("libasound_module_pcm_pulse.so", "libasound_module_ctl_pulse.so")

# Provided inside the snap's runtime: the core24 base or the mesa-2404 GPU content.
PROVIDED = {
    # core24
    "linux-vdso.so.1", "ld-linux-x86-64.so.2", "ld-linux-aarch64.so.1", "libc.so.6", "libm.so.6",
    "libdl.so.2", "libpthread.so.0", "librt.so.1", "libgcc_s.so.1", "libstdc++.so.6",
    "libdbus-1.so.3", "libffi.so.8", "libxkbcommon.so.0", "libsystemd.so.0", "libbsd.so.0",
    "libmd.so.0", "libz.so.1", "liblzma.so.5", "libzstd.so.1", "liblz4.so.1", "libcap.so.2",
    "libgcrypt.so.20", "libgpg-error.so.0", "libexpat.so.1", "libmvec.so.1",
    # mesa-2404
    "libX11.so.6", "libX11-xcb.so.1", "libXau.so.6", "libXdmcp.so.6", "libXext.so.6",
    "libXfixes.so.3", "libXxf86vm.so.1", "libxcb.so.1", "libxcb-dri2.so.0", "libxcb-dri3.so.0",
    "libxcb-glx.so.0", "libxcb-present.so.0", "libxcb-randr.so.0", "libxcb-shm.so.0",
    "libxcb-sync.so.1", "libxcb-xfixes.so.0", "libwayland-client.so.0", "libwayland-cursor.so.0",
    "libwayland-egl.so.1", "libwayland-server.so.0", "libGL.so.1", "libEGL.so.1", "libGLX.so.0",
    "libGLdispatch.so.0", "libOpenGL.so.0", "libGLESv2.so.2", "libgbm.so.1", "libdrm.so.2",
}

SNAP_YAML = """name: {name}
title: Ascension
version: '{version}'
summary: Real-time strategy from early history to an age of galactic empires
description: |
  Lead a people through fourteen ages, from the first villages to starships.
  Found cities on a round planet, research, trade and raise armies; reach
  space, claim resource worlds and cross between star systems by charged
  jumps along routes. Nations grow into continental blocs and finally into
  united humanity. Play against computer opponents or with others through a
  network server.
base: core24
architectures: [{arch}]
plugs:
  gpu-2404:
    interface: content
    content: gpu-2404
    default-provider: mesa-2404
    target: $SNAP/gpu-2404
confinement: strict
grade: stable
license: MIT OR Apache-2.0
apps:
  {name}:
    command: bin/{game}
    common-id: io.github.alfu32.ascension
    desktop: meta/gui/ascension.desktop
    # network: joining a server; network-bind: hosting a match or running a server.
    # audio-playback: the game's sound (task 125).
    plugs: [home, removable-media, desktop, desktop-legacy, wayland, x11, opengl, gpu-2404, network, network-bind, audio-playback]
  {cli}:
    command: bin/{cli}
    plugs: [home, removable-media, network, network-bind]
"""

LAUNCHER = """#!/bin/sh
export LD_LIBRARY_PATH="$SNAP/usr/lib${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}"
# Sound (task 125): ALSA's default device is the pulse plugin, which talks to the desktop's
# sound server (PulseAudio, or PipeWire's pulse socket) through the audio-playback interface.
pulse="${{XDG_RUNTIME_DIR:-/nonexistent}}/../pulse/native"
if [ -S "$pulse" ]; then
  export PULSE_SERVER="unix:$pulse"
fi
alsa_conf="${{SNAP_USER_DATA:-${{TMPDIR:-/tmp}}}}/asound.conf"
if printf '%s\\n' \\
  "pcm_type.pulse {{ lib \\"$SNAP/usr/lib/alsa-lib/libasound_module_pcm_pulse.so\\" }}" \\
  "ctl_type.pulse {{ lib \\"$SNAP/usr/lib/alsa-lib/libasound_module_ctl_pulse.so\\" }}" \\
  "pcm.!default {{ type pulse }}" \\
  "ctl.!default {{ type pulse }}" > "$alsa_conf" 2>/dev/null; then
  export ALSA_CONFIG_PATH="$alsa_conf"
fi
if [ -d "$SNAP/usr/share/X11/xkb" ]; then
  export XKB_CONFIG_ROOT="$SNAP/usr/share/X11/xkb"
fi
exec "$SNAP/gpu-2404/bin/gpu-2404-provider-wrapper" "$SNAP/opt/{app}/{binary}" "$@"
"""


def snap_arch(arch: str) -> str:
    return {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}[arch]


def snap_name(version: str, arch: str) -> str:
    return f"{NAME}_{version}_{snap_arch(arch)}.snap"


def host_library(soname: str) -> Path | None:
    """Path of a library on the build host (ldconfig cache)."""
    out = subprocess.run(["ldconfig", "-p"], capture_output=True, text=True, check=True).stdout
    machine = "x86-64" if common.host_arch() == "x86_64" else "AArch64"
    for line in out.splitlines():
        m = re.match(r"\s*(\S+) \(([^)]*)\) => (\S+)", line)
        if m and m.group(1) == soname and machine in m.group(2):
            return Path(m.group(3))
    return None


# Where ldd found each library, for those outside the ldconfig cache (libpulsecommon lives in a
# private folder of PulseAudio).
LDD_PATHS: dict[str, Path] = {}


def needed(lib: Path) -> list[str]:
    """Direct and indirect dependencies (sonames) of a library."""
    out = subprocess.run(["ldd", str(lib)], capture_output=True, text=True, check=True).stdout
    names = []
    for line in out.splitlines():
        parts = line.split()
        if line.strip() and "=>" in line:
            names.append(parts[0])
            if len(parts) > 2 and parts[2].startswith("/"):
                LDD_PATHS.setdefault(parts[0], Path(parts[2]))
    return names


def host_alsa_plugins() -> dict[str, Path]:
    """The ALSA pulse plugins on the build host (package libasound2-plugins)."""
    lib = host_library("libasound.so.2")
    if lib is None:
        raise SystemExit("libasound.so.2 is not installed on the build host")
    folder = lib.resolve().parent / "alsa-lib"
    plugins = {name: folder / name for name in ALSA_PLUGINS}
    missing = [name for name, path in plugins.items() if not path.is_file()]
    if missing:
        raise SystemExit(f"{', '.join(missing)} not in {folder}: install libasound2-plugins")
    return plugins


def library_closure(roots=BUNDLE, resolve=host_library, deps=needed) -> dict[str, Path]:
    """Every library to copy: the roots and their dependencies not provided."""
    todo, out = list(roots), {}
    while todo:
        name = todo.pop()
        if name in out or name in PROVIDED:
            continue
        path = resolve(name) or LDD_PATHS.get(name)
        if path is None:
            raise SystemExit(f"{name} is not installed on the build host")
        out[name] = path
        todo.extend(d for d in deps(path) if d not in PROVIDED)
    return out


def stage(root: Path, version: str, arch: str, bin_dir: Path, libs: dict[str, Path], xkb: Path | None,
          plugins: dict[str, Path] | None = None):
    app = common.APP
    (root / "opt" / app).mkdir(parents=True)
    for b in BINARIES:
        shutil.copy2(bin_dir / b, root / "opt" / app / b)
    (root / "bin").mkdir()
    for b in BINARIES:
        launcher = root / "bin" / b
        launcher.write_text(LAUNCHER.format(app=app, binary=b))
        launcher.chmod(0o755)
    (root / "gpu-2404").mkdir()
    (root / "usr/lib").mkdir(parents=True)
    for name, path in sorted(libs.items()):
        shutil.copy2(path.resolve(), root / "usr/lib" / name)
    if plugins:
        (root / "usr/lib/alsa-lib").mkdir()
        for name, path in sorted(plugins.items()):
            shutil.copy2(path.resolve(), root / "usr/lib/alsa-lib" / name)
    if xkb is not None and xkb.is_dir():
        shutil.copytree(xkb, root / "usr/share/X11/xkb", symlinks=True)
    doc = root / "usr/share/doc" / app
    for src, name in common.payload_files():
        (doc / name).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src, doc / name)
    (root / "meta/gui").mkdir(parents=True)
    shutil.copy2(ROOT / f"packaging/{app}.png", root / f"meta/gui/{app}.png")
    desktop = common.render_desktop(version)
    desktop = desktop.replace(f"Icon={app}", f"Icon=${{SNAP}}/meta/gui/{app}.png")
    # snapd rewrites Exec to the snap's app; the server action runs the same app.
    desktop = desktop.replace(f"Exec={common.GAME_BIN}", f"Exec={NAME}")
    desktop = "\n".join(l for l in desktop.splitlines() if not l.startswith("X-AppImage-")) + "\n"
    (root / f"meta/gui/{app}.desktop").write_text(desktop)
    (root / "meta/snap.yaml").write_text(SNAP_YAML.format(
        name=NAME, version=version, arch=snap_arch(arch), game=common.GAME_BIN, cli=common.CLI_BIN))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bin-dir", type=Path, default=common.WORKSPACE / "target/release")
    ap.add_argument("--arch", default=common.host_arch())
    ap.add_argument("--output", type=Path, default=ROOT / "dist")
    ap.add_argument("--xkb", type=Path, default=Path("/usr/share/X11/xkb"))
    a = ap.parse_args()
    version = common.app_version()
    root = common.WORKSPACE / "target/snap/prime"
    if root.exists():
        shutil.rmtree(root)
    plugins = host_alsa_plugins()
    roots = BUNDLE
    libs = library_closure(roots)
    # The plugins' own dependencies (libpulse's codecs and its private libpulsecommon).
    for plugin in plugins.values():
        for name in needed(plugin):
            if name not in PROVIDED and name not in libs:
                libs.update(library_closure((name,)))
    stage(root, version, a.arch, a.bin_dir, libs, a.xkb, plugins)
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
    for p in sorted(root.rglob("*"), reverse=True):
        os.utime(p, (epoch, epoch), follow_symlinks=False)
    a.output.mkdir(parents=True, exist_ok=True)
    subprocess.run(["snap", "pack", str(root), str(a.output.resolve()), "--compression=xz"], check=True)
    artifact = a.output / snap_name(version, a.arch)
    if not artifact.is_file():
        raise SystemExit(f"{artifact} was not produced")
    common.write_sidecar(artifact)
    print(artifact)


if __name__ == "__main__":
    main()
