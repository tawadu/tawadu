#!/usr/bin/env python3
"""Fetch the v1 starter athan assets into assets/voices/ (download provenance).

Companion to scripts/verify_audio_sources.py (Sentinel's licence-audit
condition): resolves the real file URL via the Commons imageinfo API — never
guesses upload.wikimedia.org path segments — downloads, verifies SHA-256 and
byte count against docs/audio-sources.md, and only then writes the file into
the pack layout. Stdlib only; offline-first app, online fetch tool.

Run from the repo root:  python scripts/fetch_starter_assets.py
"""
import hashlib
import json
import pathlib
import sys
import urllib.parse
import urllib.request

UA = {
    "User-Agent": "TawaduLicenceAudit/1.0 (https://github.com/tawadu/tawadu; "
    "open-source prayer app licence verification)"
}

ROOT = pathlib.Path(__file__).resolve().parent.parent

# (commons title, expected sha256, expected bytes, destination relative path)
ASSETS = [
    ("File:Beautiful_adhan.ogg",
     "35fe06b08fe80505c550c33fed8a783fa9901ddc81ac884958b4be048f5b2a79", 1229032,
     "assets/voices/starter-beautiful-adhan/athan/beautiful_adhan.ogg"),
    ("File:Adhan.ogg",
     "faeb03e4338554fb8b54dba98ed4949648615b58e991bbd560a89f4837dde413", 338741,
     "assets/voices/starter-adhan-announcement/athan/adhan.ogg"),
]


def main() -> int:
    failed = False
    for title, want_sha, want_size, dest in ASSETS:
        api = ("https://commons.wikimedia.org/w/api.php?action=query&titles="
               + urllib.parse.quote(title)
               + "&prop=imageinfo&iiprop=url|size&format=json")
        try:
            with urllib.request.urlopen(
                    urllib.request.Request(api, headers=UA), timeout=30) as r:
                d = json.load(r)
            page = next(iter(d["query"]["pages"].values()))
            info = page["imageinfo"][0]
            with urllib.request.urlopen(
                    urllib.request.Request(info["url"], headers=UA),
                    timeout=120) as r:
                data = r.read()
            got = hashlib.sha256(data).hexdigest()
            if got != want_sha or len(data) != want_size:
                print(f"{title}: MISMATCH ({len(data)} bytes, sha256 {got}) "
                      f"— NOT written")
                failed = True
                continue
            path = ROOT / dest
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            print(f"{title}: {len(data)} bytes, sha256 {got} -> MATCH; "
                  f"written {dest} (from {info['url']})")
        except Exception as e:  # noqa: BLE001 - report and fail, never guess
            print(f"{title}: ERROR {e}")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())