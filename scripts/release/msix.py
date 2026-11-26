#!/usr/bin/env python3
"""Store-associated MSIX manifest (AppxManifest.xml) with the reserved
Microsoft Store identity in packaging/windows/store-identity.json.
packaging/windows/package.ps1 packs and (when the signing secrets exist)
signs the package on the Windows runner."""

import argparse
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

IDENTITY = common.ROOT / "packaging/windows/store-identity.json"
NS = {
    "": "http://schemas.microsoft.com/appx/manifest/foundation/windows10",
    "uap": "http://schemas.microsoft.com/appx/manifest/uap/windows10",
    "rescap": "http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities",
}
for prefix, namespace in NS.items():
    ET.register_namespace(prefix, namespace)

DESCRIPTION = "Real-time strategy from early history to an age of galactic empires"


def identity(path=IDENTITY):
    value = json.loads(Path(path).read_text())
    for field in ("identityName", "publisher", "publisherDisplayName", "displayName", "packageFamilyName", "storeId", "executable"):
        if not isinstance(value.get(field), str) or not value[field].strip():
            raise ValueError("Store identity field missing: " + field)
    if not re.fullmatch(r"[A-Za-z0-9.-]{3,50}", value["identityName"]):
        raise ValueError("Invalid Store package name")
    if not value["packageFamilyName"].startswith(value["identityName"] + "_"):
        raise ValueError("Store package family/name mismatch")
    if not isinstance(value.get("versionMajorOffset"), int) or value["versionMajorOffset"] < 0:
        raise ValueError("Invalid MSIX version major offset")
    return value


def package_version(version, config=None):
    """X.Y.Z app version → (X + offset).Y.Z.0 (Store: revision 0, major >= 1)."""
    config = config or identity()
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("MSIX needs a numeric X.Y.Z version")
    fields = list(map(int, version.split(".")))
    fields[0] += config["versionMajorOffset"]
    if not 1 <= fields[0] <= 65535 or any(v > 65535 for v in fields[1:]):
        raise ValueError("MSIX version outside Microsoft Store bounds")
    return ".".join(map(str, fields + [0]))


def msi_version(version):
    """MSI ProductVersion for app version X.Y.Z: X.(Y // 256).(Y % 256 * 256 + Z).

    Windows Installer allows major and minor <= 255 and build <= 65535; the app's minor
    counts feature releases and outgrows 255, so it spills into the build field. The mapping
    keeps the order of app versions (patch Z < 256) and leaves 0.0.Z as it was.
    """
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("MSI needs a numeric X.Y.Z version")
    major, minor, patch = map(int, version.split("."))
    if major > 255 or minor > 255 * 256 + 255 or patch > 255:
        raise ValueError("MSI version outside Windows Installer bounds")
    return f"{major}.{minor // 256}.{minor % 256 * 256 + patch}"


def manifest(version, arch, config=None):
    if arch not in ("x64", "arm64"):
        raise ValueError("Unsupported MSIX architecture")
    config = config or identity()

    def add(parent, tag, text=None, **attributes):
        prefix, local = tag.split(":", 1) if ":" in tag else ("", tag)
        element = ET.SubElement(parent, "{" + NS[prefix] + "}" + local, attributes)
        element.text = text
        return element

    root = ET.Element("{" + NS[""] + "}Package", IgnorableNamespaces="uap rescap")
    add(root, "Identity", Name=config["identityName"], Publisher=config["publisher"],
        Version=package_version(version, config), ProcessorArchitecture=arch)
    properties = add(root, "Properties")
    add(properties, "DisplayName", config["displayName"])
    add(properties, "PublisherDisplayName", config["publisherDisplayName"])
    add(properties, "Description", DESCRIPTION)
    add(properties, "Logo", "Assets\\StoreLogo.png")
    add(add(root, "Dependencies"), "TargetDeviceFamily", Name="Windows.Desktop",
        MinVersion="10.0.17763.0", MaxVersionTested="10.0.22621.0")
    add(add(root, "Resources"), "Resource", Language="en-us")
    application = add(add(root, "Applications"), "Application", Id="Ascension",
                      Executable=config["executable"], EntryPoint="Windows.FullTrustApplication")
    add(application, "uap:VisualElements", DisplayName=config["displayName"], Description=DESCRIPTION,
        BackgroundColor="transparent", Square150x150Logo="Assets\\Square150x150Logo.png",
        Square44x44Logo="Assets\\Square44x44Logo.png")
    capabilities = add(root, "Capabilities")
    # Network play: join a server, or host a match for others.
    add(capabilities, "Capability", Name="internetClientServer")
    add(capabilities, "Capability", Name="privateNetworkClientServer")
    add(capabilities, "rescap:Capability", Name="runFullTrust")
    ET.indent(root)
    return '<?xml version="1.0" encoding="utf-8"?>\n' + ET.tostring(root, encoding="unicode") + "\n"


def msix_name(version, arch):
    return f"{common.APP}-{version}-windows-{arch}.msix"


def msi_name(version, arch):
    return f"{common.APP}-{version}-windows-{arch}.msi"


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--version", default=common.app_version())
    ap.add_argument("--arch", default="x64")
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--msi-version", action="store_true", help="print the MSI ProductVersion instead")
    a = ap.parse_args()
    if a.msi_version:
        print(msi_version(a.version))
        return
    a.output.write_text(manifest(a.version, a.arch), encoding="utf-8")
    print(a.output)


if __name__ == "__main__":
    main()
