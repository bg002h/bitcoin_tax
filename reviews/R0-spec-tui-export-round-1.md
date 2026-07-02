# R0 architect review — SPEC_tui_export.md (round 1)

- **Artifact:** `design/SPEC_tui_export.md`
- **Baseline verified against:** HEAD `4125db3` (spec claims `main @ 4125db3` — matches)
- **Reviewer:** independent R0 (architect), security-critical gate: first write capability in the
  deliberately never-writes `btctax-tui` viewer
- **Verdict:** **NOT GREEN — 0 Critical / 4 Important / 4 Minor / 3 Nit.**
  No path to the vault or a decrypted image exists in the specced design, and no unconditional
  guarantee hole was found (hence no Critical), but the no-clobber claim is factually wrong against
  current source, the SE-gating semantics are ambiguous in a way that re-opens the disclosure-drift
  class this spec exists to kill, the figure-parity KAT does not test what the whole-diff item
  claims, and the extended grep gate is review-time-only for a crate that is changing from
  provably-write-free to write-capable.

---

## 0. Citation / drift verification (all checked against current source)

| Spec claim | Verified |
|---|---|
| `render.rs:567` `pub fn write_csv_exports(out_dir, state, tax_year: Option<i32>, se_result, donation_details)` | ✓ exact (render.rs:567–573) |
| Four private form writers: `write_form8949_csv` :898, `write_schedule_d_csv` :942, `write_form8283_csv` :812, `write_schedule_se_csv` :736 | ✓ all four exist, all four are private (`fn`, no `pub`), line numbers exact |
| "year-scoped block inside write_csv_exports (lines 716–728)" | ✓ exact (the `if let Some(year) = tax_year` block) |
| `render_schedule_se` at render.rs:1118, `pub`, 7-param signature, three-way `None` split | ✓ exact |
| Disclosure-element line cites: breakout 1138–1145, I3 advisory 1146–1156, no-expenses note 1163–1166, §164(f) 1199–1211, W-2 1212–1229, fully-expensed 1257–1271 | ✓ all present at (±2 lines of) the cited ranges; the quoted strings match the source verbatim |
| `cmd/admin.rs:45` `export_snapshot` writes `snapshot.sqlite` then `write_csv_exports` | ✓ exact (admin.rs:45–86) |
| `cmd/tax.rs:79–106` SE-input assembly pattern | ✓ exact — **note it is wrapped in `match profile.as_ref() { Some(p) => …, None => None }`** (see I2) |
| `unlock.rs:112–120` Snapshot fields (`events, state, cli_config, profiles, tables, donation_details`); no new fields needed | ✓ (build_snapshot at unlock.rs:112–130; all six fields present, unlock.rs:121–128) |
| `tabs/tax.rs:89–126` hand-rolled SE block, inline `compute_se_tax`, no three-way split, `table_present` never computed | ✓ exact — **note it defaults `FilingStatus::Single` / $0 wages when NO profile exists (tax.rs:90–95), and it sits INSIDE the `TaxOutcome::Computed` arm** (see I2) |
| `main.rs:145–165` Viewer key arm; `e` unbound; no modal state | ✓ (145–164; `e` falls to `_ => {}`; **`Esc` on Viewer currently QUITS** — main.rs:146) |
| `app.rs:116–135` App struct, no export fields | ✓ exact |
| `fsperms::open_owner_only` 0o600 create-or-truncate; `mkdir_owner_only` 0o700 | ✓ — **but `mkdir_owner_only` is `DirBuilder::new().recursive(true)` (mkdir -p semantics): it does NOT fail on an existing dir** (fsperms.rs:73–80). See I1 — the spec's no-clobber paragraph is drift. |
| `se_net_income` pub in btctax-core | ✓ (`crates/btctax-core/src/tax/se.rs:55`, imported at crate root by cmd/tax.rs:7) |
| Injected-`now` convention in optimize/reconcile | ✓ (`now: OffsetDateTime` params throughout cmd/optimize.rs, cmd/reconcile.rs) |
| Existing gate = `save(` / `append_` / `cmd::` / `conn(`, review-level | ✓ (SPEC_tui_readonly_viewer.md:21, :172) — **review-time only; no CI/test automation exists** (`.github/workflows/ci.yml` has no grep gate). See I4. |
| `compute_se_tax` returns `None` when fully expensed (net_se == 0) | ✓ (render.rs:720–725 comment `[N4]`; KAT-E7's fully-expensed fixture premise is correct) |
| KAT-E4 column names `net_se_earnings` / `total_se_tax` / `deductible_half` | ✓ match write_schedule_se_csv's actual header (render.rs:738–746) |

Structural strength worth recording: **the TUI retains neither the `Session` nor the `Passphrase`
after unlock** — `attempt_open` drops both before returning (unlock.rs:93–107); `App` holds only the
in-memory `Snapshot`. `export_snapshot` is therefore *structurally* unreachable from the export path
(no Session to call it on), independent of the grep gate. `do_export` sees only in-memory data plus
an output path; it cannot read or write the vault file at all. This is the right architecture.

---

## Findings

### Important

**[I1] The D2 no-clobber claim is factually wrong, and the pre-existing-dir case defeats both the
0o700 guarantee and (in adversarial layouts) the "never writes the vault" guarantee.**
Spec D2: "`fsperms::mkdir_owner_only` fails naturally if the same-second subdir already exists
(acceptable — … no silent overwrite)." **False.** `mkdir_owner_only` uses
`DirBuilder::new().recursive(true)` (fsperms.rs:73–80) — `recursive(true)` gives `mkdir -p`
semantics: it silently succeeds on an existing directory and does not touch its permissions.
Consequences on the specced design:
1. A same-second re-press of `e`+Enter silently truncates and rewrites the previous export's files
   (`open_owner_only` is create-or-**truncate**) — contradicting the spec's stated behaviour
   contract and the FOLLOWUPS item built on it ("current behaviour is a natural error").
2. If the export dir was **pre-created by someone else** (the timestamp is predictable to the
   second), the export writes into a directory the user does not own, whose mode is never forced to
   0o700 — KAT-E5's dir guarantee only holds for freshly-created dirs.
3. Worst case: `open_owner_only` is `O_CREAT|O_WRONLY|O_TRUNC` **without** `O_EXCL`/`O_NOFOLLOW`
   (fsperms.rs:22–30) — it follows an existing symlink. A pre-created export dir containing a
   symlink named `form8949.csv` → *any user-writable file, including the vault* would be truncated
   and overwritten. On the default layout this is not practically exploitable (the vault parent is
   created 0o700 by the store — vault.rs:50 — so nobody else can pre-create the dir), but the vault
   path is user-supplied (`--vault /tmp/...`, shared dirs), and the re-scoped guarantee is stated
   absolutely.
**Fix (exact):** create the export dir with **exclusive** semantics in `export.rs` before calling
`write_form_csvs`: a new `fsperms::mkdir_owner_only_exclusive` (`DirBuilder` with
`recursive(false)` + `.mode(0o700)`), which genuinely fails with `AlreadyExists` — the parent (the
vault's parent) always exists, so `recursive(false)` is safe. A fresh, empty, 0o700, user-owned dir
makes the symlink/pre-created-dir setup impossible and makes the no-clobber paragraph true.
`write_form_csvs` itself may keep tolerant `mkdir_owner_only` (documented) for future CLI-style
reuse. Amend the D2 paragraph; add a KAT: pre-create the exact deterministic dir (KAT-E6's injected
timestamp makes this easy) → `do_export` errors, no files written, pre-existing dir contents
untouched.

**[I2] SE-gating semantics are ambiguous/contradictory across D2/D3, re-opening the exact
disclosure-drift class D3 exists to kill.** Two sub-issues:
1. **Profile-absent case.** `cmd/tax.rs:79–106` wraps the entire SE assembly in
   `match profile { Some(p) => …, None => None }` — no profile ⇒ **no SE section at all**, even
   with business income. The current TUI block instead **defaults**
   `FilingStatus::Single` / $0 wages / $0 expenses when no profile exists (tax.rs:90–95) and can
   show a full SE figure. The D3 replacement snippet omits the profile gate
   (`compute_se_tax(...profile params...)` — which params, when `profile` is `None`?) while
   claiming to "mirror the cmd/tax.rs:79–106 assembly … exactly". If D3 keeps the current default,
   then for a no-profile year with business income: the Tax **tab shows** an SE tax figure while
   the TUI **export omits** `schedule_se.csv` (D2 is profile-gated, matching admin.rs/tax.rs) —
   an internal tab-vs-export inconsistency AND a tab-vs-CLI-report divergence.
2. **Outcome-arm placement.** The current TUI SE block lives **inside** the
   `TaxOutcome::Computed` arm (tax.rs:53–126). `report_tax_year` computes/renders Schedule SE
   **independently of the outcome** (tax.rs:79–106 sits outside the outcome match) — the CLI shows
   the SE section (or the "wage base unavailable" note) even for a NotComputable year. D3 says
   "replace lines ~89–126" without pinning where the replacement renders.
**Fix (exact):** pin both in D3: (a) profile-gated exactly as `cmd/tax.rs:79–106` including the
`match profile` wrapper — no profile ⇒ no SE section in the tab AND no `schedule_se.csv` in the
export (state the intentional behaviour change from today's TUI, which silently assumed Single);
(b) render the SE text outside/after the `Computed` arm so NotComputable years still show the
section, matching the CLI report. Add KATs: (i) business income + table + **no profile** → no SE
section in `render_tax_content` and no `schedule_se.csv` from `do_export`; (ii) NotComputable
year + profile + business income → SE section present in the tab.

**[I3] KAT-E4 does not test what Task 2 claims ("report output vs TUI export CSVs"), and is partly
self-referential.** As specced, the test compares `do_export`'s `schedule_se.csv` values against a
`compute_se_tax` call the **test itself assembles** "with the same profile". A mirrored assembly
error (e.g. swapped `w2_ss_wages`/`w2_medicare_wages`, wrong-year table) can pass if the test
mirrors `export.rs`'s assembly — the exact drift the KAT claims to guard. Parity for
`form8949`/`schedule_d`/`form8283` is genuinely by-construction (same private writers, same
`state`), so the SE CSV + `donation_details` passthrough are the only assembly-sensitive artifacts.
**Fix (exact):** either (a) hard-code **golden expected figures** for the pinned fixture (TY2025,
Single, mining $50,000, W-2 SS $30,000, expenses $5,000 — independently derived values for
`net_se_earnings`/`total_se_tax`/`deductible_half`), and set `w2_ss_wages ≠ w2_medicare_wages` in
the fixture so parameter swaps change the answer; and/or (b) byte-compare the four form CSVs
between the CLI path (`write_csv_exports` on the same state/year/se_result/donation_details) and
`do_export`'s output — this needs a narrowly-scoped, documented test-only exception to the D5
`write_csv_exports` rule (mirror the existing `cmd::init::run` test exception; scope it to
`export.rs` tests). Option (a) has no gate impact and is sufficient; state the choice in the spec.

**[I4] The extended D5 gate is review-time-only while this change converts `btctax-tui` from
provably-write-free to write-capable — mechanize it.** The inherited gate (SPEC_tui_readonly_viewer
"Review-level: the whole-diff greps…") was backed by a compile-level guarantee (immutable
`Session`). The new isolation invariant — write-class I/O and `write_form_csvs` only in
`export.rs`; `export_snapshot`/`write_csv_exports`/`save(`/`append_`/`conn(` nowhere — has **no
compile-level backstop** (`write_form_csvs` is `pub` and callable from any TUI module) and no CI
automation (`ci.yml` has no grep step). Between whole-diff reviews the single load-bearing
invariant is unenforced. **Fix (exact):** add KAT-E10 — a `#[test]` in `btctax-tui` that walks
`crates/btctax-tui/src/`, applies the D5 pattern table (with its two documented exceptions:
`cmd::init::run` in test code, write-class calls in `export.rs`), and fails on any other hit. ~30
lines, runs on every `cargo test`/CI. Keep the whole-diff review grep as the independent second
layer; the spec's "CI item (whole-diff review Task 2 KAT)" line should become this real test.

### Minor

**[M1] `time` is a dev-dependency only in `btctax-tui`** (Cargo.toml `[dev-dependencies] time =
"0.3"`), but D2 puts `time::OffsetDateTime` in production types (`ExportConfirmState.export_now`)
and the `e`-keybinding calls `OffsetDateTime::now_utc()`. Promote `time` to `[dependencies]` and
add `crates/btctax-tui/Cargo.toml` to the Task 1 file list.

**[M2] The D5 gate table is not closed over write-class `std::fs` verbs.** It lists `File::create`
but not `OpenOptions` (what `open_owner_only` actually uses — the obvious bypass spelling),
`fs::write`, `create_dir`/`create_dir_all`, `set_permissions`, `fs::copy`, `fs::rename`,
`fs::remove_*`. The Task-2 checklist mentions `OpenOptions::new().write(true)` but the normative
table (and the KAT-E10 test from I4) must carry the full write-class list. Read-class
(`std::fs::read` in unlock.rs tests) stays permitted, as the spec already notes.

**[M3] Stale absolute read-only doc-comments are not scheduled for update.** main.rs:10–11,
app.rs:3–4, unlock.rs:11–13, tabs/tax.rs:3 all state "STRICTLY READ-ONLY … MUST NOT" in the
original absolute form. After this spec those statements are false as written (the binary writes,
under the re-scoped guarantee). Task 1 must update them to the re-scoped guarantee text ("never
writes the vault or any decrypted image; writes only the four form CSVs via export.rs on explicit
confirmation") — a load-bearing safety statement must not drift.

**[M4] Modal-blocking KAT gaps.** On Viewer, `Esc` currently quits (main.rs:146); the modal's
Esc-closes semantics depend on modal dispatch preceding the Viewer arm. KAT-E2 would catch the
ordering bug via `export_modal.is_none()`, but should also assert `!app.should_quit` (Esc closed
the modal, did not quit). Add one more assert/case: `q` while the modal is open → ignored (modal
still or just closed per design, app not quit, nothing written).

### Nit

**[N1]** `vault_path.parent()` returns `Some("")` (not `None`) for a bare relative filename, so
`unwrap_or(Path::new("."))` is nearly dead code; `"".join(name)` yields a cwd-relative dir —
behaviourally fine, but say what actually happens.

**[N2]** The re-scoped guarantee sentence says "writes only named form CSVs" — the export also
creates one directory. The modal already shows the dir; make the guarantee sentence say
"…the export directory and the four named form CSVs…" so it is literally true.

**[N3]** `ExportConfirmState` is freely constructible, so "the confirmation modal gates the ONLY
call site" is procedural (enforced by KATs + review), not type-enforced. Acceptable for this scope;
record it so a future reviewer doesn't over-read the type as a proof.

---

## Answers to the gate questions

**1. Re-scoped guarantee — airtight?** Nearly. Strong points: `export_snapshot` is structurally
unreachable (the TUI holds neither `Session` nor `Passphrase` after unlock — unlock.rs drops both);
`do_export` consumes only the in-memory `Snapshot` + a path, so no code path can reach the vault
file or a decrypted image; dir creation happens only inside the Enter path, so Esc-cancel genuinely
writes nothing (KAT-E2 is honest); the KAT-E3 bytes test extension is the right end-to-end check.
The holes are I1 (the false no-clobber claim + pre-existing-dir/symlink edge — closed by exclusive
create) and I4 (the only-in-export.rs isolation is review-time-only). With I1 + I4 fixed the
enforcement stack is: structural unreachability + fixed filenames + exclusive fresh 0o700 dir +
mechanized source gate + whole-diff review + bytes KAT — airtight for this threat model.

**2. Pub surface.** Correct and minimal. Today `write_csv_exports` writes `lots.csv`,
`disposals.csv`, `removals.csv`, `income.csv` (always) + the four form CSVs (when
`tax_year`/`se_result` present); `export_snapshot` additionally writes `snapshot.sqlite`. The new
`write_form_csvs(out_dir, state, year: i32, se_result, donation_details)` excludes all five
non-form artifacts by construction and forces year-scoping via `i32` (not `Option`). Signature
matches the private writers' needs exactly. SE assembly mirrors cmd/tax.rs — **but pin the
profile gate explicitly (I2)**. Figure-parity KAT present but under-powered (I3).

**3. render_schedule_se reuse for the Tax tab.** Sound and the right call — single source of truth
kills the drift class; `render_schedule_se` is genuinely `pub` (render.rs:1118) and every input is
Snapshot-derivable (`profiles`, `tables`, `se_net_income(&state, year)`). The CLI text under
`Paragraph { wrap }` is acceptable (the tab is prose, not a fixed-column table); losing the
condensed one-liner is a fair trade. The two unpinned semantics (profile gate, outcome-arm
placement) are I2 — fix in-spec, not in-implementation.

**4. Export dir + timestamp.** Injected `export_now` matches the existing optimize/reconcile
convention (verified); the `time` format string is correct for `20251024-143022Z`; the modal shows
the exact dir + file list; PII posture is documented in both Hard Constraints and the modal text.
The no-clobber mechanism is wrong (I1); `time` dep placement (M1); parent() nit (N1). Otherwise
reasonable and predictable for an MVP — file-picker deferral is right.

**5. Bytes test + §6017 pin + KAT genuineness.** KAT-E3 (vault byte-identical across a full export
cycle, subsuming the open→drop test) is genuine and the correct top-level assertion. The §6017
coordination pin is precise and actionable (text-only, `render_schedule_se` only, no CSV
headers/columns — and note form8283.csv already carries `#` comment lines, so the "frozen writer
surface" phrasing correctly scopes to *no new* comments). The KAT plan is genuine — E1/E2/E5/E6/
E7/E8/E9 test real behaviour with correct expected strings (all E7 assertion strings verified
verbatim against render.rs); E4 needs strengthening (I3); add the I1/I2/M4 cases.

**6. Scope.** Right-sized. One `pub fn`, one designated write module, one modal, one keybinding,
plus the disclosure fold-in (coherent — same SE assembly). Mutating-TUI, snapshot export,
all-years dumps, PDF/FDF, file-picker all explicitly out of scope; FOLLOWUPS are correctly routed.
Ceremony is proportionate to a security-relevant change.

## `write_form_csvs` misuse-exposure rating: **Low**

Rationale: (1) it is strictly **narrower** than the already-`pub` `write_csv_exports` — same
caller-supplied `out_dir`, a subset of files, and no `snapshot.sqlite` path (that remains solely
`export_snapshot`, which needs a live `Session` the TUI does not hold); it introduces no new
*class* of exposure. (2) Fixed four filenames — it cannot target the vault file by name; worst
misuse is dropping four 0o600 tax-data CSVs at a caller-chosen path, or the pre-existing-dir/
symlink truncation edge — closed on the only real call path by I1's exclusive create. (3) Path
containment being the caller's job matches every existing writer in the codebase
(`export_snapshot`, `write_csv_exports`, `backup_key` all take caller paths). Cheap hardening:
one doc-comment line on `write_form_csvs` — "callers must pass a freshly-created or trusted
directory; this function truncates the four fixed filenames in `out_dir`."

## Gate disposition

**Blocked** pending fold of I1–I4 (+ M1–M4 recommended in the same fold; nits at author's
discretion). All fixes are spec-text/KAT-level — no design rework required; the architecture
(Snapshot-only export module, structural Session/Passphrase unreachability, single gated call
site) is correct. Re-review required after the fold per §2.

---

# Round 2 — re-review (post-fold)

- **Artifact re-read in full:** `design/SPEC_tui_export.md` (R1 findings folded, tagged `[R0-…]` inline)
- **Verdict:** **0 Critical / 0 Important / 1 Minor / 3 Nit — R0 GREEN. Ready to implement.**
  The Minor is a one-sentence D5 addendum (KAT-E10 comment-stripping) that cannot weaken the
  guarantee (its failure mode is a red test, not a hole); it may be folded now or resolved during
  Task 1 under the Task-2 whole-diff check. The Nits are author's-discretion.

## Fold verification, finding by finding

**[I1] CLOSED.** D1b adds `fsperms::mkdir_owner_only_exclusive` (`recursive(false)` + mode 0o700,
fails `AlreadyExists`; non-recursive is safe — the parent is the vault's parent, which exists by
construction). D2/`do_export` call it **before** `write_form_csvs`; the stale no-clobber claim is
explicitly retracted and corrected ("that was drift (it is mkdir-p)"); `write_form_csvs` keeps the
tolerant internal `mkdir_owner_only` (now a no-op on the fresh dir) with the path-containment
doc-comment stating the caller precondition; KAT-E11 pre-creates the exact deterministic dir with
a sentinel file → `Err`, no CSVs, sentinel untouched. All three edges close: (1) same-second
re-export → `AlreadyExists`, nothing written; (2) unowned/pre-created dir → creation fails, the
user never writes into a dir they don't own or whose mode wasn't forced; (3) symlink truncation →
symlinks can only pre-exist inside a pre-existing dir; exclusive create guarantees a fresh, empty,
user-owned 0o700 dir, and nobody else can inject entries into it afterwards. Residual noted as
N-R2-2 (non-blocking).

**[I2] CLOSED.** The recon section now states the true `cmd/tax.rs:79–106` shape (the
`match profile` wrapper; SE assembly outside the outcome match) and the true TUI divergences
(Single/$0 default at tax.rs:90–95; SE inside the `Computed` arm). Both surfaces are pinned:
D2's export assembly carries the literal `match profile { Some(p) => …, None => None }`; D3's
replacement renders the SE section outside/after the outcome match with the same profile gate.
Both behaviour changes are declared intentional convergences to the CLI. Tests: KAT-E7 gains the
profile-gate case (no profile → no `"Schedule SE"` substring) and the outcome-independence case
(NotComputable + profile + business income → SE section present); KAT-E9(b) pins the export side
(business income, no profile → no `schedule_se.csv`); KAT-E1's fixture now names the required
`TaxProfile`. Tab, export, and CLI report now provably agree — the drift class is closed by
single-source-of-truth + gating parity + tests on both surfaces.

**[I3] CLOSED — goldens independently re-derived and verified against source.** Premises checked
against current code: TY2025 `ss_wage_base = 176,100` (`btctax-adapters/src/tax_tables.rs:336`);
Single Additional-Medicare threshold `$200,000` (`btctax-core/src/tax/tables.rs:167–173`,
statutory); `SeTaxResult.net_se` is the POST-expense net (`se.rs` — `max(0, gross − expenses)`),
so the CSV `net_se_earnings` golden `40000` is correct (and correctly integer-formatted — the raw
Decimal is never cent-scaled); SS cap = `max(0, wage_base − w2_ss)` and Addl threshold =
`max(0, threshold − w2_medicare)` per `compute_se_tax`; `deductible_half =
round_cents((ss + medicare)/2)` — **excludes addl** per §164(f)(1). Re-derivation:

| Component | Derivation | Golden | Reproduces |
|---|---|---|---|
| net_se | max(0, 100,000 − 60,000) | 40000 | ✓ |
| base | round_cents(40,000 × 0.9235) | $36,940.00 | ✓ |
| ss | 12.4% × min(36,940, 176,100−150,000 = 26,100) — **cap binds** | $3,236.40 | ✓ (cross-check vs SPEC_se_chunkB_expenses.md:95 — same binding cap, same figure — genuine) |
| medicare | 2.9% × 36,940 | $1,071.26 | ✓ |
| addl | 0.9% × max(0, 36,940 − (200,000−170,000 = 30,000)) = 0.9% × 6,940 — **threshold binds** | $62.46 | ✓ |
| total | 3,236.40 + 1,071.26 + 62.46 | $4,370.12 | ✓ |
| deductible_half | (3,236.40 + 1,071.26)/2, addl excluded | $2,153.83 | ✓ |

Swap-catching confirmed: both W-2 values bind AND differ (150,000 ≠ 170,000); a parameter swap
gives ss cap 6,100 → ss $756.40 (0.124 × 6,100 ✓) and addl threshold 50,000 > 36,940 → addl $0 ✓
— four of six figures change, so the hard-coded goldens fail on a swap. The
`donation_details`-passthrough assert (known `donee` label in exported `form8283.csv`) is present.
The KAT no longer self-assembles its expectation. Closed.

**[I4] + [M2] CLOSED.** D5 is now the full normative table (adds `OpenOptions`, `fs::write`/
`write_owner_only`, `create_dir`/`create_dir_all`/`DirBuilder`, `set_permissions`/`fs::copy`/
`fs::rename`/`fs::remove_`, `File::options`, and the new `mkdir_owner_only_exclusive`/`fsperms`
tokens), with exactly two documented exceptions (test-region `cmd::init::run` + fixture write
verbs; `export.rs` for write-class/`write_form_csvs`). KAT-E10 mechanizes it: in-crate `#[test]`,
walks `src/`, scans non-test regions, fails with `file:line`, includes the plant-a-token
self-check (test-the-tester), runs on every `cargo test`/CI; the whole-diff grep remains the
independent second layer over the full tree. This is compile-time-adjacent in the meaningful
sense: mechanically enforced on every build's test run, with the type-level gap honestly recorded
([R0-N3] + the sealed-token FOLLOWUP). Closed — subject to M-R2-1 below (comment handling), which
affects implementability, not strength.

**[M1] CLOSED** — `time` promotion specced in recon + D2 + Task 1 file list
(`crates/btctax-tui/Cargo.toml` present, pinned explicit version noted).
**[M3] CLOSED** — all four stale sites enumerated with exact replacement wording; per-module
"performs no writes" retention for non-export modules; Task-2 check added.
**[M4] CLOSED** — KAT-E2 now asserts `!app.should_quit` on Esc + the `q`-while-modal ignored case;
D4 pins modal dispatch BEFORE the Viewer arm with the *reason* (Viewer `Esc` quits, main.rs:146)
stated in both recon and D4; Task-2 check added.
**[N1] CLOSED** (the `Some("")` note, with the "don't 'fix' it" pin). **[N2] CLOSED** (guarantee
kept verbatim + the one-created-directory reading note). **[N3] CLOSED** (recorded procedural
limitation + sealed-token FOLLOWUP).

## New findings (round 2)

**[M-R2-1] Minor — KAT-E10 will fail on legitimate, must-keep comments; pin comment-stripping.**
The scanner as specced matches raw text of non-test regions, but the guarantee documentation
itself legitimately names forbidden tokens in comments that M3 does NOT remove: e.g.
unlock.rs:87–88/94 ("`save()` takes `&mut self`…", "`let mut session` would make `save()`
callable"), unlock.rs:111 ("never `session.conn()` directly") — these contain `save(`/`conn(`
and MUST stay (they document the compile-level guarantee). Failure mode is a loudly red test —
not a guarantee hole — but the risk is an implementer "fixing" it by deleting the safety
documentation. **Fix (one sentence in D5's mechanization paragraph):** strip `//`/`///`/`//!`
comments before scanning (or scan only non-comment code); comments may legitimately name the
forbidden calls. Non-blocking (below the 0C/0I gate); fold now or resolve in Task 1 — the Task-2
whole-diff check already re-verifies KAT-E10 coverage either way.

**[N-R2-1] Nit — D3 ordering parenthetical is inconsistent with the current layout.** "(before
the charitable-deduction and advisory-blockers sections)" — the charitable-deduction block
currently lives INSIDE the `Computed` arm (tax.rs:128–141), so SE-after-the-match renders after
it unless the charitable block also moves out. Either move it out too (it is year-scoped and
profile-independent) or drop the parenthetical; no figure or disclosure is affected and the KATs
pin presence, not position.

**[N-R2-2] Nit — recorded residual: post-create TOCTOU on adversarial layouts.** After
`mkdir_owner_only_exclusive` succeeds, an attacker with write access to the vault PARENT could in
principle race-swap the fresh dir for a symlink before the first `open_owner_only`. Requires a
non-default layout (the default vault parent is 0o700, vault.rs:50) plus winning a race on a
user-triggered action; the identical residual exists for every path-based writer in the codebase
(`export_snapshot`, `write_csv_exports`, `backup_key`); full closure needs dirfd/`openat`-style
I/O — out of proportion for this threat model. Recorded so the guarantee's practical anchor
(owner-only default layout) is explicit; no action required.

**[N-R2-3] Nit — KAT-E10 skips test regions entirely,** so the five "FORBIDDEN everywhere" tokens
(`save(`/`append_`/`conn(`/`export_snapshot`/`write_csv_exports`) are review-enforced only within
`#[cfg(test)]` blocks. Cheap tightening: also scan test regions for those five (+ `cmd::` with the
`init::run` exception). Non-blocking — the layered split is stated and the whole-diff grep covers
the full tree.

## Cross-cutting confirmation

No other new findings. Internally consistent (D1 tolerant-mkdir vs D2 exclusive-create division of
responsibility is coherent and documented on both sides; the modal `files` list and `do_export`
both use the same profile-gated assembly; KAT-E5's 0o700 assert is now guaranteed by exclusive
creation). The §6017 coordination pin is intact and *improved* (Task 2 now correctly scopes the
CSV-comment freeze to additions, given `form8283.csv`'s pre-existing `#` lines). Right-sized: the
fold added one fsperms helper, one test, and KAT extensions — proportionate; nothing was
gold-plated; the mutating TUI remains out of scope.

## Gate disposition (round 2)

**R0 GREEN — 0 Critical / 0 Important. Ready to implement** (Task 1 TDD per the spec's plan).
M-R2-1 is a one-sentence spec addendum recommended before or during Task 1; N-R2-1/2/3 at
author's discretion / FOLLOWUPS.
