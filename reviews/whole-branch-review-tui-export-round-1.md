# Whole-branch review — feat/tui-export (round 1)

- **Branch / head:** `feat/tui-export` @ `585db86` (single commit over base `4125db3`)
- **Artifact under review:** the full diff `4125db3..585db86`
  (package `.superpowers/sdd/review-4125db3..585db86.diff` verified byte-equivalent to
  `git diff 4125db3..585db86` — differences are context-line width only; every +/− line identical)
- **Spec:** `design/SPEC_tui_export.md` (R0 GREEN, 2 rounds) + `reviews/R0-spec-tui-export-round-1.md`
- **Reviewer:** independent whole-diff (Phase E), security gate: FIRST write capability in the
  never-writes `btctax-tui` viewer
- **Verdict:** **NOT GREEN — 0 Critical / 1 Important / 3 Minor / 5 Nit.**
  The re-scoped guarantee is airtight at this head: no path to the vault or a decrypted image
  exists, `export.rs` is verifiably the only write-capable module, the mechanized gate works
  (mutation-tested live), and every empirical KAT re-run is green. The single blocking finding is a
  **weakened KAT**: the spec-mandated donee-passthrough assertion in KAT-E4 was silently dropped —
  one of the exactly two assembly-sensitive artifacts R0-I3 identified is untested end-to-end.
  Fix is ~15 lines of test code; after it lands + re-run, this branch is ready to merge first.

---

## 0. Empirical verification log (all re-run at `585db86`)

### 0.1 Test suite

- `cargo test -p btctax-tui` → **74 passed, 0 failed** (matches the report's claim exactly).
  E1/E2/E3/E4/E5/E6/E7a–e/E8/E9a/E9b/E10/E11 all present and green;
  `vault_file_bytes_unchanged_after_open_build_snapshot_drop` (KAT-E3 extension) green.
- `cargo test -p btctax-cli -p btctax-store` (the two other touched crates) → green
  (regression on `render.rs` / `fsperms.rs` changes).

### 0.2 KAT-E10 mechanized gate — mutation-tested live (test-the-tester, beyond the built-in self-check)

| Mutation planted (real tree, then reverted) | Expected | Observed |
|---|---|---|
| `save(` inside a string const in `draw.rs` non-test region (everywhere token) | FAIL | **FAILED** — `draw.rs:183 — forbidden token "save(" (everywhere rule, non-test region)` |
| `fs::write` inside a string const in `app.rs` non-test region (write-class token) | FAIL | **FAILED** — `app.rs:208 — forbidden write-class token "fs::write" (export.rs-only rule)` |
| `write_csv_exports` inside a string const in `main.rs` **test region** (everywhere token) | FAIL | **FAILED** — `main.rs:961 — forbidden token "write_csv_exports" found in test region` |
| Comment line naming `save( conn( write_csv_exports export_snapshot fs::write` in `draw.rs` | PASS | **PASSED** — comment-stripping works; the unlock.rs guarantee doc-comments (`save()` at :87/:94, `conn()` at :92/:111) survive on the clean run [M-R2-1 honored] |
| `("http://x", "save(hidden")` on one line in `draw.rs` non-test region | (probe) | **PASSED — false negative demonstrated**; see M-1 |

Built-in self-check (plant-a-token via runtime string construction) present and asserted for
`save(` / `conn(` / `export_snapshot`. The extended D5 table is carried in full: all 6
everywhere-tokens (`save(`, `append_`, `cmd::`, `conn(`, `export_snapshot`, `write_csv_exports`)
and all 17 write-class token spellings from the normative table (incl. `OpenOptions`,
`File::options`, `fs::write`, `write_owner_only`, `create_dir`/`create_dir_all`/`DirBuilder`,
`set_permissions`, `fs::copy`/`fs::rename`/`fs::remove_`). Tree restored clean after mutation
testing (`git status` — no tracked modifications).

### 0.3 KAT-E11 (exclusive create) — re-run green

`e11_pre_created_dir_fails_nothing_written` passes: pre-created deterministic dir + sentinel →
`do_export` returns `Err`; `form8949.csv`/`schedule_d.csv`/`form8283.csv` do NOT exist; sentinel
byte-identical (`b"sentinel content"`). The symlink/pre-created-dir truncation edge is closed.
(Nit N-4: the test asserts `is_err()` rather than the `AlreadyExists` kind specifically — genuine
anyway, since the no-CSV + sentinel asserts prove nothing was written.)

### 0.4 Independent grep layer (full D5 table over `crates/btctax-tui/src/`, raw text — no comment stripping)

- **Everywhere tokens:** hits ONLY in (a) unlock.rs guarantee doc-comments (must-keep, comment
  lines), (b) export.rs E10 test region (the scanner's own token-list strings, self-check, and
  assert messages). **Zero production-code hits. `export_snapshot` and `write_csv_exports` are
  never named in any call position anywhere in the crate.**
- **`cmd::`:** test regions only — all hits are `btctax_cli::cmd::init::run` fixture setup
  (documented exception 1) plus E10's own token list.
- **Write-class tokens:** non-test hits ONLY in `export.rs` (module doc, `use fsperms`, the
  `mkdir_owner_only_exclusive` call at export.rs:121, the `write_form_csvs` call at
  export.rs:143). Test-region hits are E11/self-check fixtures. **`export.rs` is the only
  write-capable module — confirmed.**
- **`do_export` call sites (non-test): exactly one** — `main.rs:133`, inside the modal
  `KeyCode::Enter` arm. The modal gates the only call site (procedural per [R0-N3], as recorded).
- **`OffsetDateTime::now_utc()`: exactly one hit** — `main.rs:208`, the production `e`
  keybinding. All test paths use injected fixed timestamps (`time::macros::datetime`).
  Determinism pin honored.

### 0.5 Structural unreachability (Session/Passphrase)

`unlock.rs::attempt_open` (read at HEAD): `Session` held in an **immutable** binding; `pp`
dropped immediately after `Session::open`; only the `Snapshot` escapes
(`OpenOutcome::Success(Box<Snapshot>, year)`); `Session` drops at scope exit. `Session` is named
nowhere else in production TUI code (grep: unlock.rs only; two test-only holders for the
lock-contention tests). `App` holds `vault_path`/`unlock`/`snapshot` — no Session, no Passphrase.
`export_snapshot` needs a live `Session` → **structurally unreachable from the export path**,
independent of the gate. Intact.

### 0.6 Modal / cancel semantics (KAT-E2 + code inspection)

- Modal dispatch sits at the TOP of `handle_key`, **before** the screen match (and therefore
  before the Viewer arm whose `Esc` quits) — [R0-M4] honored, with an early `return` so the modal
  consumes every key.
- `Esc` → `app.export_modal = None` only; E2 asserts `!should_quit`, `export_status.is_none()`,
  **and the export dir does not exist** (no dir creation on the cancel path — dir creation happens
  only inside `do_export`). `q` while modal open → swallowed, asserted.
- `Enter` → `take()` the modal, `do_export`, status set on Ok/Err; `AlreadyExists` surfaces as
  `Export error: …` with nothing written.

### 0.7 `mkdir_owner_only_exclusive` (fsperms.rs, read at HEAD)

Unix impl: `DirBuilder::new().recursive(false).mode(0o700).create(path)` — genuinely exclusive
(`recursive(false)` errors `AlreadyExists` on an existing dir), 0o700 at creation (no
create-then-chmod window). Non-Unix: `recursive(false)` create (ACL-inherited), still exclusive.
Called at export.rs:121 **before** `write_form_csvs` (export.rs:143). Matches D1b exactly.

### 0.8 `write_form_csvs` (render.rs, read at HEAD)

Enumerated body: `fsperms::mkdir_owner_only(out_dir)?` (tolerant, documented as the caller's
exclusive-create precondition) → `write_form8949_csv` → `write_schedule_d_csv` →
`write_form8283_csv` → `write_schedule_se_csv` iff `se_result.is_some()`. **Exactly the four form
writers — no `lots/disposals/removals/income` dump writers, no snapshot, no events.** The
path-containment doc-comment is present verbatim per D1. The `render.rs` diff is a **single hunk**
(+31 lines) adding this fn after the `write_csv_exports` writer block; `render_schedule_se`'s text
and all four private writers are byte-untouched; no new CSV header comments anywhere. **The
burndown-lane pin (writer surface frozen; §6017 lane owns `render_schedule_se` text) is
respected from this side.**

### 0.9 KAT-E4 goldens — independently re-derived against `compute_se_tax` source

Premises verified at HEAD: TY2025 `ss_wage_base = 176,100` (tax_tables.rs:336, SSA TY2025);
Single Additional-Medicare threshold `200,000` (tables.rs); `compute_se_tax` (se.rs:99): net_se =
max(0, gross − expenses); `ss_cap = max(0, wage_base − w2_ss_wages)`;
`addl_threshold = max(0, threshold − w2_medicare_wages)`;
`deductible_half = round_cents((ss + medicare)/2)` — **excludes addl** (§164(f)(1) comment in
source). Param order `( … w2_ss_wages, w2_medicare_wages, schedule_c_expenses)` confirmed against
the signature — the E4 fixture passes `(150_000, 170_000, 60_000)` in the correct positions.

| Component | My derivation | Golden asserted | CSV col |
|---|---|---|---|
| net_se_earnings | max(0, 100,000 − 60,000) = 40,000 (Decimal scale 0 → `"40000"`) | `40000` ✓ | 0 |
| se_base_9235 | round_cents(40,000 × 0.9235) = 36,940.00 | `36940.00` ✓ | 1 |
| ss_component | cap = 176,100 − 150,000 = 26,100 < base → **cap binds**; 0.124 × 26,100 | `3236.40` ✓ | 2 |
| medicare | 0.029 × 36,940.00 | `1071.26` ✓ | 3 |
| addl | thr = 200,000 − 170,000 = 30,000; 0.009 × (36,940 − 30,000) — **threshold binds** | `62.46` ✓ | 4 |
| total_se_tax | 3,236.40 + 1,071.26 + 62.46 | `4370.12` ✓ | 5 |
| deductible_half | (3,236.40 + 1,071.26)/2, addl excluded | `2153.83` ✓ | 6 |

Column indices match `write_schedule_se_csv`'s header order (render.rs:767–777). W-2 values
**bind and differ** (150,000 ≠ 170,000). **Swap check re-derived:** swapping gives
ss_cap = 176,100 − 170,000 = 6,100 → ss = 756.40, and addl_thr = 200,000 − 150,000 = 50,000 >
36,940 → addl = 0 — **both ss AND addl flip**, so total/deductible_half flip too; the goldens
fail on a swap. The asserts are **hard-coded literal strings** read from the CSV text — not
self-assembled. ✓ (But see I-1: the E4 donee-passthrough assert is missing.)

### 0.10 SE parity (E7) — assembly compared line-by-line against `cmd/tax.rs:79–105`

`tabs/tax.rs` now: SE section **outside/after** the outcome match (Computed and NotComputable
both fall through); wrapped in `match snap.profiles.get(&year) { Some(p) => …, None => None }`;
`gross_se = se_net_income(&snap.state, year)`; `table_present = table_opt.is_some()`;
`compute_se_tax(&state, year, p.filing_status, t, p.w2_ss_wages, p.w2_medicare_wages,
p.schedule_c_expenses)`; `render_schedule_se(year, se_result.as_ref(), gross_se, table_present,
p.schedule_c_expenses, p.w2_ss_wages, p.w2_medicare_wages)` — **identical shape and identical
parameter order to the CLI assembly** (verified against `cmd/tax.rs:79–105` at HEAD).
`export.rs::do_export` carries the same profile-gated assembly. E7a–e green: disclosure lines
(a–c cases), profile gate (no profile → no `"Schedule SE"`/`"§1401"` substring), and
outcome-independent placement (NotComputable + profile + business income → SE section present,
alongside `NOT COMPUTABLE`). Both intentional behavior changes are implemented exactly as the
spec declares them; no `FilingStatus::Single`/$0 default remains (the old fallback block is
deleted; `FilingStatus` import removed from tax.rs). Charitable block stays inside the `Computed`
arm — the N-R2-1-sanctioned resolution (KATs pin presence, not position).

### 0.11 Everything else on the Task-2 checklist

- **KAT-E3 bytes test:** extended in place — vault bytes captured before, asserted identical
  after open→drop AND after a full `do_export` cycle. Green.
- **`time` in `[dependencies]`** (pinned `"0.3"`, removed from dev-deps) ✓ [R0-M1] (see N-3).
- **E6:** `export_dir_for` pure; fixed timestamps → `btctax-export-20251024-143022Z` /
  different ts → different suffix. Green. The `Some("")` parent nit is documented in-code
  verbatim per [R0-N1] (not "fixed").
- **E5:** dir 0o700, files 0o600 asserted (`#[cfg(unix)]`). Green.
- **Doc-comments [R0-M3]:** all four mandated sites (main.rs, app.rs, unlock.rs, tabs/tax.rs)
  carry the exact re-scoped wording + "This module performs no writes." for non-export modules;
  export.rs carries the guarantee header. (See N-1 for the nine untouched sibling modules.)
- **Modal content (D4):** dir, file list (schedule_se.csv iff SE result), "The vault is never
  written.", PII/0o600 statement, `[Enter] Confirm  [Esc] Cancel — writes nothing`. Footer gains
  `e: export CSVs`; `export_status` shown in footer, cleared on next non-modal key.
- **Synthetic-only:** every test uses `tempfile::tempdir()` + `cmd::init::run` fresh vaults +
  synthetic `LedgerState`/`TaxProfile`; no real vault path appears anywhere.
- **E8:** `e` with no snapshot → no modal. Green.
- **E9a/E9b:** no SE income / no profile → `schedule_se.csv` neither listed nor written;
  agrees with the E7d tab gate — tab, export, and CLI report align.

---

## Findings

### Important

**[I-1] KAT-E4 is weakened vs the spec: the donee-passthrough assertion is missing — the
`donation_details` passthrough is untested end-to-end from the TUI export path.**
Spec KAT-E4 (mandated text, present in the R0-GREEN artifact and re-verified by R0 round 2 as
"present"): *"add one assertion that a fixture donation's known `donee` label appears in the
exported `form8283.csv`."* At `585db86` there is **no** such assertion: `grep -rn donee
crates/btctax-tui/` → zero hits; the E4 fixture (and every export-path fixture) uses
`donation_details: BTreeMap::new()`, and no TUI test ever exercises a non-empty passthrough —
form8283 is only ever asserted to *exist*. R0-I3 identified exactly two assembly-sensitive
artifacts in the export path (the SE CSV — covered by the goldens — and the `donation_details`
passthrough); the second has no coverage. The passthrough itself is correct by inspection
(`do_export` passes `&snap.donation_details` straight through to `write_form_csvs` →
`write_form8283_csv(out_dir, state, year, details)`), so this is **not** a production hole — but
under this workflow a silently dropped spec-mandated KAT element is a blocking finding, and the
report's "All KATs implemented and green" claim is inaccurate on this point.
**Fix (~15 lines, test-only):** add a donation removal + a `DonationDetails { donee: "<known
label>", … }` entry to the E4 fixture's state/map (or a small dedicated test), run `do_export`,
and assert the donee label appears in the exported `form8283.csv` text. Re-run the suite.

### Minor

**[M-1] E10's comment-stripping creates a demonstrated false-negative window: a `//` inside a
string literal truncates the scan line.** Empirically shown at this head: planting
`const _P: (&str, &str) = ("http://x", "save(hidden");` in `draw.rs`'s non-test region **passes**
the gate (the scanner truncates at the `//` in `"http://x"`, hiding the `save(` that follows on
the same line). The gate is an anti-accident backstop and the whole-diff review grep (raw text,
this document §0.4) is the independent second layer, so this is not a guarantee hole — and any
deliberate evasion defeats token-scanning anyway (as the self-check's own runtime-constructed
tokens demonstrate). Cheap hardening for a FOLLOWUP: treat `//` as a comment only when not inside
a string literal (or strip string literals first), keeping the M-R2-1 exemption for genuine
comments.

**[M-2] E10 exempts `export.rs`'s test region from the five everywhere-token scan.** Documented
in-code and in the report (the exemption exists because the scanner's own token-list strings and
assert messages contain the literal tokens). Consequence: a future test inside `export.rs` could
call the `pub` `write_csv_exports` without tripping the gate — review-enforced only there. My
grep confirms the current export.rs test region contains the five tokens solely as E10's own
scanner data (no call positions). FOLLOWUP-grade tightening: allowlist the specific E10 test fn
(or its line span) instead of the whole file's test region.

**[M-3] E11 asserts `is_err()` rather than the `AlreadyExists` error kind.** The
nothing-written + sentinel-untouched asserts make the test genuine regardless (any error before
any write satisfies the safety contract), but pinning the kind would catch a future refactor that
fails for the wrong reason (e.g. permission error masking a lost exclusivity guarantee).
One-line strengthen at author's discretion.

### Nit

**[N-1] Nine sibling `tabs/*.rs` modules retain the old `STRICTLY READ-ONLY: no Session, no
persistence, no mutations` line** (compliance, utils, tags, mod, income, disposals, tests, forms,
holdings). These are module-scoped statements that remain literally true (those modules perform
no writes), and the spec mandated exactly four sites — compliant. But `tabs/tax.rs` carried the
identical sentence and was re-scoped, so the crate now has two wordings for the same per-module
claim. Uniformity sweep at author's discretion.

**[N-2] `do_export` re-implements the profile-gated SE assembly inline instead of calling the
adjacent `se_result_for`** (same module, identical logic, ~20 lines apart). The modal `files`
list uses `se_result_for` while the write path uses the inline copy — intra-module drift between
"what the modal says" and "what is written" would currently require editing one and not the
other. Collapse to one call.

**[N-3] `btctax-tui`'s `time = "0.3"` declares no features but the crate uses
`time::macros::format_description` + `.format()` (needs `macros`, `formatting`).** It compiles
via feature unification with btctax-core/cli/adapters (always in this crate's graph, so no
breakage today); declaring `features = ["macros", "formatting"]` removes the latent dependency on
sibling crates' feature sets. Failure mode is a compile error, never a runtime hole.

**[N-4] The E10 self-check comment overstates:** "runtime string construction so no literal
forbidden token appears in this source file" — literal `"save("` / `"conn("` strings DO appear in
the self-check's comparison closures and assert messages (test region, which is exactly why the
M-2 exemption exists). Harmless; reword the comment to say the *planted file content* is
runtime-constructed and the file's own literals are covered by the test-region exemption.

**[N-5] `ExportConfirmState.files` is display-only** — `do_export` re-derives the SE result and
ignores `files`. Consistent today because the `Snapshot` is immutable and both derivations share
the same gate (see N-2); recorded so a future mutable-snapshot TUI doesn't inherit the assumption
silently.

---

## Gate questions

**Re-scoped guarantee airtight?** Yes, at this head. Stack verified live: structural
unreachability (Session/Passphrase dropped at unlock; grep-confirmed no Session anywhere else) +
fixed four filenames + exclusive fresh 0o700 dir before any write + single gated `do_export` call
site behind the modal + mechanized E10 (mutation-tested: 3/3 plants caught with file:line;
comment exemption works) + this review's raw-text grep as the second layer + KAT-E3 vault-bytes
end-to-end. Esc-cancel provably writes nothing (not even the dir). No path to
`export_snapshot`/`write_csv_exports` exists in any call position.

**Parity?** SE assembly is shape- and parameter-identical to `cmd/tax.rs:79–105` on both the tab
and export surfaces; goldens independently re-derived and swap-verified; tab/export/CLI agree on
the profile gate and outcome-independence. The one parity-adjacent gap is I-1 (untested
donation passthrough), which is coverage, not behavior.

**Burndown-lane pin?** Respected: `render.rs` changed by exactly one additive hunk
(`write_form_csvs`); `render_schedule_se` text and the CSV writer internals untouched; no new
CSV header comments; no §6017 content.

## Verdict

**0 Critical / 1 Important / 3 Minor / 5 Nit → NOT ready to merge yet.** The Important (I-1) is a
~15-line test-only fix: add the spec-mandated donee-passthrough assertion to KAT-E4 (or a small
dedicated KAT) and re-run the suite. No production code needs to change. Once I-1 is closed and
re-reviewed per §2, this branch is **ready to merge FIRST** — the guarantee stack is sound and
the render.rs surface is frozen exactly as the burndown lane expects, so the burndown lane can
rebase on top cleanly. M-1/M-2/M-3 and the nits are non-blocking (FOLLOWUPS / author's
discretion).
