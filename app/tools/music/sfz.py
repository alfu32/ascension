"""A small reader for the SFZ files of the VSCO 2 library (only the opcodes those files use)."""
import re

_HEADER = re.compile(r"<(control|global|group|region)>")
_OPCODE = re.compile(r"([A-Za-z_]+)=")


def parse(text):
    """Return the regions of an SFZ text as dicts of opcodes, with headers merged in."""
    text = re.sub(r"//[^\n]*", "", text)
    scopes = {"control": {}, "global": {}, "group": {}}
    regions = []
    parts = _HEADER.split(text)
    current = None
    for chunk in parts:
        if chunk in ("control", "global", "group", "region"):
            current = chunk
            if current == "group":
                scopes["group"] = {}
            elif current == "global":
                scopes["global"] = {}
            continue
        if current is None:
            continue
        ops = _opcodes(chunk)
        if current == "region":
            merged = {}
            for scope in ("control", "global", "group"):
                merged.update(scopes[scope])
            merged.update(ops)
            regions.append(merged)
        else:
            scopes[current].update(ops)
    return regions


def _opcodes(chunk):
    ops = {}
    found = list(_OPCODE.finditer(chunk))
    for i, m in enumerate(found):
        end = found[i + 1].start() if i + 1 < len(found) else len(chunk)
        ops[m.group(1)] = chunk[m.end():end].strip()
    return ops


def sample_path(region):
    """The sample's path relative to the library root, with forward slashes."""
    return (region.get("default_path", "") + region["sample"]).replace("\\", "/")


def key_range(region):
    if "key" in region:
        k = int(region["key"])
        return k, k, k
    lo, hi = int(region.get("lokey", 0)), int(region.get("hikey", 127))
    return lo, hi, int(region.get("pitch_keycenter", lo))
