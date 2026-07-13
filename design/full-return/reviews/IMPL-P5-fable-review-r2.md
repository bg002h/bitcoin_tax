# IMPL-P5 — Fable independent re-review r2 — fold commit `c3af8f6`

**VERDICT: GREEN (0C/0I).** Every r1 blocking finding is genuinely fixed — verified against source, by KAT, and by driving the built binary — and the fold introduced no new Critical/Important defects. The full gate surface is green at `c3af8f6`. Three non-gating Nits noted below.

## Gate evidence (all at `c3af8f6`, clean tree)

- `cargo test --workspace --locked`: **82 suites, 1572 passed, 0 failed** (exit 0)
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
- `cargo run -p xtask -- check-isolation`: OK (ureq/rustls confined to btctax-update-prices)
- FROZEN files: `git diff 059ec2a..HEAD -- tax/{types,compute,se}.rs` = **0 bytes**
- `btctax limitations` output **byte-identical** to `crates/btctax-cli/LIMITATIONS.md` (cmp + the new N4 test)

## Per r1 finding

**C1 — FIXED.** Three-layer fix, all verified:
- *Guard*: `crates/btctax-cli/src/cmd/admin.rs:213-215` — `return_inputs::exists(session.conn(), tax_year)` checked **first**, before the attestation gate, before `mkdir_owner_only`, before any fill/write. `exists` (`return_inputs.rs:58`) is a per-year `SELECT 1` probe; a missing table is created-and-false, so vintage vaults can't panic past it. Grep of the whole workspace confirms `export_irs_pdf` (dispatched at `main.rs:591`) is the **only** caller of the `btctax_forms::fill_*` fillers outside tests — there is no other path that can emit a Schedule D. The parked P6 `fill_form_8959` has no CLI wiring (tests only), so it opens no bypass.
- *Zero bytes*: guard precedes even directory creation. Driven on the real binary: export for a full-return year → exit 2, loud message naming line 13 / lines 6-14 and pointing at `btctax report` + `btctax limitations`, `out_dir` empty.
- *Per-year*: same vault, `--tax-year 2025` (no `ReturnInputs`) exported f8949.pdf + schedule_d.pdf, exit 0. KAT `export_refuses_for_a_full_return_year_p5_c1` (`tests/export_irs_pdf.rs:432`) pins refusal variant + empty dir + the per-year carve-out.
- *Doc*: LIMITATIONS.md:66-80 now says "the crypto slice only", enumerates exactly what is filled, states "No full-return PDF exists yet," and documents the refusal with the missing-lines rationale. Accurate against `schedule_d.rs`/`form1040.rs` as verified in r1.

**I1 — FIXED.** LIMITATIONS.md:9-13 is now present-tense truth: watermark+attestation is pseudo-reconcile-only (matches `admin.rs` `state.pseudo_active()`), real-ledger exports unwatermarked, and the always-on full-return gate "does not exist yet, because no full-return PDF exists yet" — which is now enforced fact, since C1's refusal guarantees no full-return PDF can be produced. Follow-up `p5-i1-full-return-draft-attest-gate → P6` filed. (Residual: the Legal boilerplate at line 173 still says "Every full return is a DRAFT until you attest" — vacuously true today; see Nit-3.)

**I2 — FIXED.** `Advisory::OtherCreditsOmitted` (unconditional, `advisories.rs:195-198`) and `Advisory::RefundByPaperCheck` (fires iff `refund > 0`, line 201-203) exist, name the exact claim forms (8863/2441/8880/5695/8839, asserted by KAT), and both fired end-to-end on the driven binary. The doc's "Each fires a loud advisory" is now true for all four OMISSIONS rows in the sense the doc frames them ("benefits you *may be entitled to*"): CTC fires on captured dependents, EIC over-fires on the safe side, other-credits is unconditional, paper-check fires whenever there is a refund to deposit. **No noise leak**: `advisories_for` is called only on the `Provenance::ReturnInputs` dual-report branch (`cmd/tax.rs:238-265`) — crypto-only reports are untouched. The old "no advisories" test was honestly converted to pin the exact one-element set, preserving its not-noisy intent. SPEC needs no reconciliation — SPEC lines 75 and 469 promised exactly these advisories; the fold implemented rather than scoped down.

**I3 — FIXED, independently verified against primary source.** I fetched Rev. Proc. 2023-34 (irs.gov/pub/irs-drop/rp-23-34.pdf) and extracted §2.06's table from the PDF text itself (the first-pass summarizer hallucinated figures; the raw table is authoritative): completed phaseout MFJ = $25,511 / $56,004 / $62,688 / **$66,819** (0/1/2/3+ children); all-other = $49,084 / $55,768 / **$59,899** (1/2/3+); max credit 3+ = **$7,830**. The corrected comment's table (`advisories.rs:22-38`) matches to the dollar; `EIC_ADVISORY_AGI_CEILING = dec!(70000)` sits above the true maximum with headroom, and the previously non-discriminating $60k test leg was moved to $70k with an explanation. **Repro re-run on the built binary**: MFJ, 3 dependents, $63,000 wages → "EIC NOT COMPUTED" now prints (it did not at `00a594d`). KAT `eic_advisory_fires_for_mfj_at_63k_p5_i3` pins the exact household.

**I4 — FIXED.** Doc moved to `crates/btctax-cli/LIMITATIONS.md`; `main.rs:414` now `include_str!("../LIMITATIONS.md")`; `cargo package -p btctax-cli --list` **contains LIMITATIONS.md** (and tests/limitations.rs, which asserts the in-package path so a regression fails loudly). I also ran the full `cargo package -p btctax-cli --locked`: the verify build fails **for an unrelated, pre-existing reason** — all 20 errors are missing *full-return symbols* (`assemble_absolute`, `tax::advisories`, `tax::return_inputs`, `BundledFullReturnTables`, …) against the **published** btctax-core 0.5.0, i.e., the standard unpublished-dependency condition of any pre-release workspace; zero errors involve LIMITATIONS.md. It cures itself when core ships first, per the existing dependency-ordered release recipe. Not a fold defect and not the I4 mechanism.

**M1 — FIXED.** Doc now says the report prints the "1040-level summary" and that interior per-line detail "is **not** all printed today, so a hand transcription … still needs you to re-derive some intermediate lines" — matches `render_dual_report`. Capability gap filed as `p5-m1 → P6`.

**M2 — FIXED, no over-fire.** `spouse_dob_on_file = spouse.as_ref().is_some_and(|s| s.date_of_birth.is_some())`; the fire condition is still gated on `== FilingStatus::Mfj`, so MFS/QSS cannot fire it, and an MFJ spouse record *with* a DOB suppresses it. Absent-record MFJ now advises (KAT `mfj_with_no_spouse_record_still_advises_the_aged_box_p5_m2`), exactly mirroring `standard_deduction`'s forfeit behavior.

**M3 — FIXED.** Student-loan interest (L21 + §221(b)(2) phase-out), 1099-INT box-2 early-withdrawal (L18), taxable state refund (L1 §111) all added to Supported — each verified present in `return_1040.rs` (lines 420, 444-505, 751-765). Cited Schedule 1 line numbers match the 2024 form.

**N1/N2/N3 — FIXED** (non-passive foreign tax moved to UNREPRESENTABLE with the correct no-input rationale; taxable-income-before-QBI wording; Schedule A line 8a). **N4 — FIXED** (`tests/limitations.rs` drives the binary, pins stdout byte-identical + structural three-lists check). **N5 — PARTIALLY FIXED**: money format done (`fmt_usd`, verified "$1,550"/"$8,405" end-to-end, test tightened to `$1,950`); the *wrapping* half — advisory bullets as single 300-400-char lines — is unchanged in `render_advisories` (`render.rs:1104`) and not filed. Nit-severity; does not gate (see Nit-1).

## New-surface audit

- **`fmt_usd`** (`advisories.rs:101-122`): correct. Grouping loop verified for 1/3/4/7-digit values; zero → "$0"; sub-dollar → "$0.NN"; cents shown only when present. The `{:02}` fractional pad is safe: rust_decimal's `Display` uses `f.pad_integral` (verified in the vendored source, `decimal.rs:2557-2566`), which honors sign-aware zero-padding, so $X.05 renders "….05", not "….5". Negative handling is correct though unreachable (both call sites pass positive values; `refund` is `> 0`-guarded).
- **Refund threading**: `advisories_for` passes `ar.overpayment_refund` = `(total_payments − total_tax).max(0)` (`return_1040.rs:891`) — correct L34 semantics, zero when owing; owes/refund legs KAT'd.
- **FOLLOWUPS.md**: the five new P6-owned entries (`p5-c1` replace-refusal-with-real-export, `p5-i1` always-on DRAFT gate, `p5-m1` interior lines, `p6-printed-line-chain`, `p5-report-vs-pdf-rounding`) are correctly scoped — each is genuinely impossible before the P6 fillers exist; nothing that should have been fixed in P5 was deferred.

## New findings (all Nit; none gate)

- **N-r2-1 (Nit)** — r1-N5's wrapping half is neither fixed nor filed: `render.rs:1104` still emits each advisory as one unwrapped 300-400-char line. File it (ownerless residue or fold into P6's render work).
- **N-r2-2 (Nit)** — `export-irs-pdf`'s clap doc-comment (`cli.rs:138-175`), which generates the man page, doesn't mention the new full-return-year refusal. The runtime error and LIMITATIONS.md carry it, and P6 rewrites this text when the refusal is replaced — but until then the man page understates a refusal condition. Fold into `p5-c1`'s P6 entry.
- **N-r2-3 (Nit)** — two stale-text leftovers: LIMITATIONS.md:173 Legal boilerplate "Every full return is a **DRAFT** until you attest" (vacuously true today — no full-return PDF can exist — and becomes literally true with `p5-i1`; consider tying its wording to that follow-up), and a stale "< the $60k EIC ceiling" comment at `tests/tax_report.rs:1741` (assertion itself still valid at $70k).

**Process note for the caller** (not a severity finding): the fold commit is stacked on top of the parked P6 commit `51020d8`, so this gate closes at a HEAD that already contains P6 code. That code is inert at the user surface (the form8959 filler has no non-test caller), and every gate above ran over the superset — the green therefore covers it — but P6's own gate must not treat `51020d8` as already reviewed.

**The P5 gate closes: GREEN (0C/0I).**
