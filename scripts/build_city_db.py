#!/usr/bin/env python3
"""Build the derived GeoNames city DB (Phase 5 slice 1, binding §8.1–8.9).

Pipeline: download cities1000.zip -> verify LIVE bytes against the ledger
hash (rotation is recorded, not rejected) -> parse the 19-column geoname
order (timezone = column 18, 1-indexed) -> apply the CURATED 28-city
display overrides parsed from locations.rs (Arabic display names are
curated, never mined; GeoNames Arabic script = search aliases only) ->
derived SQLite with build-time normalized keys + indexes (no runtime
index build) -> record raw AND derived SHA-256.

The raw zip NEVER enters the repo (workdir only). Stdlib only. Re-runnable
end-to-end: same input bytes + same sqlite -> same derived DB.

Usage:
  python3 scripts/build_city_db.py                 # download + verify + build
  python3 scripts/build_city_db.py --accept-rotation   # after a daily dump
                                                       # rotation: record the
                                                       # new hash/Last-Modified
                                                       # in the ledger + licence
                                                       # entry IN THE SAME PR
"""
import argparse
import hashlib
import os
import re
import sqlite3
import sys
import urllib.request
import zipfile

URL = "https://download.geonames.org/export/dump/cities1000.zip"
# Ledger docs/phase5-city-sources.md §1.4 / audit §2 (2026-11-19 live bytes).
EXPECTED_SHA = "1dcfe6ea1731529fe53d54c8c524add5a8f4b198cff7ff0ab5e5b774d528db17"
EXPECTED_BYTES = 11_101_196
UA = {"User-Agent": "TawaduLicenceAudit/1.0 (https://github.com/tawadu/tawadu)"}

ARABIC_DIACRITICS = set(range(0x064B, 0x0660)) | {0x0670, 0x0640}
ARABIC_UNIFY = {"أ": "ا", "إ": "ا", "آ": "ا", "ى": "ي", "ة": "ه"}


def normalize(s):
    """MUST mirror tawadu-core/src/cities.rs::normalize exactly (search only)."""
    out = []
    for ch in s:
        if ord(ch) in ARABIC_DIACRITICS:
            continue
        ch = ARABIC_UNIFY.get(ch) or ch
        out.append(ch.lower())
    return "".join(out)


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_curated(repo_root):
    """The 28 curated display names, parsed from locations.rs (single source
    of truth — no transcription drift). Binding §8.5 / Atlas §10.5."""
    src = open(os.path.join(repo_root, "crates/tawadu-core/src/locations.rs"),
               encoding="utf-8").read()
    start = src.index("pub const CITIES")
    src = src[start:src.index("];", start)]
    pat = re.compile(
        r'City \{\s*name: "([^"]+)",\s*name_ar: "([^"]+)",\s*'
        r'latitude: (-?[\d.]+),\s*longitude: (-?[\d.]+),\s*'
        r'timezone: "([^"]+)",\s*country: "([^"]+)",\s*\}')
    cities = [dict(name=m[0], name_ar=m[1], latitude=float(m[2]),
                   longitude=float(m[3]), timezone=m[4], country=m[5])
              for m in pat.findall(src)]
    if len(cities) != 28:
        sys.exit(f"FATAL: expected 28 curated cities, parsed {len(cities)}")
    return cities


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--workdir", default=os.environ.get("TMPDIR", "."))
    ap.add_argument("--out", default=None)
    ap.add_argument("--accept-rotation", action="store_true")
    args = ap.parse_args()
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    out = args.out or os.path.join(repo_root, "assets", "cities.db")
    os.makedirs(args.workdir, exist_ok=True)
    zip_path = os.path.join(args.workdir, "cities1000.zip")

    print(f"downloading {URL} ...", flush=True)
    req = urllib.request.Request(URL, headers=UA)
    with urllib.request.urlopen(req, timeout=300) as r:
        data = r.read()
        last_modified = r.headers.get("Last-Modified", "?")
        etag = r.headers.get("ETag", "?")
    with open(zip_path, "wb") as f:
        f.write(data)
    raw_sha = sha256(data)
    print(f"raw: {len(data)} bytes sha256={raw_sha}")
    print(f"Last-Modified: {last_modified}  ETag: {etag}")
    if raw_sha != EXPECTED_SHA or len(data) != EXPECTED_BYTES:
        print("ROTATION DETECTED: live bytes differ from the ledger pin.")
        print(f"  ledger: {EXPECTED_BYTES} B {EXPECTED_SHA}")
        print(f"  live:   {len(data)} B {raw_sha} (Last-Modified {last_modified})")
        if not args.accept_rotation:
            print("Re-run with --accept-rotation AND update docs/phase5-city-sources.md"
                  " + the licence entry with the new hash/Last-Modified IN THE SAME PR.")
            return 2
        print("--accept-rotation: proceeding (ledger + licence MUST be updated).")

    with zipfile.ZipFile(zip_path) as z:
        inner = [n for n in z.namelist() if n.endswith("cities1000.txt")]
        if len(inner) != 1:
            sys.exit(f"FATAL: unexpected zip contents {z.namelist()}")
        text = z.read(inner[0]).decode("utf-8")

    rows = []
    for i, line in enumerate(text.splitlines()):
        if not line:
            continue
        f = line.split("\t")
        if len(f) != 19:
            sys.exit(f"FATAL: row {i} has {len(f)} fields, expected 19 (geoname order)")
        rows.append(f)
    print(f"records: {len(rows)} (19 columns each)")

    curated = parse_curated(repo_root)
    # Match curated cities by (asciiname|name, country); duplicates resolved
    # by proximity to the curated coordinates — reported, never silent.
    overrides = {}
    for c in curated:
        cands = [r for r in rows
                 if r[8] == c["country"]
                 and (r[2] == c["name"] or r[1] == c["name"]
                      or c["name"] in r[3].split(","))]
        if not cands:
            sys.exit(f"FATAL: curated city {c['name']} ({c['country']}) not in dump")
        best = min(cands, key=lambda r: (float(r[4]) - c["latitude"]) ** 2
                   + (float(r[5]) - c["longitude"]) ** 2)
        if len(cands) > 1:
            print(f"curated match: {c['name']} -> geonameid {best[0]} "
                  f"({len(cands)} candidates, nearest coordinates)")
        overrides[best[0]] = c
    print(f"curated overrides matched: {len(overrides)}/28")

    if os.path.exists(out):
        os.remove(out)
    conn = sqlite3.connect(out)
    conn.executescript("""
        CREATE TABLE cities (
            geonameid INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            name_ar TEXT NOT NULL DEFAULT '',
            name_norm TEXT NOT NULL,
            ascii_norm TEXT NOT NULL,
            latitude REAL NOT NULL,
            longitude REAL NOT NULL,
            country TEXT NOT NULL,
            admin1 TEXT NOT NULL,
            population INTEGER NOT NULL,
            timezone TEXT NOT NULL
        );
        -- Index-organized alias keys: the PK IS the prefix-search index
        -- (no rowid + no duplicate index = ~half the size).
        CREATE TABLE aliases (
            alias_norm TEXT NOT NULL,
            geonameid INTEGER NOT NULL,
            PRIMARY KEY (alias_norm, geonameid)
        ) WITHOUT ROWID;
    """)
    tz_set = set()
    arabic_rows = 0
    alias_count = 0
    for f in rows:
        gid = int(f[0])
        cur = overrides.get(f[0])
        name = cur["name"] if cur else f[1]
        name_ar = cur["name_ar"] if cur else ""
        alternates = [a for a in f[3].split(",") if a]
        if any(any("\u0600" <= ch <= "\u06FF" for ch in a) for a in alternates):
            arabic_rows += 1
        name_norm = normalize(name)
        ascii_norm = normalize(f[2])
        keys = list(dict.fromkeys(normalize(a) for a in alternates if normalize(a)))
        if name_ar:
            k = normalize(name_ar)
            if k and k not in keys:
                keys.append(k)
        # Keys already covered by the cities indexes are redundant here.
        keys = [k for k in keys if k != name_norm and k != ascii_norm]
        conn.execute("INSERT INTO cities VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                     (gid, name, name_ar, name_norm, ascii_norm,
                      float(f[4]), float(f[5]), f[8], f[10],
                      int(f[14]), f[17]))
        conn.executemany("INSERT OR IGNORE INTO aliases VALUES (?,?)",
                         [(k, gid) for k in keys])
        alias_count += len(keys)
        tz_set.add(f[17])
    conn.executescript("""
        CREATE INDEX idx_cities_name_norm ON cities(name_norm);
        CREATE INDEX idx_cities_ascii_norm ON cities(ascii_norm);
        CREATE INDEX idx_cities_pop ON cities(population DESC);
    """)
    conn.commit()
    conn.execute("VACUUM")
    conn.close()

    derived = open(out, "rb").read()
    print(f"derived: {out}")
    print(f"derived: {len(derived)} bytes sha256={sha256(derived)}")
    print(f"aliases: {alias_count} keys; rows with Arabic-script alternate: {arabic_rows}")
    print(f"distinct timezones: {len(tz_set)} (Rust gate test asserts all parse)")
    print(f"sqlite version: {sqlite3.sqlite_version} (determinism note: same input"
          " + same sqlite -> same bytes)")
    print("PROVENANCE BLOCK for the licence entry:")
    print(f"  raw:     {len(data)} B sha256={raw_sha} Last-Modified={last_modified}")
    print(f"  derived: {len(derived)} B sha256={sha256(derived)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
