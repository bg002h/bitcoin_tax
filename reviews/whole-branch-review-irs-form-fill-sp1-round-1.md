# Whole-diff review (Phase E) — feat/irs-form-fill-sp1 — round 1

**Verdict: 0 Critical / 0 Important — SHIP.**

Independent Phase-E review. Diff `main (3117379)..HEAD` — 2 task commits (T1 `4a6ac92` + T2 `3ae4aaa`).
Contract: `design/SPEC_irs_form_fill_sp1.md` (R0-GREEN, 2 rounds; R0 round 1 on Fable caught the XFA hybrid +
the 1099-DA Box I/L). SP1 of task #45 — the fill engine proven on Form 8949 + Schedule D, TY2025.

## Fault-injection of the ★ safety net (map restored byte-for-byte)
- **[★ geometric read-back — a wrong OFFICIAL tax form must never be written] CONFIRMED load-bearing.** The
  verifier (verify.rs) re-derives column-x/row-y bands from the blank PDF's OWN widget `/Rect`s — the map is
  distrusted, the geometry is the oracle (R0-I3). **My fault-inject:** swapping the Row-1 (d)proceeds/(e)cost
  field assignments in the map drove the read-back RED — `Geometry("… f1_07: x-center 370.4 not in column 3
  band (273.6, 337.65) (mis-mapped column)")`, 7 KATs failed, the fill **fails closed** (no bytes written). A
  mis-mapped cell cannot escape. (+ `no_unmapped_field_filled` fires on any stray write.)

## Verified end-to-end + by inspection (my own runs + named KATs)
- **[C2 — the 1099-DA correctness] Box I/L, NOT C/F.** The 2025 map checks Box I (`c1_1[5]`/on-state `6`, ST) +
  Box L (`c2_1[5]`/`6`, LT); Box C/F stay off; core `Form8949Box::{C,F}` (forms.rs) deliberately NOT reused.
  KAT `ty2025_bitcoin_uses_box_i_and_l`. (Checking Box C = "other than digital assets" for BTC would be false.)
- **[C1 — XFA] dropped.** My `qpdf` check on the actual output: **XFA=0** on both PDFs; `output_has_no_xfa` +
  `NeedAppearances true` — so the forms render filled in Acrobat (not blank).
- **End-to-end (my run):** `btctax export-irs-pdf --tax-year 2025 --out … --attest` wrote valid `f8949.pdf` +
  `schedule_d.pdf` with the values present (`pdftotext`), the diagonal `DRAFT — ESTIMATE, NOT FOR FILING`
  watermark, and the 17-22 scope-out notice; `--tax-year 2017` → a clean "bundles 2025 only" error (SP1 scope).
- **Determinism:** `fill_is_byte_deterministic` — fill-twice byte-identical + a pinned golden sha256, via
  stripping `/Info` dates + trailer `/ID` (NFR4).
- **Overflow:** 11 rows/part; >11 → page copies with per-copy field RENAMES (the shared-name `/V` trap) + per-copy
  totals; Schedule D aggregates. `overflow_renames_fields_per_copy_no_shared_value`, `each_copy_has_its_own_totals`.
- **Schedule D:** lines 3/7/10/15/16 + QOF=No filled; 17-22 scoped out with a printed notice; the Box G/H/J/K
  separate-1099-DA warning (I5). Money = exact `Decimal` matching the CSV.
- **Estimate guard:** pseudo-active ⇒ `require_attestation` (checked first, no bytes on refusal) + the watermark;
  real ledger ⇒ clean. `pseudo_fill_requires_attestation`, `pseudo_fill_is_watermarked`, `real_fill_is_clean`.
- **Architecture:** new pure-Rust `btctax-forms` (lopdf), reuses the projection's `form_8949()`/`schedule_d()`
  (no recompute); the bundled TY2025 PDFs are US-gov public domain (no attribution). Per-year TOML maps → adding
  a year is data-only (SP3).

## Network isolation
`btctax-forms` added to `xtask check-isolation` TAX_CRATES; `cargo run -p xtask -- check-isolation` = OK — no
`ureq`/rustls in any tax crate (lopdf is pure Rust). The vault-touching binaries still cannot open a socket.

## Suite
`cargo test --workspace --locked` **1244 passed / 0 failed** (implementer; my re-run in progress, 0 failed so
far); clippy -D + fmt clean. MINOR + new crate (bundled public-domain PDFs) → next release bump.

**SHIP — SP1 proves the official-form-fill engine (fail-closed, Box I/L, XFA-dropped, deterministic). SP2
(8283/SE/1040 summary) + SP3 (2017/2024) remain.**
