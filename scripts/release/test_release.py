"""Unit tests for the release helpers: python3 -m unittest discover -s scripts/release -p 'test_*.py' -v"""

import re
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import appimage  # noqa: E402
import common  # noqa: E402
import msix  # noqa: E402
import snap  # noqa: E402

WORKFLOW = (common.ROOT / ".github/workflows/release.yaml").read_text()


class MsixTests(unittest.TestCase):
    def test_store_identity_and_version_mapping(self):
        ident = msix.identity()
        self.assertEqual(ident["identityName"], "K3D0.ascension-rts-platform")
        self.assertEqual(ident["packageFamilyName"], "K3D0.ascension-rts-platform_tfmtd4yrk952g")
        self.assertEqual(ident["storeId"], "9PHHR2KQKVJ2")
        self.assertEqual(ident["publisher"], "CN=7F0CBC95-8035-4AF2-9B17-C38D58351CE5")
        self.assertEqual(msix.package_version("0.0.112", ident), "1.0.112.0")
        self.assertEqual(msix.package_version("2.10.3", ident), "3.10.3.0")
        with self.assertRaises(ValueError):
            msix.package_version("1.2", ident)
        self.assertEqual(msix.msix_name("0.0.112", "x64"), "ascension-0.0.112-windows-x64.msix")
        self.assertEqual(msix.msi_name("0.0.112", "arm64"), "ascension-0.0.112-windows-arm64.msi")

    def test_identity_matches_the_owner_record(self):
        record = (common.ROOT / "packaging/msstore-identity.txt").read_text().split()
        ident = msix.identity()
        for field in ("identityName", "publisher", "publisherDisplayName", "packageFamilyName", "storeId"):
            self.assertIn(ident[field], record)

    def test_msi_version_bounds(self):
        self.assertEqual(msix.msi_version("0.0.112"), "0.0.112")
        for bad in ("256.0.0", "0.256.0", "0.0.65536", "1.2"):
            with self.assertRaises(ValueError):
                msix.msi_version(bad)

    def test_manifest_has_identity_and_full_trust(self):
        ns = {"m": msix.NS[""], "uap": msix.NS["uap"], "rescap": msix.NS["rescap"]}
        text = msix.manifest("0.0.112", "x64")
        root = ET.fromstring(text.split("\n", 1)[1])
        ident = root.find("m:Identity", ns)
        self.assertEqual(ident.get("Name"), "K3D0.ascension-rts-platform")
        self.assertEqual(ident.get("Publisher"), msix.identity()["publisher"])
        self.assertEqual(ident.get("Version"), "1.0.112.0")
        app = root.find("m:Applications/m:Application", ns)
        self.assertEqual((app.get("Executable"), app.get("EntryPoint")), ("rise-of-man.exe", "Windows.FullTrustApplication"))
        self.assertEqual(root.find(".//rescap:Capability", ns).get("Name"), "runFullTrust")
        for logo in ("StoreLogo", "Square44x44Logo", "Square150x150Logo"):
            self.assertIn(f"Assets\\{logo}.png", text)
        arm = ET.fromstring(msix.manifest("0.0.112", "arm64").split("\n", 1)[1]).find("m:Identity", ns)
        self.assertEqual(arm.get("ProcessorArchitecture"), "arm64")

    def test_windows_packaging_builds_msi_and_msix_for_both_architectures(self):
        self.assertIn("windows-11-arm", WORKFLOW)
        self.assertIn("msix: arm64", WORKFLOW)
        self.assertIn("msix: x64", WORKFLOW)
        package = (common.ROOT / "packaging/windows/package.ps1").read_text()
        for word in ("StoreLogo", "Square44x44Logo", "Square150x150Logo", "wix build", "makeappx",
                     "ASCENSION_PFX_B64", "ASCENSION_PFX_PASSWORD", ".cer", "msiexec"):
            self.assertIn(word, package)
        wxs = ET.parse(common.ROOT / "packaging/windows/ascension.wxs").getroot()
        ns = {"w": "http://wixtoolset.org/schemas/v4/wxs"}
        self.assertEqual(wxs.find("w:Package", ns).get("Name"), "Ascension")
        self.assertIsNotNone(wxs.find(".//w:StandardDirectory[@Id='ProgramFiles6432Folder']", ns))
        shortcut = wxs.find(".//w:Shortcut[@Id='StartAscension']", ns)
        self.assertEqual(shortcut.get("Target"), "[INSTALLFOLDER]rise-of-man.exe")
        self.assertTrue((common.ROOT / "packaging/windows/ascension.ico").is_file())


class SnapTests(unittest.TestCase):
    def test_snap_store_name_and_file_names(self):
        self.assertEqual(snap.NAME, "ascension")
        self.assertEqual(snap.snap_name("1.2.3", "x86_64"), "ascension_1.2.3_amd64.snap")
        self.assertEqual(snap.snap_name("1.2.3", "aarch64"), "ascension_1.2.3_arm64.snap")

    def test_library_closure_skips_what_core24_and_mesa_provide(self):
        deps = {
            "libXcursor.so.1": ["libXrender.so.1", "libXfixes.so.3", "libX11.so.6", "libc.so.6"],
            "libXrender.so.1": ["libX11.so.6"],
            "libxkbcommon-x11.so.0": ["libxkbcommon.so.0", "libxcb.so.1", "libxcb-xkb.so.1"],
            "libxcb-xkb.so.1": ["libxcb.so.1", "libXau.so.6"],
            "libXi.so.6": ["libXext.so.6", "libextra.so.2"],
            "libextra.so.2": [],
        }
        got = snap.library_closure(
            roots=("libXcursor.so.1", "libxkbcommon-x11.so.0", "libXi.so.6"),
            resolve=lambda n: Path("/lib") / n,
            deps=lambda p: deps.get(p.name, []),
        )
        self.assertEqual(
            sorted(got),
            ["libXcursor.so.1", "libXi.so.6", "libXrender.so.1", "libextra.so.2", "libxcb-xkb.so.1", "libxkbcommon-x11.so.0"],
        )

    def test_staged_snap_is_strict_core24_with_gpu_content_and_two_apps(self):
        with tempfile.TemporaryDirectory() as t:
            t = Path(t)
            bins = t / "bin"
            bins.mkdir()
            for b in snap.BINARIES:
                (bins / b).write_text("#!/bin/sh\n")
            lib = t / "libXi.so.6"
            lib.write_text("")
            root = t / "prime"
            plugin = t / "libasound_module_pcm_pulse.so"
            plugin.write_text("")
            snap.stage(root, "1.2.3", "aarch64", bins, {"libXi.so.6": lib}, None,
                       {"libasound_module_pcm_pulse.so": plugin})
            meta = (root / "meta/snap.yaml").read_text()
            for want in ("name: ascension\n", "version: '1.2.3'", "base: core24", "architectures: [arm64]",
                         "confinement: strict", "grade: stable", "default-provider: mesa-2404",
                         "command: bin/rise-of-man\n", "command: bin/rom\n", "network-bind",
                         "audio-playback]"):
                self.assertIn(want, meta)
            launcher = (root / "bin/rise-of-man").read_text()
            self.assertIn("gpu-2404-provider-wrapper", launcher)
            # Sound (task 125): ALSA's default is the bundled pulse plugin.
            self.assertIn('export ALSA_CONFIG_PATH="$alsa_conf"', launcher)
            self.assertIn('lib \\"$SNAP/usr/lib/alsa-lib/libasound_module_pcm_pulse.so\\"', launcher)
            self.assertIn('export PULSE_SERVER="unix:$pulse"', launcher)
            self.assertTrue((root / "usr/lib/alsa-lib/libasound_module_pcm_pulse.so").is_file())
            self.assertIn("libasound.so.2", snap.BUNDLE)
            self.assertIn('"$SNAP/opt/ascension/rise-of-man"', launcher)
            self.assertIn('LD_LIBRARY_PATH="$SNAP/usr/lib', launcher)
            self.assertTrue((root / "usr/lib/libXi.so.6").is_file())
            self.assertTrue((root / "usr/share/doc/ascension/README.md").is_file())
            desktop = common.parse_desktop((root / "meta/gui/ascension.desktop").read_text())["Desktop Entry"]
            self.assertEqual(desktop["Icon"], "${SNAP}/meta/gui/ascension.png")
            self.assertEqual(desktop["Exec"], "ascension")
            self.assertNotIn("X-AppImage-Version", desktop)


class ReleaseTests(unittest.TestCase):
    def test_version_is_semver_and_tags_have_no_prefix(self):
        self.assertRegex(common.app_version(), r"^\d+\.\d+\.\d+$")
        self.assertIn("tags: ['[0-9]+.[0-9]+.[0-9]+']", WORKFLOW)

    def test_workflow_has_every_job_and_publishes_only_after_all(self):
        for job in ("tag:", "validate:", "linux:", "desktop:", "publish:"):
            self.assertIn(f"\n  {job}", WORKFLOW)
        self.assertIn("needs: [tag, validate, linux, desktop]", WORKFLOW)
        for runner in ("ubuntu-22.04-arm", "windows-2022", "windows-11-arm", "macos-26", "macos-15-intel"):
            self.assertIn(runner, WORKFLOW)
        self.assertIn("-p rom-app -p rom-cli", WORKFLOW)

    def test_appimage_name_and_update_information(self):
        self.assertEqual(common.appimage_name("1.2.3", "x86_64"), "ascension-1.2.3-linux-x86_64.AppImage")
        self.assertEqual(
            common.update_information("aarch64"),
            "gh-releases-zsync|alfu32|ascension|latest|ascension-*-linux-aarch64.AppImage.zsync",
        )

    def test_desktop_entry_is_complete(self):
        v = common.app_version()
        groups = common.parse_desktop(common.render_desktop(v))
        self.assertEqual(common.check_desktop(groups, v), [])
        self.assertEqual(groups["Desktop Entry"]["X-AppImage-Version"], v)

    def test_startup_wm_class_matches_the_window_app_id(self):
        shell = (common.WORKSPACE / "crates/rom-app/src/shell.rs").read_text()
        groups = common.parse_desktop(common.render_desktop("0.0.0"))
        self.assertIn(f'app_id: Some("{groups["Desktop Entry"]["StartupWMClass"]}".into())', shell)

    def test_metainfo_gets_the_current_release(self):
        text = appimage.metainfo("9.8.7", "2030-01-02")
        self.assertIn('<release version="9.8.7" date="2030-01-02"/>', text)
        self.assertEqual(len(re.findall("<release ", text)), 1)
        ET.fromstring(text.encode())

    def test_binaries_are_the_workspace_bins(self):
        app = (common.WORKSPACE / "crates/rom-app/Cargo.toml").read_text()
        cli = (common.WORKSPACE / "crates/rom-cli/Cargo.toml").read_text()
        self.assertIn(f'name = "{common.GAME_BIN}"', app)
        self.assertIn(f'name = "{common.CLI_BIN}"', cli)

    def test_icon_and_appimage_pins(self):
        self.assertTrue((common.ROOT / "packaging/ascension.png").is_file())
        self.assertTrue((common.ROOT / "packaging/ascension.svg").is_file())
        pins = appimage.pins()
        for key in ("tool", "runtime"):
            self.assertRegex(pins[key]["sha256"], r"^[0-9a-f]{64}$")
            self.assertRegex(pins["aarch64"][key]["sha256"], r"^[0-9a-f]{64}$")
        self.assertTrue(all(re.fullmatch(r"[0-9a-f]{64}", n["sha256"]) for n in pins["notices"]))


if __name__ == "__main__":
    unittest.main()
