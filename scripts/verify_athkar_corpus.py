#!/usr/bin/env python3
"""Re-verify the bundled athkar corpus against the PINNED upstream bytes.

Sentinel's Phase 4 binding condition: per-file SHA-256 + a reproducible
provenance script (pattern: scripts/verify_audio_sources.py). The corpus is
the MIT-licensed Hisn al-Muslim TEXT data from YousefAsalya/Islamic-Pro-azkar-API
at commit d793023e3b69c91674023f692425950c8ebbb64d (pinned in docs/audio-sources.md).
Recitation AUDIO is deliberately NOT bundled (separate performance-rights check).

Two modes:
  python3 scripts/verify_athkar_corpus.py            # offline: bundled files vs ledger
  python3 scripts/verify_athkar_corpus.py --live     # also re-download the pinned bytes

Stdlib only. Exit 0 = every file matches its recorded SHA-256 and byte count.
"""
import hashlib
import sys
import urllib.request

PIN = "d793023e3b69c91674023f692425950c8ebbb64d"
BASE = f"https://raw.githubusercontent.com/YousefAsalya/Islamic-Pro-azkar-API/{PIN}"
UA = {"User-Agent": "TawaduLicenceAudit/1.0 (https://github.com/tawadu/tawadu)"}

# (local path, upstream path, expected sha256, expected bytes)
LEDGER = [
    ("assets/athkar/ar.json", "data/ar.json",
     "7affdd9c1356f66f77057b631ac239cc41bde68f9bdc85d344a852a0a6ff6732", 166566),
    ("assets/athkar/en.json", "data/en.json",
     "49d155762b9eac1011127a3afabd4a4094206138b7c161f587b29a473f59f3f1", 145287),
    ("assets/licenses/azkar-api-mit.txt", "LICENSE",
     "d6930e8db4f791647d10abb43c218d10b6f6fb5fd9905aab2602ce41dd1d33ba", 1093),
]


def check(label, data, want_sha, want_size):
    sha = hashlib.sha256(data).hexdigest()
    ok = sha == want_sha and len(data) == want_size
    print(f"{'OK ' if ok else 'FAIL'} {label}: {len(data)} bytes sha256={sha}")
    return ok


def main():
    live = "--live" in sys.argv
    failed = False
    for local, upstream, want_sha, want_size in LEDGER:
        with open(local, "rb") as f:
            bundled = f.read()
        failed |= not check(f"bundled {local}", bundled, want_sha, want_size)
        if live:
            req = urllib.request.Request(f"{BASE}/{upstream}", headers=UA)
            with urllib.request.urlopen(req, timeout=60) as r:
                remote = r.read()
            failed |= not check(f"live {upstream}@{PIN[:8]}", remote, want_sha, want_size)
            failed |= bundled != remote
            if bundled != remote:
                print(f"FAIL {local}: bundled bytes differ from pinned upstream")
    print("PROVENANCE:", "FAIL" if failed else "PASS (pinned commit " + PIN + ")")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
