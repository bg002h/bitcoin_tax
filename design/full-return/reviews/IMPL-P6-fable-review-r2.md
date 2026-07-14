# P6 GATE REVIEW — full-return fold verification (`4d3fdea`) — Fable r2

Persisted verbatim before folding, per STANDARD_WORKFLOW §2.

**VERDICT: 0 Critical / 3 Important / 14 Minor / 5 Nit** — the gate remains **OPEN**.
All three new Importants are RESIDUALS of the r1 fold (I2 / I5 / I7), i.e. the fixes were
incomplete in exactly the direction r1's prescriptions warned about. NEW-I2 is a REGRESSION
the fold introduced.

---

**Reviewer:** Fable (independent; same reviewer as r1, verifying my own findings' folds adversarially).
**Scope:** the fold commit `4d3fdea`, everything since the persisted r1 (`5abc4b6..HEAD`), the whole phase (`b40bdec..HEAD`), and the §3.1 SPEC amendments that rode with the fold.

**Gates re-verified by me, not taken on faith:** `cargo test --workspace --locked` = **1683 passed / 0 failed** (exit 0; +7 = 8 new tests − 1 deleted, consistent with the commit's claim), `cargo clippy --workspace --all-targets --locked -- -D warnings` = 0, `cargo fmt --all -- --check` clean, `cargo run -p xtask -- docs` regenerates every man page with a **clean tree** (no drift), FROZEN files (`tax/{types,compute,se}.rs`) = **0 bytes** changed since `059ec2a`.

**Summary judgment:** the fold is genuine work — seven of the nine Importants are fixed outright, with real, discriminating KATs. But the two hardest ones are **fixed incompletely in exactly the direction r1's fix prescriptions warned about**: I2's Tax-Table step-function defect survives on the QDCGT path (the worksheet's third operand is still exact-derived, not the printed Schedule D figure), and I5's rewiring **dropped the negative-AGI clamp** the old chain inherited from `ScheduleAParts.agi`, so a negative-AGI itemizer now files a Schedule A that deducts more medical than was paid. Separately, the I7 disjointness KAT is **vacuous on the property it claims to pin** — it passes with the fix reverted — and LIMITATIONS again claims a guarantee the test does not deliver, which is the same false-safety-claim class I7 was about. The gate stays open.

---

## Per-finding verification

### I1 — 1040 line 1a — **FIXED**
- Map: `line1a = "topmostSubform[0].Page1[0].f1_32[0]"` (`f1040.map.toml:48`). The cell is right — f1_32 is the 1a AMOUNT cell per my own r1 dump, and the map's correlation comment (f1_32=1a … f1_39=1h, f1_40=1i mid-column combat-pay, f1_41=1z) matches the 2024 form's field census exactly (1a–1h = 8 cells, 1i, 1z).
- Filler: 1a inserted at ordinal 0 of `GRP_P1_AMOUNT` (`form1040_full.rs:108–124`), above 1z — geometric descent holds (f1_32 sits physically above f1_41; the read-back verifier would have failed closed otherwise, and the suite is green). `Form1040Lines` carries both fields; `form_1040_income_lines` writes the same rounded figure to both (`line1z = line1a`).
- KATs: the read-back KAT (`the_1040_prints_line_1a_under_the_1z_that_adds_it_up`) asserts f1_32 == "120000" on serialized bytes — discriminating against a map or filler revert.

### I2 — 1040 L16 from the printed L15 — **FIXED-BUT** → new Important (NEW-I1)
What was asked, was delivered for the operand it named: `printed.rs:600` now computes `line16 = qdcgt_line16(schedule, bp, line15, line3a, round_dollar(ar.net_ltcg))` — printed L15 ✓, printed 3a ✓ — and the discriminating KAT (`form_1040_line16_is_the_tax_on_the_printed_line15_not_on_the_exact_taxable_income`) is real: it pins `line16 == Table(printed 47,150)` **and** `line16 != round(tax(exact 47,149.80))`, so a revert fails. The §1(h) clamp interacts correctly (worksheet's F-A cap `pref = pref_full.min(ti)` now binds on the printed TI, and `bottom = (ti − pref_full).max(0)` floors — a preferential slice exceeding the printed TI cannot make L16 worse; the "new L16 higher than the old" cases are the *correct* consistency with the filed L15).

But the QDCGT worksheet has **three** inputs, and the third is still exact-derived — see **NEW-I1** below. The scrutiny question in my brief ("should it be a Schedule-D-derived PRINTED figure?") answers **yes**, and the fold did not do it. The KAT covers only the ordinary path (3a = 0, net_ltcg = 0), which is how the gap survives the suite.

### I3 — 8949 identity on both pages — **FIXED**
- Map: `[identity_page1]`/`[identity_page2]` blocks with the f1_1/f1_2/f2_1/f2_2 FQNs I dumped in r1 (`f8949.map.toml:55–63`); `Form8949Map` carries them as `Option<IdentityCells>` with serde defaults, so the 2017/2025 slice maps still parse.
- Filler: `fill_8949_parts_inner` writes name + SSN on both pages when a header is passed, **fails closed on a missing identity block** (`FormsError::Geometry`), renders the SSN against the actual `/MaxLen` from the blank fields, and pushes `Geo::Check` placements — which per `verify.rs:31/173` are geometry-exempt but **in the no-unmapped set**, so the oracle covers them. The slice path (`fill_8949_parts`) routes through the same inner with `None` — no writes, no placements, behavior unchanged; the slice byte-golden (kats.rs) passed in the green suite.
- KAT (`the_full_return_8949_is_named_on_both_pages`) reads back all four cells from serialized bytes, exercising both parts so both pages are in play — discriminating, and it explicitly pins page 2 ("the page that was missed").

### I4 — Schedule D QOF answered No on the full path — **FIXED**
`schedule_d_full.rs:280–293` checks the No box using the same map cell (`c1_1[1]`, on="2") and the same rationale as the slice (`schedule_d.rs:111`). KAT (`the_full_return_schedule_d_answers_the_qof_question`) asserts `box_on` on the serialized output — discriminating. One residue: the write is `if let Some(qof_no)` — a *silent skip* on a map without the cell, where the rest of the full filler uses fail-closed `need()`. Unreachable for TY2024 (the only supported full-return year, and its map has the cell) — recorded as a new Minor (NM1), not a blocker.

### I5 — Schedule A L2 ← printed 1040 L11 — **FIXED-BUT** → new Important (NEW-I2)
The composition is right and the cycle question is settled cleanly: `form_1040_income_lines` (L1a–L11) was extracted, is derived **before** `schedule_a_lines(ar, income.line11)` (`core/tax/packet.rs:461–464`), and L11 depends only on Schedules B/1/D, never on Schedule A — no cycle, verified by reading the income-block body. The SPEC closed list now carries the citation (`SPEC_full_return.md` §3.1, "[ADDED 2026-07-13…] Sch A L2 ← the 1040's printed L11"). The unit KAT (`schedule_a_line2_is_the_printed_1040_line11`) feeds a deliberately divergent L11 and pins L2 and the 7.5% floor on it — discriminating for the function.

But the negative-AGI case I raised in the finding is now **worse**, not fixed — the fold dropped the clamp. See **NEW-I2**. Also: nothing pins the *assembly wiring* (no packet-level `sch_a.line2 == f1040.line11` tie-out; a revert of `packet.rs:464` to `round_dollar(ar.agi)` fails no test) — new Minor (NM3).

### I6 — the non-crypto-noncash refusal keys on the aggregate — **FIXED**
- The guard moved to `screen_compute_dependent` (`return_1040.rs:546–575`) and refuses on `user_noncash > 0 && user_noncash + crypto_noncash > $500`, with `crypto_noncash = year_donation_deduction(state, year)` = Σ of the year's ledger `claimed_deduction` — the same §170(e)-reduced figures that feed Schedule A L12, so the aggregate is the conservative superset of the printed trigger. Both directions verified: **crypto-only never refuses** (`user_noncash == 0` short-circuits, pinned by the KAT's second assertion); **the mixed case refuses** ($300 user + $400 crypto, pinned); user-only > $500 still refuses (2000 + 0 > 500 — logic verified, though its *test pin* was deleted with the old screen, NM5); ≤ $500 aggregate passes and no 8283 attaches (presence keys on printed L12 > $500, unchanged); cash classes are excluded from the sum.
- **Called on every 8283-producing path:** the export runs `screen_inputs` → `screen_compute_dependent` → `screen_absolute` before assembling (`admin.rs:453–462`, all before any byte is written), and the report reaches `assemble_printed_forms` only through `resolve_and_screen`, which runs the compute-dependent screen for every `ReturnInputs` year (`resolve.rs:189–204`, fail-closed if the invariant breaks). The only other `resolve_profile` caller is a test helper. The slice's 8283 (crypto-only, no `ReturnInputs`) has no user gifts by construction.
- KAT is discriminating (revert → `screened` returns `None` → fail).

### I7 — filename disjointness — **FIXED-BUT** → new Important (NEW-I3)
The **code** fix is real and total: every packet member is written as `{seq}_{name}.pdf` with `unwrap_or("00")` (`admin.rs:501–505`) — `00_f1040` through `155_f8283` — while the slice writes bare stems (`f8949.pdf`, `schedule_d.pdf`, `schedule_se.pdf`, `form_8283.pdf`, `form_1040_capgains.pdf`; enumerated from `admin.rs:256/264/308/331/360`). No slice name starts with a digit prefix; `manifest.txt` is full-path-only. The two name-spaces are disjoint — I verified by enumeration, not the KAT.

The **KAT is vacuous on the property it pins** — see NEW-I3.

### I8 — CLI output on the full path — **FIXED**
`main.rs:631–661`: the full branch lists every packet path plus the manifest ("← your stapling order"), and the false Schedule-D-17-22 note moved to the `else` (slice-only) branch. The 8283 escalations are carried: `export_full_return` now populates `form_8283_needs_review` and `form_8283_section_b` from `printed.forms.f8283` (`admin.rs:541–551`), and the full branch prints both loud notices; no duplication with the slice-path notices, which key on `form_8283_path.is_some()` (None on the full path). Residue, non-blocking: the preceding "Filled IRS forms →" header prints an **empty list** on the full path before the packet block, and the whole main.rs block is display-layer with no test — NM6.

### I9 — the report prints printed figures — **FIXED**
`render.rs:1187–1234` verified line-by-line: every form-line-labelled figure in the absolute block now renders from `PrintedForms` — L9/L10/L11/L12 (`f.line*`), Sch 3 L1, Sch 2 L4, 8959 L18, 8960 L17, plus the previously-converted L13/15/16/24/33/34/37. The delta block stays cents (untouched). Conditions still branch on `ar.*` nonzero-ness, which is fine — only the displayed figure was the finding. LIMITATIONS' claim is now true.

---

## NEW findings

### Critical — none

### Important

**NEW-I1 — The QDCGT worksheet's line-3 operand is still the exact-cents figure; I2's step-function defect survives on every Schedule-D + preferential-income return.** *(I2 residual.)*
`printed.rs:600–606` passes `round_dollar(ar.net_ltcg)` as `qdcgt_line16`'s net-capital-gain operand. The worksheet's own line 3 — and `qdcgt_line16`'s own doc (`method.rs:66–67`: "`net_ltcg` = … `min(Sch D 15,16)`, ≥0; L3") — takes **the smaller of the printed Schedule D lines 15 and 16, not less than 0**: figures a human copies off the filed schedule. Those printed lines are sums of printed operands (`schedule_d_lines`: L15 = printed 10h + 13 − 14, L16 = printed 7 + 15) and legitimately drift ≥ $1 from `round_dollar(exact preferential_gain)` — the phase's own 302≠301 KAT exists precisely because `Σround ≠ roundΣ` on the 8949 totals feeding them. A $1 difference in worksheet L3 shifts the ordinary remainder (`bottom = L15 − pref`) by $1, and `worksheet_tax(bottom)` is the **same $50-tread step function I2 gated on**: across a bin edge, the filed L16 differs from the worksheet a human (or the Service) computes from the filed forms by a whole bin step (~$11–12 in the Table region), not the $1 residual §3.1 tolerates. Concrete: printed Sch D L15/L16 = 5,003 (row-rounding) vs `round_dollar(net_ltcg)` = 5,000; printed 1040 L15 = 60,000 → code's bottom = 55,000 (bin [55,000–55,050)), human's bottom = 54,997 (bin [54,950–55,000)) → L22 differs by round(50 × 22%) = $11 → filed L16 contradicts the packet's own Schedule D through the worksheet — the same math-error-notice channel as I2.
The I2 KAT cannot see this: its fixture has no preferential income. The §3.1 amendment as written also under-specifies the rule — it names the printed **L15** but not the worksheet's other operands.
Fix: pass `min(sch_d.line15, sch_d.line16).max(0)` (the printed figures — available at the assembly site; thread it through `Form1040Income` or a parameter, since `form_1040_lines` no longer receives `sch_d`); when Schedule D doesn't file, all its printed lines are 0, so the operand degenerates correctly. Extend the §3.1 amendment to say the worksheet is applied to **printed operands throughout** (L1 = printed 1040 L15, L2 = printed 3a, L3 = min of printed Sch D L15/L16 ≥ 0). Discriminating KAT: a fixture whose printed Sch D min(L15,L16) ≠ round(exact net_ltcg) with `bottom` straddling a bin edge, asserting L16 equals the worksheet on printed operands and differs from the current derivation.

**NEW-I2 — The I5 rewiring dropped the negative-AGI clamp: a negative-AGI itemizer now files a Schedule A that deducts more medical than was paid.** *(I5 residual; regression of the "composition change breaking a case the old code got right" class.)*
Before the fold, `schedule_a_lines` took `round_dollar(p.agi)` where `ScheduleAParts.agi` is **clamped at 0** (`return_1040.rs:276–278` — "clamp it so the floor never helps the taxpayer (review M1)"; the test fixture `parts()` clamps identically at `printed.rs:1889`). The fold switched the operand to the **unclamped** `income.line11` (`packet.rs:464`; `line11 = line9 − line10`, no clamp) and added no clamp downstream: `printed.rs:1081–1083` computes `line3 = round_dollar(RATE × line2)` and `line4 = (line1 − line3).max(0)`. On a negative-AGI itemizer (reachable: e.g. small wages + a §1211-capped loss + an early-withdrawal penalty exceeding the interest, with SALT+medical itemizing over the standard), line2 = −50,000 → line3 = **−3,750** → line4 = **13,750 on $10,000 of medical expenses paid**. `push_money` writes negatives with a leading minus, and no filler guard refuses — the packet **files**. The printed L17 → 1040 L12 then exceeds the engine's own computed itemized deduction (whose exact chain still clamps) by an **unbounded** amount — not a $1-class residual. §213(a) allows "the expenses paid … **to the extent** that such expenses exceed 7.5 percent of AGI" — the deduction cannot exceed the expenses paid; the exact engine agrees; only the filed form now disagrees. The bottom-line tax happens to be unchanged (line2 < 0 ⟹ printed L11 < 0 ⟹ L15 = 0 either way), which is why this is Important rather than Critical — but a filed Schedule A claiming a deduction larger than the underlying expense is a misstatement on the return, and r1's fix prescription said in terms to "keep the §213(a) floor's `max(0,·)` on line 3/4".
The test suite cannot see it: the updated negative-AGI test (`printed.rs:2077–2099`) passes `round_dollar(parts.agi)` — the **fixture's clamped 0**, not the production operand — so it now pins the fixture's clamp, not the code path production actually takes.
Fix: `line3 = round_dollar(MEDICAL_FLOOR_RATE * line2).max(Usd::ZERO)` (L2 keeps the true printed L11, satisfying the citation; the floor never goes negative, L4 ≤ L1). Flagged uncertainty, explicitly: I have not re-verified the 2024 Schedule A instructions' exact wording for a negative line 2 (a human literally following "multiply line 2 by 7.5%" would also write −3,750); the §213(a) substantive cap and the divergence from the engine's own deduction are not uncertain, and the conservative direction (never inflate a claimed deduction) is this project's mandated posture. Re-point the negative-AGI KAT at the production operand (a negative `income.line11`).

**NEW-I3 — The I7 disjointness KAT is vacuous on the overwrite property it claims to pin, and LIMITATIONS again claims a guarantee that isn't delivered.** *(I7 residual — the KAT was the explicit condition.)*
`the_two_pipelines_write_disjoint_filename_sets` (export_irs_pdf.rs) runs both pipelines into one directory and asserts: (1) the slice added files, (2) no packet file was *removed*, (3) `for name in slice_only { assert!(!before.contains(name)) }`. Assertion (3) is a **tautology** — `slice_only = after − before` cannot contain an element of `before` by definition of set difference — and assertions (1)/(2) cannot see an overwrite either, because `write_bytes_owner_only` → `open_owner_only` uses `create(true).truncate(true)` (`fsperms.rs:24–29`): a colliding write silently truncates and replaces, leaving the *name set* identical. I walked the reverted code through the KAT: un-prefix the packet names and the slice's `f8949.pdf`/`schedule_d.pdf`/`schedule_se.pdf` clobber the packet's — and **every assertion still passes**. The commit message ("A KAT now runs both pipelines into one directory and asserts nothing is overwritten") and LIMITATIONS.md:69–72 ("A KAT asserts the disjointness — it is a guarantee, not a claim") are both false as shipped — the exact false-safety-claim defect class r1-I7 was raised about, on the same paragraph of the same doc. The code fix itself is correct (verified by my own enumeration above), but the load-bearing mitigation for deleting the P5-C1 refusal is unpinned against regression.
Fix (small): either assert what r1 asked — compute the two pipelines' filename **sets** (e.g. run them into two directories, or capture the packet's `full_return_paths` and the slice's report paths) and assert an empty intersection — or hash the `before` files and assert their bytes are unchanged after the slice run. Either version fails on the reverted code; keep the current run-both-into-one-directory shape if you add the byte check.

### Minor (new)

- **NM1** — the full Schedule D's QOF write is `if let Some(qof_no)` (silent blank on a map without the cell) where every other mandatory full-path cell uses fail-closed `need()`. Unreachable for TY2024; align the convention. (`schedule_d_full.rs:280`)
- **NM2** — "the prefix IS the stapling order" (`admin.rs:497` comment) is false under the one ordering a filer actually sees: lexicographic directory sort puts `12A_f8949.pdf` **before** `12_schedule_d.pdf` (`A` < `_`) and `155_f8283.pdf` between `12A` and `17_schedule_se.pdf`. The manifest (emission order, ascending sequence) is correct and labeled — fix the comment or zero-pad/scheme the prefixes; also the manifest shows `—` for the 1040 while its filename says `00`.
- **NM3** — no packet-level tie-out pins `sch_a.line2 == f1040.line11`; the I5 unit KAT cannot see an assembly-wiring revert at `packet.rs:464`.
- **NM4** — the r1 Minors/Nits (M1–M8, N1–N5) are neither burned down nor **filed**: `design/full-return/FOLLOWUPS.md` has no P6-r1 entries. The standing rule requires Minors be recorded with an owning phase before the phase closes; M7 (burndown hygiene) is now compounded rather than addressed.
- **NM5** — the deleted `return_refuse.rs` test was the only pin on the **user-only > $500** refusal; the new KAT covers mixed and crypto-only but not that original case. The behavior is correct (verified by reading), but the coverage regressed.
- **NM6** — on the full path the CLI prints an empty "Filled IRS forms for tax year N →" header before the packet block, and the I8 output block (main.rs) has no test coverage at all (display-layer; the integration tests drive `cmd::admin` directly).

---

## The r1 Minors, reassessed in light of the fold

None is elevated to Important:

- **M2 (MFS-no-spouse)** — untouched by the fold; the 1040 filler change was line-1a-only. Stays Minor: the capture gap is an incomplete header on an unusual capture, not a wrong figure, and the refusal architecture (screens) is where the fix belongs.
- **M5 (`must_file` skipping the 8949 branch)** — untouched (`forms/packet.rs` changed only the push call). Re-examined against the fold: the printed-L15 L16 change doesn't interact (an all-cells-round-to-zero Schedule D yields L16 = Table-region arithmetic on figures that are 0 either way). Stays Minor: a sub-dollar edge (every printed cell of a real disposal rounding to $0) — but note it now also intersects NEW-I1's operand question, so fix them together.
- **M1, M3, M4, M6, M8** — all untouched, all still Minor for the r1 reasons. M6 (instruction-level citations) is the milder cousin of NEW-I1 — the distinction (on-face SOURCE citation vs "see instructions") still holds, but the NEW-I1 fix note should say why the worksheet operands are source-grade (the worksheet's own line text names the Schedule D lines).
- **M7** — still open and now larger (see NM4).

Nits N1–N5 carry unchanged (N3 is partially answered: the 8949 identity KAT exists, but `every_schedule_carries_the_name_and_ssn_header` still tests one form and the 1040↔Sch 2 cell-text leg is still absent).

---

## KAT quality — would each fold-KAT fail if its fix were reverted?

| KAT | Verdict |
|---|---|
| `form_1040_line16_is_the_tax_on_the_printed_line15…` | **Discriminating** — `assert_eq` to Table(printed) *and* `assert_ne` to round(exact); revert fails both. Covers the ordinary path only (NEW-I1 gap). |
| `form_1040_line1a_carries_the_w2_wages…` (unit) | Discriminating for the income block; blind to a filler/map revert on its own. |
| `the_1040_prints_line_1a_under_the_1z…` (read-back) | **Discriminating** — serialized-bytes read of f1_32; a map-key removal fails `need()`, a filler revert fails the assert. |
| `schedule_a_line2_is_the_printed_1040_line11` | Discriminating for the function; blind to the assembly wiring (NM3), and the negative-AGI companion test pins the fixture's clamp, not production (NEW-I2). |
| `the_full_return_8949_is_named_on_both_pages` | **Discriminating**, both pages, serialized read-back. |
| `the_full_return_schedule_d_answers_the_qof_question` | **Discriminating** (`box_on` on output bytes). |
| `mixed_noncash_gifts_over_the_aggregate_8283_threshold_refuse` | **Discriminating** for mixed + crypto-only; the user-only pin was lost (NM5). |
| `the_two_pipelines_write_disjoint_filename_sets` | **VACUOUS** on overwrite — passes with the fix reverted (NEW-I3). |
| I8 / I9 | No KATs (display layer). I9 verified by code inspection; I8 noted in NM6. |

## Regression sweep (shipped code the fold touched)

- `schedule_a_lines` / `form_1040_lines` signature changes: all callers updated (one production site each, in `core/tax/packet.rs`; the rest are tests). The extracted `Form1040Income` is a verbatim move of the income block (diff-verified) — except for the L2-source change, which is NEW-I2.
- `assemble_printed_return`/`assemble_printed_forms` grew `table` — both CLI callers updated (`admin.rs`, `tax.rs`); ordering income → Sch A → 1040 is correct and cycle-free.
- The 8949 filler's identity path: slice behavior byte-identical (None), full path fail-closed on missing map blocks, read-back verify runs on serialized bytes, `Geo::Check` in the no-unmapped set. No regression.
- The refuse-guard move: input-screen consumers lost nothing (the only 8283-producing paths both run the compute-dependent screen; `resolve_profile`'s sole extra caller is a test helper). Behavior strictly widens the refusal to the mixed case; user-only and cash cases unchanged.
- The exact-cents chain (`ar.regular_tax`, FROZEN files): untouched, 0 bytes.

---

## What must happen to close the gate

Fix NEW-I1 (printed Sch D operand + §3.1 operand-completeness amendment + a QDCGT-path discriminating KAT), NEW-I2 (clamp the printed medical floor at ≥ 0 + re-point the negative-AGI KAT at the production operand), and NEW-I3 (a KAT that actually fails on the collision + re-true the LIMITATIONS sentence). Each is small and local. File the r1 Minors/Nits and the new NM1–NM6 in FOLLOWUPS with owning phases (NM4 is that requirement). I am available for r3 after the fold.

**VERDICT: 0 Critical / 3 Important / 14 Minor / 5 Nit** — the gate remains **OPEN**. (Composition: 3 new Importants, all residuals of r1's I2/I5/I7; 6 new Minors + 8 carried unfiled; 5 carried Nits.)
