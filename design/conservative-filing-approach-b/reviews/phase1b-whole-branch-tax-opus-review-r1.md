# Phase-1b whole-branch review — US-federal-tax-correctness lens (Opus r1)

Range `88a0980..6505c18`. Reviewing ONLY the Phase-1b delta (official Form 8275 fillable PDF +
`export-irs-pdf`/full-return/CSV/TUI wiring + re-pointed BG-D8 gate + M1 no-loss widening +
`promote` un-hide). Phase-1a treated as settled. Re-derived each guarantee from first principles
against current source.

## Verdict

**GREEN** — 0 Critical / 0 Important / 3 Minor / 2 Nit

No wrong filed/disclosed number, no unmet BG-D7/D8 guarantee, no permanently-refused supported year
was found in the Phase-1b delta. The three Minors are surfaced for adjudication; none gates.

---

## What I verified GREEN (the load-bearing tax guarantees)

1. **Part I amount = the AS-FILED 8949 col (e), never the pre-clamp floor.**
   `disclosure_8275` sets `amount: leg.basis` (`form8275.rs:147`) — the clamped as-filed number, not
   the floor; `printed_8275` (`printed.rs:182`) only whole-dollar-rounds it; `fill_form_8275_inner`
   writes `fmt_money(item.amount)` (`form8275.rs:122`). The KAT
   `a_clamped_leg_disclosure_adds_the_no_loss_sentence_and_files_the_clamped_amount`
   (`kat_promote.rs`) pins `amount == leg.basis` and `!= $12,000` floor. Chain is faithful.

2. **M1 `==`→`>=` widening is correct and complete.** For a *promoted* leg gain = proceeds − basis,
   so `basis >= proceeds` ⟺ gain ≤ 0. Above-floor sale: `basis = floor < proceeds`, gain > 0 → no
   suffix (pinned by new KAT `an_above_floor_promoted_sale_files_positive_gain_and_no_no_loss_suffix`,
   mutation-proven). Below-floor clamp (`basis == proceeds`) and below-floor+documented-fee
   (`basis == proceeds + doc_fee > proceeds`) → suffix. BOTH emitters are `promoted_origins`-scoped
   (`form8275.rs:122`/`135`; `conservative.rs:157`/`163`), so a normal (non-promoted) loss sale is
   never in the loop and never mislabeled. Grep confirms **no third `==`-clamp site** survives
   (`forms.rs:113`'s `basis == proceeds` is the §1015 gift NoGainNoLoss zone — unrelated).

3. **Gate is a REAL refuse-before-bytes, on every surface, same predicate.**
   `promote_export_gate` (`admin.rs:78`) is called FIRST in `export_irs_pdf` (`:385`, before the
   watermark check and `mkdir_out`), `export_full_return` (`:651`, before `fill_full_return`+mkdir),
   `export_snapshot` (`:142`), and the TUI `do_export` (`export.rs:174`, before the exclusive mkdir).
   The TUI reaches it via the crate-root re-export `btctax_cli::promote_export_gate` (`lib.rs`) — no
   `cmd::` token, KAT-E10 stays green. KAT-E12 asserts out_dir is never created on refusal;
   `export_gate_now_refuses_when_the_8275_pdf_is_absent` asserts neither the txt nor the new PDF
   partially writes.

4. **★ Year coverage — no permanent refusal of the dominant flow.** `Form8275Map::for_year`
   (`map.rs`) and `f8275_pdf` (`pdf.rs`) alias the single Rev.10-2024 asset to EVERY
   `SUPPORTED_YEARS = [2017,2024,2025]`. A promoted 2025 (current-year) or 2017 export fills
   `form_8275.pdf` and passes the gate — pinned end-to-end at the export layer
   (`a_promoted_2025_export_fills_the_8275_and_the_gate_passes`, both years) and byte-identically at
   the fill layer (`sp4.rs::form_8275_fills_for_every_supported_non_2024_year`). 2026 is not a
   supported *PDF* export year for the whole build, so it fails closed uniformly — not a supported
   flow permanently refused. The TUI's 2026 path emits only `form_8275.txt` (text, no `for_year`), so
   it works for any promoted year.

5. **Overflow pre-check refuses before bytes, whole-export, correctly independent of `--forms`.**
   `admin.rs:407-418` (crypto-slice) and `:345-356` (full-return) refuse a >6-leg promoted year
   before `mkdir_out`, naming the year, leg count, and remedy; capacity check (`> map.rows.len()`,
   =6) matches `fill_form_8275_inner`'s own Overflow threshold exactly. Refusing the *whole* export
   regardless of `--forms` is the correct conservative BG-D8 posture: a promoted 8949/Schedule-D
   position cannot be filed when its mandatory 8275 cannot be produced. KATs
   `promoted_export_with_more_than_6_legs_refuses_cleanly_not_panics` (+ crypto-slice twin) pin
   no-panic, year+count+remedy in the message, and empty out_dir — non-vacuous and site-distinct.

6. **Crypto-slice identity-less fill is tax-appropriate + consistent.** `fill_form_8275_slice` passes
   `filer: None`, skipping the name/SSN cells, exactly mirroring the Form 8283 crypto-slice fill (the
   disclosure rides beside a return btctax did not produce; the filer completes the header). The map
   always *declares* the identity cells; the slice leaves them unwritten. Consistent with the
   8949/8283 crypto-slice contract.

7. **No Phase-1a regression from the wiring.** The `events` arg threaded through
   `assemble_printed_return`/`assemble_printed_forms` (`packet.rs`) is read only by `disclosure_8275`
   (`f8275` arm); every prior chain is byte-unchanged. BG-D4 clamp, BG-D11 documented-only removal,
   and the fold-diff advisory are untouched. Census bumped 14→15, `f8275` at Attachment Sequence 92
   (< 8283's 155, so packet order stays sequence-ascending); the "iff a promoted leg is filed" both
   directions is pinned (`census.rs::full_return_packet_emits_8275_iff_a_promoted_leg_is_filed`).

8. **Non-ASCII (em-dash) encoding fails closed, not silent-wrong.** `encode_pdf_text` (`pdf.rs:~400`)
   writes non-ASCII as UTF-16BE+FEFF; `decode_pdf_text` (`pdf.rs:311`) decodes it; `verify_flat`
   read-back would RED any mojibake, and `form_8275_is_byte_deterministic` pins the SHA. `Part1Item.line`
   ("Part I — column (e)") round-trips (`sp4.rs`).

9. **BG-D10 risk copy present + correct.** `RISK_PARAGRAPH` (`form8275.rs:44`) states 20%/40% **of the
   resulting additional tax (underpayment attributable to the misstatement)**, cites Woods and
   §6664(c)(2), and never says "safe harbor". It is emitted on every promoted export via
   `write_form_8275_txt` → `render()` (co-emitted unconditionally on crypto-slice `admin.rs:437`, CSV,
   full-return, and TUI), and was shown on the BG-D6 consent screen. See Nit-1 on its (correct)
   absence from the filed PDF.

---

## Minor

### Minor-1 — an explicit `--forms` slice can omit the OFFICIAL Form 8275 PDF on a promoted crypto-slice year, and the export still succeeds (no warning).
`admin.rs:~530` (`export_irs_pdf`): the official PDF write is gated
`if wants(forms, FormArg::Form8275) { … fill_form_8275_slice … }`. `wants` (`:333`) returns true only
when `--forms` is empty (default) or explicitly lists `form8275`. So
`btctax export-irs-pdf --tax-year 2025 --forms f8949` on a promoted-basis year writes `f8949.pdf` (with
the promoted col-(e) basis) but **no** `form_8275.pdf`; the export returns Ok.
**Why not Important:** the disclosure *content* is never absent — `write_form_8275_txt` (`admin.rs:437`)
is called UNCONDITIONALLY (not under `wants`), so `form_8275.txt` (full Part I/II + risk paragraph)
always rides, and the default path emits the official PDF. So the tool always emits the disclosure;
only the §1.6662-4(f) FORM *format* is droppable, and only by deliberate opt-out — the same
honor-the-slice contract that already lets `--forms` drop a legally-required 8283.
**Why still worth surfacing:** it is an internal inconsistency — the completeness gate and the overflow
pre-check both *ignore* `--forms` (they treat the 8275 as non-optional and refuse the whole export),
yet the emitter *respects* `--forms` (treats it as optional). A filer who slices `f8949` and then files
only the sliced PDF has an inadequate §1.6662-4(f) disclosure (a plain-paper memo "has no §6662 effect",
per SPEC §4 / D-4/D-10). **Fix (one-liner):** emit the 8275 PDF whenever `printed_8275.is_some()`,
ignoring `wants(forms, Form8275)` — mirroring the unconditional `form_8275.txt`/`basis_methodology.txt`
emits and the gate's own posture; or, at minimum, warn on stderr when the official PDF is omitted while
a promoted leg files. (Full-return path is immune: `--forms` is ignored there.)

### Minor-2 — whole-dollar (official PDF) vs exact-cents (crypto-slice 8949 PDF + `form_8275.txt`) inconsistency for the disclosed basis inside one packet.
The official 8275 PDF amount is `fmt_money(round_dollar(leg.basis))` → e.g. `"12346"` (`printed.rs:188`,
`form8275.rs:122`). The crypto-slice 8949 col (e) prints the exact `Decimal` — `r.cost_basis.to_string()`
→ `"12345.67"` (`fill8949.rs:42`), and `form_8275.txt` renders `${amount:.2}` on the UNrounded
`Disclosure8275` → `"12345.67"` (`form8275.rs:176`). So a promoted `filed_basis` carrying cents shows
whole-dollars on the official 8275 PDF but cents on the sibling 8949 PDF (crypto-slice) and on the
companion `form_8275.txt`. Sub-dollar (|Δ| < $0.50), IRS whole-dollar rounding is standard, and both
derive from the same `leg.basis` — cosmetic, not a wrong position. Full-return path is internally
consistent (its 8949 is also whole-dollar via `form_8949_printed`), so only the crypto-slice PDF and the
txt disagree. **Fix (optional):** round the crypto-slice 8949 col (e) at the cell, or leave a note that
the txt is a cents-exact companion to whole-dollar official forms.

### Minor-3 — TUI export modal `compute_files` under-lists `form_8275.txt`.
`export.rs:126` `compute_files` pushes `basis_methodology.txt` but not `form_8275.txt`, though
`do_export` writes `form_8275.txt` for a promoted year (`export.rs:206`, pinned by KAT-E13). The
confirmation modal's file preview therefore omits an artifact the export will actually write. UX
inconsistency (parallel to how `basis_methodology.txt` IS listed); no tax number affected. **Fix:**
push `"form_8275.txt"` in `compute_files` when `disclosure_8275(&snap.events,&snap.state,year).is_some()`.

---

## Nit

### Nit-1 — BG-D10 risk paragraph is (correctly) absent from the FILED PDF; carried only in `form_8275.txt` + consent.
`fill_form_8275_inner` writes Part I items + Part II narrative + identity only — never `RISK_PARAGRAPH`.
This is right: the penalty-exposure copy is filer-education, not content one writes onto the form filed
to the IRS; it is delivered via the always-co-emitted `form_8275.txt` (`render()`) and the BG-D6 consent
screen. Suggest a one-line doc comment on `fill_form_8275_inner` stating the risk copy is intentionally
txt/consent-only, so a future reader does not "fix" it onto the filed form.

### Nit-2 (out-of-scope observation, settled Phase-1a) — the `incomplete` flag is computed on the CONCATENATED Part II.
`disclosure_8275` sets `incomplete = part_ii.trim().is_empty()` on the joined narrative
(`form8275.rs:154-159`). In a raw-vault, multi-promote, same-year, mixed-completeness corner (tranche A
complete, tranche B empty narrative), the join is non-empty → `incomplete=false` → the gate passes while
B's position has no filer facts in the single Part II field. This is Phase-1a code (unchanged by the
delta; the T10 verb refuses empty narratives at record time, so it is raw-vault-only) and thus outside
this review's scope — noted for awareness only, does not affect the verdict.

---

## KAT integrity (spot-check)

The new Phase-1b KATs pin the delta's tax guarantees non-vacuously and at the right seam: year-coverage
(2025 & 2017 e2e at the export layer + byte-identical at the fill layer), gate-refusal
(both-artifacts, out_dir untouched), overflow (no-panic, year/count/remedy, empty out_dir, full-return
AND crypto-slice sites distinguished), M1 suffix over-fire guard (mutation-proven), TUI refuse
(E12, self-verifying: only passes when the promote is live AND incomplete) and TUI emit (E13),
per-field sentinel readback (catches a map swap `verify_flat` can't), and unsupported-year rejection.
No vacuous assertions found (e.g. the success KAT keys on `form_8275.txt` alone, never a
`txt || basis_methodology` disjunction).

**Note:** I did not independently run the full validation suite (the green-gate is a separate check);
this lens is tax-correctness of the delta. The KATs are non-vacuous by inspection and the continuity
record reports the branch driven to gate-green.
