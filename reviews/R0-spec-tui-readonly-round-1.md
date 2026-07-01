# R0 Architect Review — `SPEC_tui_readonly_viewer.md` (btctax-tui) — Round 1

**Artifact:** `design/SPEC_tui_readonly_viewer.md`
**Baseline verified against:** current source @ HEAD `30570e0` (spec's stated baseline).
**Reviewer role:** independent architect (author ≠ reviewer).
**Gate:** R0, must reach 0 Critical / 0 Important before implementation.

## Verdict

**NOT 0C/0I — gate is OPEN.** 0 Critical, **2 Important**, 6 Minor, 3 Nit.

The design is genuinely read-only and offline as specified — there is **no live mutation
hole and no network dependency** in the paths it relies on. The two Important findings are
about the *airtightness of the enforcement mechanisms* on the security-sensitive surface
(the read-only-guarantee discipline, and the passphrase-buffer zeroization mechanism); both
are cheap, one-line-of-design fixes but must land before implementation so the guarantees are
provably tight at this security gate.

**MSRV headline (Q4): NOT a blocker.** ratatui 0.29.0 declares `rust-version = "1.74.0"`
(verified from the `v0.29.0` tag `Cargo.toml` and crates.io); crossterm 0.28.1 MSRV is
1.63.0. Both compile on Rust 1.74. The pin is valid. Caveats below (M5).

---

## Recon citation verification (all confirmed against HEAD 30570e0)

| Spec claim | Source | Status |
|---|---|---|
| `btctax-cli` is a lib; `Session`+`CliError` `pub` | `cli/lib.rs:5-14`, `[lib] name="btctax_cli"` in `cli/Cargo.toml` | ✅ |
| `Session::open(&Path,&Passphrase)->Result<Session,CliError>`, no save | `cli/session.rs:52-56` | ✅ non-mutating |
| `load_events_and_project()->(Vec<LedgerEvent>,LedgerState,ProjectionConfig)` | `cli/session.rs:114-122` | ✅ pure (load_all + project) |
| `all_tax_profiles()->BTreeMap<i32,TaxProfile>` | `cli/session.rs:86-90` | ✅ |
| `optimize_attested_set()->BTreeSet<EventId>` | `cli/session.rs:94-98` | ✅ (but unused — see M3) |
| `config()` | `cli/session.rs:75-77` | ⚠ returns `Result<CliConfig,CliError>`, not `CliConfig` (trivial) |
| `save()` is the ONLY mutator | `cli/session.rs:64-67` (`&mut self`) | ✅ — also `create`/`repair`/`from_fresh_vault` save, but TUI uses only `open` |
| `Passphrase::new(String)`, zeroizes on Drop | `store/crypto.rs:17-30` | ✅ `Passphrase(String)` + `Drop::zeroize` |
| `Vault::open` holds `VaultLock`; `StoreError::Locked` | `store/vault.rs:116-120`, `store/lib.rs:23` | ✅; `CliError::Store(#[from])` wraps it (`cli/lib.rs:18-19`) |
| `WrongPassphrase`/`HalfCreatedVault`/`Io` variants | `store/lib.rs:25,35-37,19` | ✅ |
| `render::build_verify` `pub` | `cli/render.rs:465` | ✅ |
| `render::{wallet_label, filing_status_tag}` `pub` | `cli/render.rs:180, 357` | ✅ |
| `term_tag/basis_source_tag/income_kind_tag/dispose_kind_tag/compliance_status_tag` private | `cli/render.rs:86,40,60,53,138` | ✅ all private (re-impl locally is correct) |
| `compute_tax_year(events,&state,year,Option<&TaxProfile>,&dyn TaxTables)->TaxOutcome` | `core/tax/compute.rs:229-235` | ✅ pure |
| `compute_se_tax(&state,year,FilingStatus,&TaxTable)->Option<SeTaxResult>` | `core/tax/se.rs:80-85` | ✅ pure (note: single `&TaxTable`, not the tables collection) |
| `forms::{form_8949,schedule_d,form_8283}(&state,year)` | `core/forms.rs:97,170,299` | ✅ pure |
| `disposal_compliance(&events,&state)` | `core/project/compliance.rs:91` (doc: "Read-only … pure function") | ✅ |
| `conservation_report(&state)` | `core/project/conservation.rs:37` | ✅ |
| `BlockerKind::severity()->Severity` (Hard/Advisory) | `core/state.rs:62` | ✅ |
| `LedgerState.{lots,holdings_by_wallet,disposals,income_recognized,pending_reconciliation,blockers}` | `core/state.rs` struct | ✅ all `pub` (also `removals`,`stats`) |
| `BundledTaxTables::load()` compiled-in | `adapters/tax_tables.rs:49` (const `ty2025()`), re-exported `adapters/lib.rs:19` | ✅ no fs/net |
| `BundledPrices::load()` compiled-in | `adapters/price.rs:20` via `include_str!("../data/…csv")` (line 10) | ✅ no fs/net |
| Workspace: resolver 2, MSRV 1.74, edition 2021 | `Cargo.toml` | ✅ (but no `[workspace.dependencies]` — see M1) |

**Drift found:** only three, all Minor/trivial — `config()` returns `Result<_>` (spec wrote
`-> CliConfig`); the "workspace-shared versions" phrase for `rust_decimal/time/thiserror`
(M1); and the Snapshot's field list (M2/M3). No structural drift; the read-only API surface
the spec depends on exists exactly as described.

---

## Findings

### IMPORTANT

#### I1 — The read-only *enforcement* discipline is incomplete: it ignores the `&mut`-gated `save()` lever and the interior-mutability path through `Session::conn()`

The design is genuinely read-only, but the checkable safeguard the spec relies on — the
whole-diff grep for `save(` / `append_` / `cmd::` (spec lines 16, 148) — does **not** fully
cover the mutation surface, and it under-uses the one *compile-time* guarantee available:

1. **`save()` takes `&mut self`** (`session.rs:64`). If the `App` holds the `Session` by an
   **immutable binding** (`session: Session`, never bound `mut`), then `save()` is
   *compile-impossible* to call — a genuine, checkable guarantee. The spec asserts "Rust
   gives no compile enforcement — read methods take `&self`" (line 15), which is true for the
   *readers* but misses that the *mutator* is `&mut`-gated. Leverage it.
2. **`Session::conn() -> &Connection` is `pub`** (`session.rs:59`) and `rusqlite` mutates
   through `&Connection` (interior mutability). So `session.conn().execute("UPDATE …")` — or
   `persistence::append_*(session.conn(), …)` — would mutate the in-memory DB **through an
   immutable `&Session`**, bypassing (1). The grep catches `persistence::append_*` by name,
   but a raw `conn().execute(...)` slips past `{save(, append_, cmd::}` entirely.

   *Mitigating fact (why this is Important, not Critical):* an in-memory mutation is never
   persisted unless `save()` runs (which (1) forbids), and the process holds the exclusive
   `VaultLock`, so a stray `conn()` write cannot corrupt the on-disk vault or leak to another
   reader. The design as written never calls `conn()`. So there is **no live hole today** —
   but the *guarantee mechanism* must be airtight at this gate.

**Exact fix:** (a) require the `App` to own `Session` via a non-`mut` binding (state it in the
spec so `save()` is compile-unreachable); (b) add `conn(` to the whole-diff grep set and add
an explicit rule: **btctax-tui never calls `Session::conn()`** — every read goes through the
high-level Session/core builders. Final grep set: `save(` / `append_` / `cmd::` / `conn(`.
(Note: the grep is scoped to the `btctax-tui` crate only; `session.config()` internally
calling `conn()` inside btctax-cli is fine and out of scope for this grep.)

#### I2 — Passphrase input-buffer zeroization is under-specified and inconsistent with the declared dependency set

Spec line 21: "construct `Passphrase::new(String)` (zeroizes on drop) **and zeroize the input
buffer after**." But the masked field accumulates the passphrase into a TUI-owned buffer
(spec line 122: "character buffer, rendered as `●`"), and the dep list (lines 52-54) does
**not** include `zeroize`. So "zeroize the input buffer" is not implementable as written
without a dep that isn't there — inviting the naive `Passphrase::new(buffer.clone())` +
drop-without-wipe, which leaves a plaintext passphrase copy in freed heap. That undermines the
store's deliberate defense-in-depth (`Passphrase` Drop-zeroize + `SecretBuf` mlock).

**Exact fix — pick one and state it in the spec:**
- **Preferred (no new dep):** move the buffer into the constructor —
  `Passphrase::new(std::mem::take(&mut self.buffer))`. The `String` is moved into `Passphrase`,
  whose `Drop` zeroizes it (`crypto.rs:26-30`); `mem::take` leaves an empty `String` behind.
  Never `clone()` the buffer.
- **Alternative:** add `zeroize` to the crate deps and call `buffer.zeroize()` after
  constructing the `Passphrase`.

Also keep the two hygiene invariants explicit (already implied, keep them): the mask renders
only `●`×len — never the buffer contents — and no error/log/render line ever includes the
passphrase. The `BTCTAX_PASSPHRASE` fast-path is fine (mirrors the CLI); note the env value is
transient and not echoed.

---

### MINOR

#### M1 — "workspace-shared versions" is inaccurate: there is no `[workspace.dependencies]` table
`Cargo.toml` has only `[workspace]` + `[workspace.package]` (edition/license/rust-version).
Each crate pins its own explicit versions (consistently `rust_decimal = "1.36"`,
`time = "0.3"`, `thiserror = "1"`). **Fix:** in `btctax-tui/Cargo.toml`, pin those explicit
versions (matching the other crates); only `edition`/`rust-version`/`license` are
`.workspace = true`. Do **not** write `rust_decimal.workspace = true` — it would fail to
resolve (no such table). Spec's `edition.workspace/rust-version.workspace` for the package
fields is correct.

#### M2 — Snapshot omits `CliConfig`, which the Compliance tab's `build_verify` requires
`build_verify(state, events, cli: &CliConfig)` (`render.rs:465`) needs a **`CliConfig`**; the
CLI's own verify path obtains it via `session.config()?` (`cmd/inspect.rs:30-33`). The spec's
Snapshot enumerates `ProjectionConfig` (from `load_events_and_project`) — which is **not**
`CliConfig`. **Fix:** capture `session.config()?` into the Snapshot at unlock (read-only), and
pass it to `build_verify` in the Compliance tab. (Add `CliConfig` to the Snapshot field list.)

#### M3 — Snapshot loads `optimize_attested_set()` but no read-only tab consumes it
`build_verify` and `disposal_compliance` do **not** take an attested set (verified:
`cmd/inspect.rs:33` calls `build_verify(&state,&events,&cli)`; the attested set is used **only**
by the optimizer — `cmd/optimize.rs:46,180`, which is explicitly out of scope). The
`AttestedRecording` compliance status is derived inside `disposal_compliance` from the event
log, not from this side-table — so Compliance parity with the CLI holds **without** it.
**Fix:** drop `optimize_attested_set()` from the Snapshot (dead fetch), or annotate why it is
retained. Keeps the Snapshot minimal and honest to the read-only scope.

#### M4 — Terminal restore must also cover the run-loop `Err` path, and hook-install ordering
The panic-hook requirement (lines 22-24) is correctly specified for panic + clean quit. Add:
the restore must **also** run when the event loop returns `Err` (the common TUI leak — run the
loop, capture the `Result`, always restore, *then* propagate). And install the panic hook
before/atomically with entering raw mode + alt screen. Recommended: use ratatui 0.29's
`ratatui::init()` / `ratatui::restore()` (init installs a terminal-restoring panic hook that
chains the previous hook so the backtrace still prints after restore) — that satisfies "restore
before printing" for free; still wrap the run result in an always-restore.

#### M5 — ratatui 0.29 MSRV is *exactly* 1.74.0 (zero headroom); guard against transitive drift
Verified: ratatui 0.29.0 `rust-version = "1.74.0"` (v0.29.0 tag `Cargo.toml` + crates.io);
crossterm 0.28.1 MSRV 1.63.0 — both build on 1.74, pin is valid. But 1.74.0 is the *floor* for
ratatui 0.29 (no margin), and with resolver "2" a minor bump of a transitive dep can raise the
effective MSRV silently. Also ratatui 0.30 requires 1.86 — a hard wall. **Fix (advisory):**
commit `Cargo.lock`, add a `cargo +1.74 check` CI gate on the workspace to catch MSRV drift,
and do not bump ratatui past 0.29 without a deliberate workspace-MSRV decision. Not a blocker.

#### M6 — Task-2 "assert `save()`/append are never called" has no runtime seam
There is no mock/interception point to assert a method was *not* called at runtime. **Fix:**
reframe Task 2's read-only assertion as (a) the structural whole-diff grep (Task 5, per I1) and
(b) the compile-time `&mut`-immutability of the held `Session` (I1). Keep the *behavioral*
tests (correct pass → `Viewer`+Snapshot; wrong pass → error, stays on Unlock; locked → Locked)
— those are sound and mirror the existing `session.rs` temp-vault pattern.

---

### NIT

#### N1 — Local re-implementation of the CLI's private tag fns risks silent drift
Re-implementing `term_tag`/`basis_source_tag`/etc. locally is the right call (don't widen the
CLI API). To bound drift, add a tiny KAT in btctax-tui asserting each local tag fn emits the
known stable string per enum variant (e.g. `Term::LongTerm => "long"`). The strings are
contract-stable (they back the CSV export), so hard-coding the expected values is legitimate.

#### N2 — Tax-tab wiring for years without a stored `TaxProfile` (accuracy note, not a defect)
Confirmed `compute_tax_year` returns `NotComputable(TaxProfileMissing)` when
`profile == None` (`compute.rs:267-273`) — so the spec's "if NotComputable(blocker), show the
reason + no numbers" path already covers no-profile years. Make the wiring explicit in the
plan: pass `snapshot.profiles.get(&year)` to `compute_tax_year`; for the SE block, compute only
when a profile exists (`profile.filing_status`) **and** `tables.table_for(year).is_some()`
(`compute_se_tax` takes a single `&TaxTable` from `table_for(year)` + a `FilingStatus`).

#### N3 — `TestBackend` capability confirmed (no change needed)
`ratatui::backend::TestBackend` + `Terminal::new(TestBackend::new(w,h))` →
`terminal.backend().buffer()` yields an assertable `Buffer` — a real ratatui capability, and
the correct tool for the tab KATs. Coverage plan (per-tab header/known-row/TOTAL, year filter,
empty state, Computed vs NotComputable) is adequate without a real terminal. Recorded as
confirmation.

---

## Objective-by-objective summary

1. **Read-only guarantee (highest priority):** open→project→compute→forms→verify contains **no
   mutation** — all readers take `&self`/pure refs; `save()` is the sole mutator (`&mut self`).
   BUT the *discipline* is not yet sufficient/airtight: leverage `&mut`-gating (hold `Session`
   immutable) + add `conn(` to the grep and forbid `Session::conn()` in the TUI (**I1**).
2. **Security:** (a) opens ONLY the vault via `Session::open`; `ReadOnly` is never read by any
   path the TUI calls (only in doc/test-privacy comments; import/`read.rs`/`ingest.rs` are not
   invoked) — ✅. (b) passphrase hygiene intent correct but zeroization mechanism
   under-specified + `zeroize` dep missing (**I2**). (c) offline confirmed — ratatui/crossterm
   are terminal-only; `BundledPrices`/`BundledTaxTables` are `include_str!`/const compiled-in;
   sequoia crypto-rust is pure-Rust; no `reqwest`/`http`/`net::` anywhere in the dep path — ✅.
   (d) `VaultLock` → `StoreError::Locked` → the Locked screen — correct (✅).
3. **Terminal safety:** panic-hook restore correctly mandated for panic + clean exit; extend to
   the run-loop `Err` path + install-ordering (**M4**).
4. **Dependency/MSRV:** ratatui 0.29.0 = **1.74.0**, crossterm 0.28.1 = 1.63.0 → **compile on
   1.74; pin is valid, NOT a blocker.** Zero headroom + transitive-drift guard (**M5**).
5. **Architecture:** depending on `btctax-cli` (lib) for `Session` is sound (reuses the
   encapsulated vault+project wiring; don't replicate crypto). Snapshot-once + `r`-reproject is
   sound. Figure parity **by construction** is real and verified — Compliance reuses the exact
   `build_verify`→`disposal_compliance` path the CLI's `inspect` uses; Tax reuses
   `compute_tax_year`/`compute_se_tax`; Forms reuse `form_8949`/`schedule_d`/`form_8283`.
   Local private-tag re-impl acceptable for display (**N1** to bound drift). Snapshot needs
   `CliConfig` (**M2**) and should drop the unused attested set (**M3**).
6. **Testability:** `TestBackend` KAT approach is a real, adequate capability (**N3**);
   temp-vault + `BTCTAX_PASSPHRASE` unlock tests mirror the existing pattern; reframe the
   "save never called" claim (**M6**).
7. **Scope/right-sizing:** 5-task TDD spine is correct; export + all mutating flows correctly
   deferred to FOLLOWUPS; MVP scope is right. No missing task.

## Required before proceeding past R0
Fix **I1** and **I2** (spec edits), then re-review (§2 loop). Folding the six Minor items in the
same pass is recommended. Re-run the review after the fold — including the last.

---

# Round 2 — re-review (post-fold)

**Artifact:** `design/SPEC_tui_readonly_viewer.md` (revised).
**Baseline re-verified against:** current source @ `crypto.rs:17-30`, `session.rs:52-114`,
`main.rs:334-347` (re-read this round, not trusted from round 1).
**Reviewer role:** independent architect (author ≠ reviewer). **Gate:** R0.

## Verdict — Round 2

**0 Critical / 0 Important → R0 GREEN, ready to implement.** I1 and I2 are both CLOSED; M1–M5
are correctly folded and internally consistent; the folds introduced **no** new Critical or
Important finding. Residuals are 2 Minor + 1 Nit, all non-blocking (do not gate GREEN); fold
opportunistically or track in FOLLOWUPS.

## Fold verification

### I1 — read-only enforcement — **CLOSED**
Re-verified against source: `save(&mut self)` (`session.rs:64`) and `conn(&self) -> &Connection`
`pub` (`session.rs:59`), so the two levers are exactly as the finding described. The revised spec
now closes both:
- **Compile lever (a):** the constraint (lines 15-17) mandates an IMMUTABLE `Session` binding
  (`let session = …`, never `let mut`), making `save()` — `&mut self` — a *compile* error;
  "keep it immutable everywhere it's stored." Task 2 (line 139) holds the returned `Session`
  immutable; Task 5 (line 167) checks the immutable binding. The misleading round-1 claim
  ("Rust gives no compile enforcement") is gone.
- **Interior-mutability lever (b):** the constraint (lines 18-20) FORBIDS `Session::conn()` in
  btctax-tui (rusqlite writes through `&Connection` without `&mut`/`save()`), restricting reads
  to the typed methods. `conn(` is added to the whole-diff grep set (lines 21-22 and Task 5
  line 167: `save(` / `append_` / `cmd::` / `conn(`), correctly scoped to the crate (so
  btctax-cli's internal `config()`→`conn()` is out of scope, as intended).
Compile guarantee + review grep together make the read-only guarantee airtight. **Adequate.**

### I2 — passphrase zeroization — **CLOSED**
Re-verified `Passphrase(String)` + `Drop::zeroize(self.0)` (`crypto.rs:17-30`). The revised
constraint (lines 26-31) and Task 2 (line 139) hand the passphrase over by **move** —
`Passphrase::new(std::mem::take(&mut buffer))` — so the store owns the only heap copy and wipes
it on drop; `mem::take` leaves an empty String; **never `clone()`**; never logged/rendered; the
`BTCTAX_PASSPHRASE` path passes the env `String` straight into `new` (no persistent buffer,
mirroring `main.rs:336-337`). No new `zeroize` dep. **Sound: no un-wiped retained heap copy.**
(One residual on transient copies — see M7 — but the mechanism as scoped by I2 is closed.)

### Minors M1–M5 — all correctly folded, internally consistent
- **M1** (dep pins) — CLOSED. Lines 60-65: explicit `ratatui = "0.29"` / `crossterm = "0.28"`,
  explicit pins for `rust_decimal`/`time`/`thiserror`, and the correct note that there is **no**
  `[workspace.dependencies]` table so `.workspace = true` is used *only* for the
  `[workspace.package]` `edition`/`rust-version` keys. Consistent.
- **M2** (Snapshot captures `CliConfig`) — CLOSED. Snapshot field list (line 74) adds `CliConfig`
  "`build_verify` needs it (not ProjectionConfig)"; Task 2 (line 141) loads `config`
  (`session.config()?`, which returns `Result<CliConfig,_>` — handled by the Task-2 `CliError`
  mapping); Compliance tab (lines 108, 159) feeds it to `build_verify(&state,&events,&cli)`.
  Consistent with `render.rs:465`.
- **M3** (drop `optimize_attested_set`) — CLOSED. Line 74-75 explicitly does NOT store it
  ("`disposal_compliance`, `build_verify` don't consume it; omit it"); Task 2 (line 141) "NOT
  optimize_attested_set." It remains in the recon *inventory* (line 43) as available API — that
  documents availability, not a stored field, so no contradiction. Snapshot loads `config`,
  not the attested set. Consistent.
- **M4** (restore on `Err` path) — CLOSED. Constraint (lines 32-34): ALWAYS restore on normal
  exit, on the run-loop's `Err` return path, AND on panic (hook restores before printing).
  Task 5 (line 172) checks all three. Consistent.
- **M5** (Cargo.lock + MSRV gate) — CLOSED. Lines 66-69: commit `Cargo.lock`, add a
  `cargo +1.74 check` CI gate, do not bump ratatui past 0.29 (0.30 → 1.86 wall). Task 5
  (line 172) checks `Cargo.lock` committed + MSRV-1.74. Consistent.

## No new Critical / Important from the folds
Checked each fold for regressions: the M2 `CliConfig` fetch is a read (`config()` is `&self`,
`session.rs:75`) and its `Result` is handled; the M3 omission removes a fetch without breaking
Compliance parity (the `AttestedRecording` status is derived inside `disposal_compliance`, not
from the side-table); the immutable-binding requirement (I1) does not conflict with the `r`
re-project (re-projection re-runs `load_events_and_project(&self)`, a read — no `&mut` needed).
Spec stays internally consistent, right-sized (5 TDD tasks), strictly read-only + offline +
terminal-safe + passphrase-safe. **0 new C/I.**

## Residual findings (non-blocking — do NOT gate GREEN)

### M7 (NEW) — Minor — masked-input buffer may leave partial-passphrase fragments in freed heap
`mem::take` (I2) wipes the *final* buffer copy, but the ratatui masked field accumulates the
passphrase char-by-char into a *growing* `String`; each reallocation as it grows copies the
partial passphrase to a new allocation and frees the old one **un-zeroized**. The CLI avoids
this by reading through `rpassword::prompt_password` (`main.rs:339`); the TUI's hand-rolled
field does not. **Severity = Minor**, not Important: the threat model is offline/local/
single-user, an attacker who can scrape this process's freed heap can equally read the live
buffer or the mlock-exempt transient copies the store itself makes (`crypto.rs:23`
`self.0.as_str().into()`), and it does not touch the read-only guarantee. **Cheap mitigation:**
pre-size the field once — `String::with_capacity(128)` — and cap input length so it never
reallocates (then `mem::take` wipes the sole allocation); or accept-and-document as matching the
store's existing transient-copy posture. Recommend folding the one-line `with_capacity`, else
FOLLOWUPS.

### M6 (round-1 residual, not folded) — Minor — Task-2 "assert save()/append never called" still lacks a runtime seam
Task 2 (line 146) retains "Assert `save()`/append are never called" verbatim. As round-1 M6
noted, there is no mock/interception seam to assert a method was *not* called at runtime. The
guarantee is in fact delivered by (a) the Task-5 grep (now incl. `conn(`) and (b) the
compile-time immutable `Session` — so this is a wording/testability imprecision, not a gap.
**Fix:** reframe as those two structural checks, or make it a behavioral on-disk assertion
(temp-vault bytes/mtime unchanged after the Snapshot build). Non-blocking.

### N (round-1 trivial drift, still present) — Nit — `config()` signature in recon
Line 44 still writes `config() -> CliConfig`; actual is `Result<CliConfig, CliError>`
(`session.rs:75`). Task 2's error mapping already handles the `Result`, so this is cosmetic.
Correct the recon line when convenient.

## Round-2 bottom line
**I1 + I2 CLOSED; M1–M5 folded and consistent; 0 new Critical/Important → the spec is R0
GREEN and ready to implement.** Residual M7 (new, Minor), M6 (Minor), and the `config()` Nit
are non-blocking; recommend folding the one-line M7 `with_capacity` and the M6 wording in the
implementation pass, and tracking the rest in `FOLLOWUPS.md`.
