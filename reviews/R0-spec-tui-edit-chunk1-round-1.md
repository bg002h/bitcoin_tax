# R0 Architect Review (mandatory gate) — `tui-edit-chunk1` (mutating TUI, chunk 1), Round 1

- **Artifact reviewed:** `design/SPEC_tui_edit_chunk1.md`
- **Source verified against:** working tree @ HEAD `22cda75` — `crates/btctax-tui/{Cargo.toml, src/{main,app,unlock,draw,export}.rs, src/tabs/{holdings,disposals,income,tax,forms,compliance,tests}.rs}`, `crates/btctax-cli/src/{lib,session,tax_profile,eventref,render,main}.rs`, `crates/btctax-cli/src/cmd/tax.rs`, `crates/btctax-core/src/{persistence.rs,tax/types.rs}`, `crates/btctax-store/src/{vault,atomic,lock}.rs`, workspace `Cargo.toml`. **Every line citation in the spec was re-checked against current source** (result: all accurate; see §7).
- **Reviewer role:** independent architect (author ≠ reviewer).
- **Date:** 2026-07-02.

## Verdict

**NOT GREEN.** 0 Critical, **1 Important**, 5 Minor, 5 Nit.

The two-guarantee architecture is fundamentally sound and the spec is unusually well-grounded
in source (all four DRIFT notes check out; no additional drift found). The blocking issue is a
**coverage hole in the editor's mechanized gate**: the D3 token table misses the vault-*creating*
constructors (`Session::create`/`Session::repair`, `Vault::create`/`Vault::repair`), which are
exactly the class of vault-file writer the guarantee statement claims to exclude ("the vault file
only via `Vault::save`'s atomic path"). One spec-text edit fixes it. Fix I1 (and fold M1–M5),
re-review, and this is implementable.

**Two-guarantee enforcement rating:**

- **Viewer (a): AIRTIGHT as specified.** Task 1 is genuinely a pure visibility refactor. The E10
  scanner walks `CARGO_MANIFEST_DIR/src` recursively, so the new `lib.rs`/`viewer.rs` files are
  automatically *inside* its coverage, and no moved code introduces any forbidden token.
  `open_session` returning a live `Session` adds **no new capability**: any dependent of the
  workspace can already call `btctax_cli::Session::open` directly (btctax-cli exports
  `pub use session::Session`, lib.rs:15); the seam only single-sources error mapping + snapshot
  building. The viewer still drops the session at unlock (wrapper), `App` never stores one, and
  the immutable-binding compile-level property is untouched. The DRIFT-3 App-free `render`
  extraction is **verified adequate**: grep of all six tab renderers shows they consume exactly
  `app.snapshot`, `app.selected_year`, and the per-tab `TableState` — nothing else — so the
  proposed signatures lose nothing, and keeping the `pub(crate)` wrappers preserves the
  TestBackend suite verbatim (tests.rs:92/104/116/827/839/851 call the wrappers with an `App`).
- **Editor (b): STRONG, ONE HOLE (I1).** The confinement architecture — single allowlisted module
  + E10-clone gate with comment-stripping and plant-a-token self-check + blocking payload modal +
  KAT-C1/P1/G1 + the Task-4 independent grep — is the right layered design, and the "procedural,
  not type-level" limitation is honestly recorded with a sealed-token FOLLOWUP (mirrors viewer
  R0-N3). But the gate's token table cannot see `Session::create(…)`, which writes a vault file
  outside `persist.rs` without tripping `save(`, `conn(`, `cmd::`, or any fs verb. With I1 folded,
  I rate the editor's enforcement airtight *at the procedural level the spec claims*.

---

## 1. The two-guarantee structure — PASS with one Important finding

### 1a. Viewer lib split (D1) — PASS

Verified point by point:

- **No `[lib]` today; all modules bin-private** — confirmed (Cargo.toml:8–10; main.rs:13–17).
  The gap statement is accurate.
- **E10 coverage survives the split.** The gate lives in export.rs's `#[cfg(test)]` region
  (export.rs:690–919, confirmed) and scans `$CARGO_MANIFEST_DIR/src` recursively
  (export.rs:693–698, 812–826). File moves *within* `src/` (main.rs body → lib.rs/viewer.rs)
  stay in scope; coverage strictly grows. The spec's claim "continues to cover every file it
  covers today" is correct (understated, even).
- **No new write-capable surface *from the viewer*.** The externally-`pub` table exposes only:
  read-only data types (`Snapshot`, `Screen`, `Tab`), the unlock widget, terminal-lifecycle
  helpers, App-free renderers, and `open_session`. `open_session` hands out a `Session` — but
  that is not a viewer-granted capability (see rating above), and the viewer's own source still
  cannot call `save(`/`conn(` (E10 everywhere-tokens, export.rs:708–715, unchanged). The
  structural backstop relied on by SPEC_tui_export ("the viewer App never stores a Session") is
  explicitly preserved by the wrapper design. Sound.
- **`App` demotion is required and the spec pins it.** `pub mod app` must be pub for `Snapshot`,
  so `App` must drop to `pub(crate)` — the table's "INTERNAL" row covers this; the E0446
  reasoning for keeping `draw::draw` and the tab wrappers internal is correct Rust.
- **Wrapper behavior-identity** (`attempt_open` over `open_session`): logic-equivalent, including
  the build_snapshot-Err path (session drops, `Error(msg)` returned — same as today's scope-exit
  drop). One pin missing → **M5** (early `drop(pp)` ordering).

### 1b. Editor guarantee enforcement (D3/D4) — one hole

**[I1] IMPORTANT — the editor gate's token table misses the vault-creating constructors.**

The guarantee statement: *"…the vault file only via `Vault::save`'s atomic path."* The stated
enforcement is the D3 gate. But the editor crate deps include `btctax-cli` and `btctax-store`,
so any editor module can name `btctax_cli::Session::create(path, &pp)` (session.rs:30–32) or
`Session::repair` (session.rs:37–39) — or `btctax_store::Vault::create`/`Vault::repair` directly —
each of which **creates/overwrites a vault file on disk**. None of the D3 tokens fire on these
call spellings:

- `save(` / `conn(` — not present at the call site (the save happens inside btctax-cli/btctax-store).
- `cmd::` — `Session::create` is not a `cmd::*` path.
- The fs-verb list (`File::create`, `fs::write`, …) — matches `File::create`, not `Session::create`.

The viewer's E10 shares this blind spot, but there it is inert (the viewer's threat model has no
reason to construct vaults, and `attempt_open` only *opens*). For the **first vault-writing
binary**, whose guarantee text specifically enumerates the only sanctioned vault-file write path,
this is a real coverage hole in the load-bearing enforcement mechanism.

**Exact fix (spec-text only):** add a row to the D3 token table — tokens `Session::create`,
`Session::repair`, `Vault::create`, `Vault::repair` (four explicit tokens; do not use a broad
`::create(` which would collide with the existing `File::create`/`create_dir` handling) —
**FORBIDDEN everywhere in non-test code**; test regions keep the existing sole fixture exception
(`cmd::init::run`, which is the sanctioned creation path and lives outside the scanned crate).
Mirror the same four tokens into the Task-4 whole-diff grep list, and plant one of them
(runtime-constructed, e.g. `format!("Session::{}", "create")`) in the KAT-G1 self-check.

**Why Important, not Critical:** the specified chunk-1 code contains no such call (verified —
the only Session constructor named anywhere in the design is `open_session`'s `Session::open`);
the hole is in *regression enforcement*, not in the specified behavior, and the Task-4 review
layer partially backstops it. It still blocks the gate: the gate is the spec's own stated
enforcement of the crate contract, and the fix is one table row.

Everything else in 1b checks out:

- **Confinement:** `conn()`/`save()`/`tax_profile::set`/`append_decision` named only in
  `edit/persist.rs`; the persist fn mirrors `cmd::tax::set_profile` (cmd/tax.rs:14–23, verified:
  `Session::open → tax_profile::set → s.save()` — the open/drop-per-call shape is indeed wrong
  for a held-lock editor and would deadlock on `StoreError::Locked`; bypassing `cmd::*` is
  correct and the gate forbids `cmd::` to keep it that way).
- **The gate's test-region policy is self-consistent with the KATs**: KAT-P1's fixture needs
  `append_decision` + `save(` + `conn(` in test code; the table permits exactly that
  (non-test-only rules for those tokens) while keeping the fs-verb and export tokens strict
  where they should be.
- **Modal as the only path:** procedural, recorded as a limitation with a sealed-token FOLLOWUP,
  and independently checked twice (KAT-G1 confinement of the *surface* + Task-4 "sole non-test
  call site is the modal's Enter arm"). Acceptable for one flow; the FOLLOWUP correctly notes it
  must harden if flows multiply.
- **Esc-cancel bytes-unchanged:** KAT-C1 drives the real `handle_key` (not synthetic state), and
  its confirmed-mutation complement prevents a trivially-green pass. The dispatch-order pin
  (modal → form → screen, R0-M4 lesson) matches the viewer's proven structure (main.rs:124–153,
  verified: blocking modal, `q` swallowed, Esc-does-not-quit).

## 2. Session lifecycle — PASS

- **VaultLock exclusivity: verified real.** `Session::open` → `Vault::open` acquires
  `VaultLock::acquire` (vault.rs:120) before anything else; the lock is `flock`-based
  (fs2 `try_lock_exclusive`, lock.rs:11–26), surfaces `StoreError::Locked` on contention, and is
  held as `Vault::_lock` for the vault's lifetime (vault.rs:137–142). Same-process second opens
  are also refused (flock treats independently-opened fds as conflicting; lock.rs's
  `second_acquire_refused` test proves it in-process). The "no concurrent-writer case" claim is
  sound in both directions.
- **DRIFT-2 seam: correct and necessary.** Confirmed `attempt_open` returns only
  `Success(Box<Snapshot>, i32)` and drops the `Session` at scope exit (unlock.rs:93–107) — a
  session-holding editor genuinely cannot reuse it as-is. The `open_session` +
  behavior-identical-wrapper design is the right cut; `map_open_error` single-sourcing keeps the
  editor's unlock strings identical by construction. (See M5 for the one ordering pin.)
- **Save-per-action atomicity: the crash story is accurate.** Verified `Vault::save` =
  `db_to_bytes → encrypt → atomic_write` (vault.rs:147–151); `atomic_write` = `.tmp` write +
  `sync_all` → copy live → `.bak` + `sync_all` → `fs::rename` + parent-dir sync (atomic.rs:6–31);
  `recover_target`/`reap_tmp` run at the next `Vault::open` — **at vault.rs:124–125, exactly as
  the DRIFT-1 correction states** (the recon's 147–151 cite was indeed `Vault::save`). The
  durability contract as worded (crash between actions loses nothing; crash during save leaves
  old-or-new complete image, never torn) matches the mechanism, including the
  crash-after-bak-before-rename case (old image survives; stray `.tmp` reaped at next open).
- **Re-projection:** `build_snapshot` takes `&Session` (unlock.rs:112) — already correct for
  post-mutation re-projection with the held session; making it `pub` is the minimal seam. The
  Err-path story (keep old snapshot, tell the user to restart; the save already landed) is the
  right priority ordering. See M1 for the *save*-Err sibling path.

## 3. Validation parity pin — PASS (posture: correct)

Re-verified all ten rules one-for-one against main.rs:688–760 (current source):

| # | Field | CLI rule (verified lines) | Spec D4 | Match |
|---|---|---|---|---|
| 1 | filing_status | required (690–692) | required, structurally satisfied | ✓ |
| 2 | ordinary_taxable_income | required + parse (693–700) | same | ✓ |
| 3 | magi_excluding_crypto | required + parse (701–708) | same | ✓ |
| 4 | qualified_dividends… | required + parse (709–716) | same | ✓ |
| 5 | other_net_capital_gain | optional → 0 (718–722) | same | ✓ |
| 6 | carryforward_short | optional → 0 (723–727), **no negativity check** | same | ✓ |
| 7 | carryforward_long | optional → 0 (728–732), **no negativity check** | same | ✓ |
| 8 | w2_ss_wages | optional → 0; `is_sign_negative()` rejected (733–740) | same | ✓ |
| 9 | w2_medicare_wages | optional → 0; negative rejected (741–750) | same | ✓ |
| 10 | schedule_c_expenses | optional → 0; negative rejected (751–760) | same | ✓ |

Confirmed: **only** fields 8–10 are negativity-checked today; fields 2–7 accept negatives at the
CLI even though the `Carryforward` doc-contract (types.rs:17–19) says magnitudes ≥ 0 — the spec's
parity pin describes the source exactly. Parse is `Decimal::from_str(s.trim())`
(`parse_usd_arg`, eventref.rs:75–78, verified). The construction mirrors main.rs:762–775. The
field order matches `--show` (main.rs:663–685) and the 10-leaf surface matches types.rs:31–68
(9 struct fields, `Carryforward` contributing two leaves) and the 10 CLI value flags.

**Should the editor tighten now? No — the spec's posture is right.** Unilateral editor tightening
would make the same user data settable via the CLI but rejected by the editor (or vice versa on
pre-population of an existing negative-carryforward profile, which would then fail re-validation
on an unrelated edit — a genuinely nasty UX trap). Tightening must land on both surfaces
simultaneously; the FOLLOWUP records exactly that. Zero invented rules, zero dropped rules,
asymmetry risk zero by construction — as claimed. (See M4 for the one edge the spec should pin:
whitespace-only buffers.)

## 4. The safety tests — PASS with two Minors

- **KAT-P1 is genuinely non-vacuous.** Three independent guards: (i) the log is *seeded* with ≥ 2
  decision events before `pre` is captured, so `post == pre` is not an empty-vs-empty tautology;
  (ii) the round-trip assertion (`session.tax_profile(2025) == fixture_profile`) proves the
  mutation actually executed; (iii) the second, *differing* upsert re-asserting `log == pre`
  while the read-back updates proves upsert-not-append. The drop + reopen re-assertion binds the
  in-memory claim to the persisted image. The degenerate strong form (`post == pre`) is the
  correct chunk-1 instantiation of the program-level prefix invariant, and stating the
  strictly-growing chunk-2 form now (with the same skeleton) is good forward wiring.
- **`load_all_ordered` mechanism: correct.** `ordinal` is `INTEGER PRIMARY KEY AUTOINCREMENT`
  (persistence.rs:104, "insertion order ONLY") — AUTOINCREMENT guarantees monotonic, never-reused
  ordinals, so `ORDER BY ordinal` is a stable insertion-order read; no ordered read exists today
  (verified: no `ORDER BY` anywhere in persistence.rs); the fn is read-only and correctly
  documented as not-a-projection-input (NFR4). See **M2** for the column-projection width.
- **KAT-C1: solid.** Real-dispatch driving, modal-blocking sub-asserts, and the
  confirmed-mutation-changes-bytes complement close the trivially-green loophole.
- **What's missing:**
  - *Kill-mid-save:* **adequately covered where it lives.** `atomic.rs`'s unit tests
    (`write_keeps_prev_in_bak_and_target_never_absent`, `recover_from_bak_when_target_missing`,
    `reap_tmp_*`, atomic.rs:60–101) simulate every crash artifact the mechanism can produce
    (missing target + bak; stray tmp with/without target). The editor adds no new write
    *mechanism*, only a new caller of `Vault::save` — re-testing crash recovery per-caller would
    be duplicate coverage. A live SIGKILL-mid-save integration test is FOLLOWUP-grade at most;
    not required for this chunk.
  - *Save-error path:* **M3** — D4's Err-arm makes four behavioral claims with no KAT.
  - The editor-holds-lock ⇒ CLI-gets-Locked converse is proven by lock.rs's own test +
    KAT-U1's Locked path; no additional test needed.

## 5. Form / modal — PASS

- **`FieldBuffer` cap pattern (rated, as asked):** the *hygiene* rationale (never-reallocate to
  avoid scattering secret bytes in freed heap) does **not** transfer to plaintext money fields —
  and the spec already says so explicitly ("the pattern reused is the buffer discipline, not the
  masking"). What the cap *does* buy is bounded input (no pathological growth, no per-keystroke
  realloc churn), which is worth having and costs nothing. `FIELD_CAP = 64` is ample for any
  `Decimal` rendering (~31 chars worst-case). No change needed; correctly justified.
- **FilingStatus Tab-cycling:** 5 variants confirmed (types.rs:9–15); the "Tab NEVER inserts
  text" pin is the load-bearing part and is present; scoping latitude + FOLLOWUP is fine.
- **Pre-population:** `snapshot.profiles` carries all profiles (app.rs:108, via
  `all_tax_profiles`, session.rs:88–92 — verified). `Decimal` `Display` ↔ `from_str` round-trips
  exactly (scale-preserving), so pre-fill → unmodified re-submit reproduces the stored profile
  bit-for-bit. KAT-F1 pins it.
- **Modal payload:** all 10 leaves + year from the **validated** `TaxProfile` (not raw buffers) —
  the right choice (what is shown is what is persisted, verbatim); `render::filing_status_tag`
  is `pub` (render.rs:359, verified reachable). KAT-F2 pins rendering exactness.
- **EDITOR marker:** all three surfaces (unlock title + note, tab-bar badge, footer) verified
  visibly distinct from the viewer's actual strings (draw.rs:26, :52, :97). The note-line swap
  ("read-only · PGP-encrypted" → "EDITOR — writes on explicit confirmation only") is the single
  most important disambiguator and is correctly routed through the `draw_unlock_screen`
  extraction so the viewer's rendering stays byte-identical.

## 6. Scope / right-sizing — PASS

- Four tasks with the lib split isolated first (its acceptance gate — full viewer suite with
  zero content changes + E10 green — is the strongest possible "pure refactor" proof) is the
  right decomposition. Task 2's "the key must not exist half-wired" pin is a good touch.
- Chunk 2+ exclusion is clean, and pre-wording the guarantee + gate for `append_decision` now
  (so the contract never rewords) is the right call.
- **MSRV/DRIFT-4: correct** — workspace `rust-version = "1.88"` (Cargo.toml:7); no
  `[workspace.dependencies]` table exists, so per-crate pins matching the viewer's
  (ratatui 0.29 / crossterm 0.28 / rust_decimal 1 / time 0.3, verified against
  btctax-tui/Cargo.toml:17–24) are the correct dependency posture. The viewer-spec MSRV-1.74
  doc correction belongs in FOLLOWUPS, where the spec put it.
- Workspace members list is one line (Cargo.toml:3) as cited.

## 7. Citation audit (drift beyond the four DRIFT notes)

All spec citations were re-verified against the working tree; **no additional drift found**. The
four DRIFT notes are each accurate (DRIFT-1: recover call site is vault.rs:124, not 147–151 ✓;
DRIFT-2: `attempt_open` drops the Session ✓; DRIFT-3: tests call the tab draws directly with an
`App` at tests.rs:92/104/116/827/839/851 ✓; DRIFT-4: MSRV 1.88 ✓). Two sub-line-level nits: N1.

---

## Findings

### Critical

None.

### Important

- **[I1] Editor gate misses vault-creating constructors.** `Session::create`,
  `Session::repair`, `Vault::create`, `Vault::repair` write vault files but trip no D3 token.
  Add the four tokens (FORBIDDEN everywhere, non-test; test fixture path remains
  `cmd::init::run`) to the D3 table, the Task-4 grep list, and the KAT-G1 self-check plant.
  Full analysis in §1b.

### Minor

- **[M1] Failed-save session divergence is unstated.** If `tax_profile::set` succeeds but
  `session.save()` fails, the held conn holds a confirmed-but-unpersisted upsert: the snapshot
  is not re-projected (correct), but ANY later successful save (e.g. a retry, or a different
  year's profile) will also persist the earlier upsert. This is *safe* (everything persisted was
  explicitly confirmed at some point, and the upsert is idempotent on retry), but D4's Err-arm
  should say it: one sentence pinning "the in-memory session retains the confirmed upsert; a
  retry re-runs the idempotent upsert + save; any later successful save also carries it" — so no
  implementer 'fixes' it by rolling back the side-table, and no later reviewer calls it a leak.
- **[M2] `load_all_ordered` projects 2 of 10 event columns.** "Nothing rewritten" is only
  enforced over `(event_id, payload_json)`; a bug rewriting `utc_timestamp`, `wallet_json`,
  `tz_offset_sec`, `kind`, `source`, `source_ref`, `decision_seq`, or `fingerprint` on an
  existing row would pass KAT-P1 on the *confirmed* path (KAT-C1's byte-check only covers the
  cancel path). Widen the SELECT to all columns (`ORDER BY ordinal` unchanged) — the fn is new,
  so this costs nothing and strengthens the chunk-2 prefix test for free.
- **[M3] The save-error path has behavioral claims but no KAT.** D4's Err-arm promises: modal
  closes, form stays open with buffers intact, `status = "Save error: …"`, vault unchanged on
  disk. Add a KAT (cfg(unix): make the vault's parent dir read-only to force `atomic_write`
  failure; assert the four claims + that a retry after restoring permissions succeeds), or
  explicitly record the gap in FOLLOWUPS instead of leaving it implicit.
- **[M4] Pin "empty" = len-0 for the optional-field default.** The CLI errors on
  `--flag "  "` (parse of trimmed-empty fails in `parse_usd_arg`) but defaults on an *absent*
  flag. The editor's empty-buffer→0 mapping is right, but a whitespace-only buffer must take the
  parse path (→ error), not the default path — i.e. test emptiness BEFORE trimming. One sentence
  in D4 + one KAT-V case.
- **[M5] Pin the early `drop(pp)` in `open_session`.** Today `attempt_open` zeroizes the
  passphrase immediately after `Session::open` succeeds, *before* `build_snapshot`
  (unlock.rs:100–101). "Body = today's logic" implies it, but this is the one hygiene-relevant
  ordering in the seam being re-cut and the wrapper-consistency KAT cannot detect its loss —
  make it an explicit D1 bullet.

### Nit

- **[N1] Two citation micro-nits:** `Screen` spans app.rs:18–24 (derive line included; spec says
  19–24); tests.rs:128 is `crate::draw::draw(f, app)` (the full-frame helper), not a *tab* draw —
  it still supports DRIFT-3's point, but the sentence says "these draw fns".
- **[N2] Inherited scanner limitations.** The E10 structure the editor clones (a) sets
  `in_test = true` at the first `#[cfg(test)]` and never resets, and (b) naively strips from the
  first `//` (breaking on string literals containing `//`). Both are accepted in the viewer;
  restate the operative convention in D3 ("tests are the last item in every editor module") so
  the property holds by construction in the new crate too.
- **[N3] Re-projection reloads `BundledTaxTables`/`BundledPrices` per mutation** (via
  `build_snapshot` → `load_events_and_project`). Correctness-first is right for chunk 1; worth a
  one-line acknowledgment so it isn't 'discovered' as a perf bug later.
- **[N4] Validation error strings differ cosmetically from the CLI's** ("ordinary-taxable-income
  is required" vs "--ordinary-taxable-income is required when setting a profile"; "bad USD
  {input}" vs `bad USD {s:?}: {e}`). Parity is behavioral (accept/reject), which is what D4
  pins — fine as spec'd; noting it here so no later review claims string parity was promised.
- **[N5] `status` doc says "cleared on next key"** (EditorApp field comment) — mirror the
  viewer's precise semantics ("cleared on the next non-modal key press", app.rs:140) to avoid
  the modal-Enter status being instantly wiped by the modal's own key handling.

---

## Gate disposition

**Blocked at 1 Important.** Required to clear: fold **I1** (D3 table row + Task-4 grep list +
self-check plant). M1–M5 are each one-to-three-sentence spec edits and should be folded in the
same pass. Per §2 of `STANDARD_WORKFLOW.md`, re-review after the fold.

---

# Round 2 — re-review (post-fold)

- **Artifact re-reviewed:** `design/SPEC_tui_edit_chunk1.md` (full re-read of the folded spec;
  every `[R0-…]` fold tag verified in place, and re-verified against source wherever a fold
  makes a new factual claim).
- **Reviewer role:** independent architect (same reviewer as round 1; author ≠ reviewer).
- **Date:** 2026-07-02.

## Verdict

**0 Critical / 0 Important — R0 GREEN.** All 11 round-1 findings are genuinely folded (not
paraphrased away); the folds introduce no new findings; the two-guarantee structure is intact
and strengthened; the spec remains right-sized (4 tasks, chunk-2+ excluded). **Ready to
implement Tasks 1–4.** Two residual micro-nits recorded below — both non-blocking, both
optional at implementation.

## Fold-by-fold verification

- **[I1] CLOSED — the vault-creation hole is shut.** Verified at all four required sites:
  (1) the Hard-constraints confinement bullet names the ban; (2) the D3 table gains the
  dedicated row — the four **explicit** tokens `Session::create` / `Session::repair` /
  `Vault::create` / `Vault::repair`, FORBIDDEN everywhere in non-test code, correctly rejecting
  the broad-`::create(` alternative (which would collide with the `File::create`/`create_dir`
  rows), with accurate source cites (session.rs:30–32, 37–39 — re-checked); (3) the KAT-G1
  self-check plants a runtime-constructed `Session::create` (`format!("Session::{}", "create")`
  — no literal token in test source, matching the E10 self-check discipline); (4) the Task-4
  grep list adds the zero-hits check. Test regions keep `cmd::init::run` as the sole sanctioned
  fixture-creation path (it lives outside the scanned crate). Consistency checked: no chunk-1
  KAT needs a banned constructor (`Session::open`/`open_session` are unbanned, correctly), and
  there is no `persist.rs` allowlist exception for these tokens (also correct — persist.rs has
  no business creating vaults either).
- **[M1] CLOSED — failed-save semantics pinned, coherent, and honest.** The D4 Err-arm now
  states: no side-table rollback (the divergence is intentional); disk = pre-action state per
  the atomic path; retry re-runs the idempotent upsert + save; **(b) any later successful save
  also persists the earlier confirmed upsert** — the subtle consequence is stated, not hidden;
  (c) quit-without-save loses the mutation, correctly framed as the save-per-action contract
  itself; no re-projection on Err (UI keeps showing last-saved state). Every persisted byte
  remains user-confirmed; nothing unconfirmed can escape. Cross-referenced from Task 4's
  modal-gating check and tested by KAT-S1.
- **[M3] CLOSED — KAT-S1 is a genuine test, not a gesture.** Mechanics check out: chmod
  `0o500` on the vault's parent blocks `atomic_write`'s `.tmp` creation (EACCES for non-root)
  while leaving the vault readable for the byte-compare; the confirm-then-fail sequence
  exercises exactly the M1 divergence (set succeeded in-memory, save failed on disk); all four
  Err-arm claims are individually asserted; restore-perms + re-confirm proves the idempotent
  retry AND re-asserts the event log unchanged (tying into KAT-P1's invariant). The root-skip
  guard probes the actual denial rather than checking uid. The pre-recorded fallback
  (`#[ignore]` + explicit documented-not-tested FOLLOWUP entry, conditionally recorded) makes
  any downgrade loud, never silent. The gate permits the test's `set_permissions` use (fs verbs
  are test-region-allowed) — no self-contradiction.
- **[M2] CLOSED.** `load_all_ordered` now returns `RawEventRow` — verified against the DDL:
  all 10 non-`ordinal` columns present with types matching the existing `load_all` reads
  (`decision_seq: Option<i64>`, `tz_offset_sec: i32`, nullable `source`/`source_ref`/
  `wallet_json`/`fingerprint`), `PartialEq`/`Eq` derived for prefix comparison, `ORDER BY
  ordinal` retained, and the rationale paragraph states the exact hole it closes. SemVer
  header, Task-3 files, and Task-4's prefix-test check all updated consistently.
- **[M4] CLOSED.** Empty = byte-length 0, tested **before** trimming; whitespace-only takes the
  parse path for BOTH optional and required fields — re-verified against the CLI: `--flag "  "`
  is a `parse_usd_arg` error (not a default, and not the "required" error), so the pin is exact
  parity. KAT-V11 covers both cases; the V-series is consistently renumbered V1..V11.
- **[M5] CLOSED.** The early-`drop(pp)`-before-`build_snapshot` ordering is a stated
  requirement in the `open_session` table row (correct cite: unlock.rs:100–101), with the
  honest note that the wrapper-consistency KAT cannot detect its loss — hence the Task-4
  inspection line, which is present.
- **[N1] CLOSED.** app.rs:18–24/27–36 (derives included); DRIFT-3 now separates the six direct
  tab-draw call sites (92/104/116/827/839/851) from tests.rs:128's full-frame `draw::draw`.
- **[N2] CLOSED.** Both inherited scanner limitations restated as by-construction conventions
  (tests are the last item in every editor module; no `//` inside non-test string literals).
- **[N3] CLOSED.** Reload cost acknowledged inline at the re-projection paragraph + FOLLOWUP.
- **[N4] CLOSED.** Behavioral-not-string parity stated where the error strings are defined.
- **[N5] CLOSED.** `status` cleared on the next **non-modal** key press, mirroring
  app.rs:140's semantics — the modal's own Enter can no longer wipe the status it just set.

## New-finding sweep

Re-checked the folds for introduced inconsistencies: the guarantee wording is unchanged (still
covers chunk 2 — no rewording needed later); the modal Enter arm calling `persist_tax_profile`
from dispatch code remains gate-clean (the wrapper name is not a banned token — confinement is
of the raw surface, as designed); KAT-S1 vs the gate's fs-verb rules, KAT-P1 vs `RawEventRow`,
and the FOLLOWUPS list are mutually consistent; the header records the fold and the re-review
requirement. **No new Critical, Important, or Minor findings.**

## Residuals (non-blocking, optional at implementation)

- **[N6] `RawEventRow` deliberately excludes `ordinal`.** A delete + byte-identical re-insert
  of precisely the *tail* row would still pass `post == pre` (the re-inserted row gets a new
  AUTOINCREMENT ordinal, but content and position match). Including `ordinal` in the row would
  close even this, at zero cost. No plausible chunk-1 bug has this shape (`event_id UNIQUE`
  forces it to be the same event) — implementer MAY add the field; not required.
- **[N7] Citation micro-nit:** the D1 surface-table row for `app::Screen`/`app::Tab` still
  cites app.rs:19–36 (the Current-state cite was corrected to 18–24/27–36; the table row
  wasn't). Cosmetic only.

## Gate disposition

**GREEN — 0 Critical / 0 Important.** The R0 gate is cleared; implementation of Tasks 1–4 may
proceed per the plan (TDD, KATs red-first; Task 4's whole-diff review remains its own Phase-E
gate and is not pre-cleared by this R0).
