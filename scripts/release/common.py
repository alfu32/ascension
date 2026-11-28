"""Shared release helpers (standard library only)."""

import hashlib
import platform
import re
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / "app"
APP = "ascension"
REPO = "alfu32/ascension"
# The game binary (rom-app) and the headless runner and server (rom-cli).
GAME_BIN = "rise-of-man"
CLI_BIN = "rom"
ARCH_ALIASES = {"amd64": "x86_64", "x86_64": "x86_64", "arm64": "aarch64", "aarch64": "aarch64"}


def app_version() -> str:
    """The app version: `version` under [workspace.package] in app/Cargo.toml."""
    text = (WORKSPACE / "Cargo.toml").read_text()
    section = re.search(r"^\[workspace\.package\]\s*$(.*?)(?=^\[|\Z)", text, re.M | re.S)
    m = section and re.search(r'^version\s*=\s*"([^"]+)"', section.group(1), re.M)
    if not m:
        raise SystemExit("no version under [workspace.package] in app/Cargo.toml")
    return m.group(1)


def host_arch() -> str:
    return ARCH_ALIASES.get(platform.machine().lower(), platform.machine().lower())


def appimage_name(version: str, arch: str) -> str:
    """AppImage asset name: <app>-<version>-linux-<arch>.AppImage."""
    return f"{APP}-{version}-linux-{arch}.AppImage"


def update_information(arch: str) -> str:
    """Embedded AppImage update information (zsync via GitHub releases)."""
    owner, repo = REPO.split("/")
    return f"gh-releases-zsync|{owner}|{repo}|latest|{APP}-*-linux-{arch}.AppImage.zsync"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def write_sidecar(path: Path) -> None:
    """`<file>.sha256` beside a release file."""
    (path.parent / (path.name + ".sha256")).write_text(f"{sha256(path)}  {path.name}\n")


def download(url: str, digest: str, dest: Path, attempts: int = 4) -> Path:
    """Download once into `dest` and verify the pinned SHA-256 (retries network errors)."""
    if dest.is_file() and sha256(dest) == digest:
        return dest
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    for attempt in range(1, attempts + 1):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "release-script"})
            with urllib.request.urlopen(request, timeout=120) as r, tmp.open("wb") as f:
                while chunk := r.read(1 << 20):
                    f.write(chunk)
            break
        except OSError:
            if attempt == attempts:
                raise
            time.sleep(10 * attempt)
    got = sha256(tmp)
    if got != digest:
        tmp.unlink()
        raise SystemExit(f"checksum mismatch for {url}: {got}")
    tmp.rename(dest)
    return dest


def render_desktop(version: str) -> str:
    text = (ROOT / "packaging/linux/ascension.desktop").read_text()
    return text.replace("@VERSION@", version)


def parse_desktop(text: str) -> dict:
    """Parse a desktop file into {group: {key: value}}."""
    groups, current = {}, None
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = groups.setdefault(line[1:-1], {})
        elif "=" in line and current is not None:
            k, v = line.split("=", 1)
            current[k.strip()] = v.strip()
    return groups


# Keys an AppImage manager reads from the embedded desktop entry.
DESKTOP_KEYS = ("Name", "Icon", "X-AppImage-Version", "Comment", "Categories",
                "Keywords", "StartupWMClass", "StartupNotify", "Terminal", "Exec")


def check_desktop(groups: dict, version: str) -> list:
    """Problems that would make an AppImage manager show less than it can."""
    entry = groups.get("Desktop Entry", {})
    problems = [f"missing {k}" for k in DESKTOP_KEYS if not entry.get(k)]
    if entry.get("X-AppImage-Version") not in (None, version):
        problems.append(f"X-AppImage-Version {entry.get('X-AppImage-Version')} != {version}")
    if entry.get("Icon") != APP:
        problems.append(f"Icon must be the bare name '{APP}'")
    if not re.fullmatch(r"[A-Za-z0-9._+\-]{1,100}", entry.get("StartupWMClass", "")):
        problems.append("StartupWMClass has unusual characters")
    for action in filter(None, entry.get("Actions", "").split(";")):
        g = groups.get(f"Desktop Action {action}", {})
        if not g.get("Name") or not g.get("Exec"):
            problems.append(f"action {action} needs Name and Exec")
    return problems


def payload_files() -> list[tuple[Path, str]]:
    """Documents that travel in every package: (source, name in the package)."""
    files = [(ROOT / "README.md", "README.md"),
             (ROOT / "LICENSE", "LICENSE"),
             (ROOT / "THIRD-PARTY-NOTICES.md", "THIRD-PARTY-NOTICES.md"),
             (WORKSPACE / "assets/fonts/LICENSE-DejaVu.txt", "licenses/LICENSE-DejaVu.txt")]
    return [(src, name) for src, name in files if src.is_file()]
