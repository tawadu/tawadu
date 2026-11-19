#!/usr/bin/env python3
"""Re-verify the v1 starter athan assets against LIVE Wikimedia Commons bytes.

Sentinel's licence-audit condition (2026-11-19): Wayback size-match is not
content-match; asset commits must carry live-byte provenance. Run this script
in the asset PR (or CI) and paste its output into the PR description.

Stdlib only. Resolves the real file URL via the Commons imageinfo API (never
guess upload.wikimedia.org path segments), downloads, and compares SHA-256
and byte count against the ledger in docs/audio-sources.md.
"""
import hashlib
import json
import sys
import urllib.parse
import urllib.request

UA = {
    "User-Agent": "TawaduLicenceAudit/1.0 (https://github.com/tawadu/tawadu; "
    "open-source prayer app licence verification)"
}

# (commons title, expected sha256, expected bytes) — docs/audio-sources.md Tier A
ASSETS = [
    ("File:Beautiful_adhan.ogg",
     "35fe06b08fe80505c550c33fed8a783fa9901ddc81ac884958b4be048f5b2a79", 1229032),
    ("File:Adhan.ogg",
     "faeb03e4338554fb8b54dba98ed4949648615b58e991bbd560a89f4837dde413", 338741),
]


def main() -> int:
    failed = False
    for title, want_sha, want_size in ASSETS:
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
            ok = got == want_sha and len(data) == want_size
            print(f"{title}: {len(data)} bytes, sha256 {got} -> "
                  f"{'MATCH' if ok else 'MISMATCH'}")
            if not ok:
                failed = True
        except Exception as e:  # noqa: BLE001 - report and fail, never guess
            print(f"{title}: ERROR {e}")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
