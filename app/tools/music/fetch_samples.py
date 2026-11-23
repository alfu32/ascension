#!/usr/bin/env python3
"""Download the sample files the renderer uses and check them against samples.lock.

    python3 fetch_samples.py <samples-dir>                 fetch what is missing, verify everything
    python3 fetch_samples.py <samples-dir> --update-lock   rebuild samples.lock from instruments.py

Files come from the libraries' official repositories at the commits pinned in instruments.py,
one file at a time (only what the instruments use, about 1.7 GB rather than the whole 6 GB).
Every file is checked against the SHA-256 in samples.lock. --update-lock reads the repositories'
file trees, checks each download against its git blob hash, and writes the lock anew.
Standard library only.
"""
import concurrent.futures
import hashlib
import json
import os
import re
import sys
import urllib.parse
import urllib.request

import instruments as ins
import sfz

HERE = os.path.dirname(os.path.abspath(__file__))
LOCK = os.path.join(HERE, "samples.lock")
SOURCES = {"vsco": (ins.VSCO_REPO, ins.VSCO_COMMIT), "vcsl": (ins.VCSL_REPO, ins.VCSL_COMMIT)}


def url(source, path):
    repo, commit = SOURCES[source]
    return f"https://raw.githubusercontent.com/{repo}/{commit}/{urllib.parse.quote(path)}"


def get(u, attempts=4):
    last = None
    for _ in range(attempts):
        try:
            req = urllib.request.Request(u, headers={"User-Agent": "music-fetch"})
            with urllib.request.urlopen(req, timeout=120) as r:
                return r.read()
        except OSError as e:  # retry network errors
            last = e
    raise RuntimeError(f"cannot download {u}: {last}")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def git_blob_sha1(data):
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def local(dest, source, path):
    return os.path.join(dest, source, path)


def read_lock():
    rows = []
    with open(LOCK) as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            source, size, digest, path = line.rstrip("\n").split("\t")
            rows.append((source, path, int(size), digest))
    return rows


def fetch_one(dest, source, path, size, digest):
    p = local(dest, source, path)
    if os.path.exists(p) and os.path.getsize(p) == size:
        with open(p, "rb") as f:
            if sha256(f.read()) == digest:
                return "ok"
    data = get(url(source, path))
    if len(data) != size or sha256(data) != digest:
        raise RuntimeError(f"checksum mismatch: {source}/{path}")
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p + ".part", "wb") as f:
        f.write(data)
    os.replace(p + ".part", p)
    return "fetched"


def fetch(dest):
    rows = read_lock()
    done = {"ok": 0, "fetched": 0}
    with concurrent.futures.ThreadPoolExecutor(6) as pool:
        futs = [pool.submit(fetch_one, dest, *r) for r in rows]
        for i, fut in enumerate(concurrent.futures.as_completed(futs), 1):
            done[fut.result()] += 1
            if i % 200 == 0:
                print(f"  {i}/{len(rows)}", flush=True)
    total = sum(r[2] for r in rows)
    print(f"{len(rows)} files ({total / 1e6:.0f} MB): {done['ok']} already present, "
          f"{done['fetched']} downloaded, all checksums match")
    if not os.path.exists(ins.FLUID_SF2):
        print(f"note: {ins.FLUID_SF2} is missing (Debian/Ubuntu package fluid-soundfont-gm)")
    else:
        with open(ins.FLUID_SF2, "rb") as f:
            ok = sha256(f.read()) == ins.FLUID_SHA256
        print(f"FluidR3_GM.sf2 {'matches' if ok else 'DOES NOT MATCH'} its checksum")
        if not ok:
            sys.exit(1)


def tree(source):
    repo, commit = SOURCES[source]
    u = f"https://api.github.com/repos/{repo}/git/trees/{commit}?recursive=1"
    t = json.loads(get(u))
    assert not t["truncated"]
    return {e["path"]: e for e in t["tree"] if e["type"] == "blob"}


def update_lock(dest):
    wanted = []  # (source, path)
    vsco = tree("vsco")
    for patch, arts in sorted(ins.VSCO_PATCHES.items()):
        for name in sorted(set(arts.values())):
            wanted.append(("vsco", name))
            data = get(url("vsco", name))
            assert git_blob_sha1(data) == vsco[name]["sha"], name
            for region in sfz.parse(data.decode("latin-1")):
                if patch == "perc" and sfz.key_range(region)[0] not in ins.PERC_KEYS:
                    continue
                wanted.append(("vsco", sfz.sample_path(region)))
    vcsl = tree("vcsl")
    for patch, spec in sorted(ins.VCSL_PATCHES.items()):
        pat = re.compile(spec["pattern"])
        for path in sorted(vcsl):
            folder, base = os.path.split(path)
            if folder == spec["dir"] and pat.fullmatch(base):
                wanted.append(("vcsl", path))
    wanted = sorted(set(wanted))
    trees = {"vsco": vsco, "vcsl": vcsl}

    def one(item):
        source, path = item
        p = local(dest, source, path)
        if os.path.exists(p):
            with open(p, "rb") as f:
                data = f.read()
            if git_blob_sha1(data) != trees[source][path]["sha"]:
                data = None
        else:
            data = None
        if data is None:
            data = get(url(source, path))
            if git_blob_sha1(data) != trees[source][path]["sha"]:
                raise RuntimeError(f"git hash mismatch: {source}/{path}")
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "wb") as f:
                f.write(data)
        return source, len(data), sha256(data), path

    with concurrent.futures.ThreadPoolExecutor(6) as pool:
        rows = list(pool.map(one, wanted))
    with open(LOCK, "w") as f:
        f.write("# source\tbytes\tsha256\tpath   (written by fetch_samples.py --update-lock)\n")
        for source, size, digest, path in rows:
            f.write(f"{source}\t{size}\t{digest}\t{path}\n")
    print(f"{len(rows)} files, {sum(r[1] for r in rows) / 1e6:.0f} MB; samples.lock written")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    if "--update-lock" in sys.argv[2:]:
        update_lock(sys.argv[1])
    else:
        fetch(sys.argv[1])
