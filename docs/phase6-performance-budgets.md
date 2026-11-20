# Phase 6 — performance budgets (measured)

Companion to [`ROADMAP.md`](ROADMAP.md) Phase 6 task 4 (*"Measure the budgets …
Enforce with a benchmark, not a claim"*) and gap #9 (*"Performance budgets
unmeasured"*). Phase 5 routed two UI-thread costs here as **N10** and **N11**;
this document records what they actually cost, the design that closes them, and
how the budget gets enforced.

---

## 1. Budgets

| # | Budget | Target | Status |
|---|---|---|---|
| B1 | idle CPU | ≈ 0 % | not measured |
| B2 | memory | < 120 MB | not measured |
| B3 | cold start | < 1.5 s | not measured |
| B4 | athan latency | < 300 ms | not measured |
| **B5** | **city search on the UI thread** | **p95 < 16 ms, no visible hitch** | **closed — PR #10** |
| **B6** | **mosque profile writes** | **no commit per keystroke; ≤ 1 write per pause** | **closed — PR #11** |

B1–B4 are Phase 6 task 4's original list and are still open. B5/B6 are the two
costs Phase 5 found; both now have a measured baseline and a merged fix.

- **B5 → PR #10** (`07b87cc`): literal LIKE escaping, one `IN (…)` per 500-id
  chunk instead of one query per id, and the whole search moved to a
  `city-search` worker thread. The broad-query cost fell hard (`al` 17.8 →
  1.3 ms) and the residual ~70 ms substring scan no longer runs on the UI
  thread.
- **B6 → PR #11**: the three setters mutate memory immediately and the write
  waits for an 800 ms pause (drained on the 200 ms tick), with unconditional
  flushes at quit / profile switch / remove. `persist_mosque_edit` still
  decides at flush time, so the F4 invariants are untouched.

---

## 2. N10 — city search

### Method

Probe scripts `scratch/n10_probe.py`, `scratch/n10_probe2.py` (timings) against
`assets/cities.db` — 172,046 records, 807,552 aliases, the exact bundle the app
ships. The probe reproduces `CityDb::search` step for step (prefix range scans →
substring fallback when the prefix pass is sparse → per-id lookup loop).

**Caveat:** the probe runs Python's `sqlite3` (SQLite **3.53.1**); the app bundles
SQLite **3.46.0** via `rusqlite` 0.32.1 (`bundled` feature). Treat the numbers as
magnitudes, not as the app's exact figures — the shapes and ratios hold.

### Measured

| query | total | hits | note |
|---|---|---|---|
| `london` | 0.9 ms | 23 | prefix pass ≥ 10 ids → **no** substring scan |
| `riyadh` | **73.8 ms** | 4 | prefix pass 4 ids < 10 → substring scan runs |
| `al` | 17.8 ms | 50 | substring scan |
| `%` | 24.7 ms | 50 | **unescaped wildcard** — matches everything |
| `_` | 23.7 ms | 50 | **unescaped wildcard** — matches everything |
| *(empty)* | 1.5 ms | 50 | default populous list |

Breakdown of the `riyadh` case: prefix range scans 0.1 ms + 0.1 ms; substring
fallback **12.0 ms** (`cities.name_norm`) + **52.4 ms** (`aliases.alias_norm`).

The per-id lookup loop alone, for a sparse query yielding 572 ids:
**16.3 ms** as 572 individual `SELECT … WHERE geonameid = ?` calls, vs
**0.8 ms** for one `IN (…)` query — **≈ 20×**.

### Findings

1. **Search is already debounced.** A 150 ms `Timer` (Prism §3) drains a pending
   query, so the scan runs once per pause, not per keystroke. The "~110 ms per
   keystroke" framing in the routing note is stale on that point.
2. **The cost is the substring fallback, and it is the common case.** It runs
   whenever the prefix pass yields fewer than 10 ids — which is most real
   queries, including a top-five city like Riyadh (4 prefix ids). ~74 ms.
3. **The lookup is N+1.** One `query_row` per hit id; ~20× more than a single
   `IN (…)` query at 572 ids.
4. **`normalize()` does not escape LIKE metacharacters.** `%` and `_` reach the
   `LIKE` pattern raw, so they match everything: 400 rows per table, then the
   N+1 loop over them, then the UI shows 50 populous cities that match nothing.
   This is a **correctness** bug (the results are noise), not a perf cliff.
5. **All of it runs on the UI thread** — inside the debounce timer callback — so
   the residual ~74 ms is a visible hitch, and a slow disk makes it worse.

### Design

Implemented in **PR #10** (`07b87cc`):

- **(a) Escape the substring pattern.** ✅ `like_literal()` escapes `%`, `_`
  and the escape character; the fallback carries `ESCAPE '\'`. A unit test
  pins `%`, `_` and `\` as literals. Prism §3's prefix + substring behaviour
  is unchanged.
- **(b) Collapse the N+1.** ✅ one `IN (…)` per 500-id chunk through a shared
  `hit_from_row` mapper (which also de-duplicates `by_id`). Measured 16.3 ms →
  0.8 ms at 572 ids.
- **(c) Move the search off the UI thread.** ✅ `CitySearchWorker` owns a
  private read-only connection on a `city-search` thread; queries and results
  carry a sequence number so a slow older search can never overwrite a newer
  one. The 150 ms debounce is unchanged.

---

## 3. N11 — mosque profile writes

### Method

`scratch/n11_probe.py` against a **copy** of the live
`%APPDATA%/tawadu/tawadu.db`, running the app's exact UPSERT
(`save_mosque`, `storage.rs:246`) with a commit per statement — which is what a
keystroke does.

### Measured

The app sets no `PRAGMA`, so the connection runs SQLite's defaults:
`journal_mode = delete`, `synchronous = 2` (**FULL**).

| configuration | p50 | max |
|---|---|---|
| as shipped — DELETE journal, `synchronous=FULL`, commit per keystroke | **4.9 ms** | **10.0 ms** |
| `journal_mode=WAL`, `synchronous=NORMAL`, commit per keystroke | 1.6 ms | 1.9 ms |
| same 200 writes, **one** commit (debounced) | **4.35 ms total** | — |

Commit-per-keystroke is ~**225×** more expensive than one commit per pause.

### Findings

1. `edited` fires per keystroke on the mosque name fields (`ui/main.slint`), and
   each one funnels through `persist_mosque_edit` → `save_mosque` — a synchronous
   UPSERT **with a commit**, on the UI thread. ~5 ms p50 / 10 ms max per
   character.
2. With the default rollback journal and `synchronous=FULL`, every commit fsyncs
   the journal **and** the database file. On a slow or contended disk that is a
   multi-millisecond stall per character.
3. The write is already correctly gated (F4): `persist_mosque_edit` is the single
   non-test `save_mosque` call site, and it never persists an unnamed draft.
   **The defect is timing, not policy.**

### Design

Implemented in **PR #11**:

- **Debounce the persist, not the edit.** ✅ The in-memory mutation stays
  immediate; the *write* waits for an 800 ms pause (the settings surface's
  window) drained on the existing 200 ms tick, plus an unconditional flush at
  quit/hide, profile switch, and just before a remove.
- **Keep the decision in `persist_mosque_edit`.** ✅ Only *when* it runs
  changes, so the F4 invariants are untouched: unnamed never persists,
  create-on-first-name, discard-on-dismiss, activation keyed on the EN name.
  Because the gate re-reads `is_named()` at flush time, a name typed and then
  emptied inside one pause is never created — pinned by a test. That is exactly
  why the single gate was worth building.
- **`PRAGMA journal_mode=WAL` + `synchronous=NORMAL` — measured, not taken.**
  It drops the residual from 4.9 ms to 1.6 ms per commit, but with the
  per-keystroke commit gone there is one commit per pause, where 1.6 ms is
  invisible; it also creates `-wal`/`-shm` files beside the user's DB. Recorded
  here so the omission is a decision, not an oversight.

---

## 4. Enforcing

ROADMAP task 4 requires a benchmark, not a claim. **Still owed:** B5 and B6 are
closed by the fixes above rather than by an assertion, and the harness that
would stop them regressing has not been built. Concretely, when it is:

- a benchmark that runs the real `CityDb::search` over a fixed query set
  (including `%`, `_`, an Arabic name, an empty query) against the **bundled**
  `assets/cities.db` and asserts p95 < 16 ms;
- a write-burst test that types a 20-character name and asserts **≤ 1** commit
  per pause, plus the F4 invariants (no row for an unnamed draft).

Both belong with the B1–B4 harness.

---

## 5. Not in this document

- B1–B4 (idle CPU, memory, cold start, athan latency) — Phase 6 task 4, unmeasured.
- N8d (accessible labels) — a separate accessibility track, unrelated to budget.
