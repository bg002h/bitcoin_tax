# ROADMAP STATUS — from here to a FILED federal return

**The single live tracker.** The plan itself is `design/LONG_RANGE_PLAN_filing.md` (phases, gates and
the calendar); this file is its **progress ledger** and is updated on every task that closes. Read
this first, then the plan for the reasoning behind a phase.

Last updated: **2026-09-05**.

---

## 0. The one-line answer

**TY2025 is the only blocker.** The bitcoin engine is not what stands between btctax and a filed
return — the year being filed is. TY2025 had 5 of 17 bundled form artifacts and no Schedule 1-A; it
now has 15 of 17 and Schedule 1-A computes end to end. `full_return_for(2025)` is still a tested
`None`, **by design**, and that refusal is why no wrong return has ever shipped.

E-file is closed to this product by IRS rule (MeF is gated to vetted providers with an EFIN/ETIN;
valid XML is not sufficient). **Paper is the shipped path** — which usefully makes prior-year AGI and
a Self-Select PIN correctly N/A rather than gaps.

---

## 1. The critical path

```
  P1 Schedule 1-A ──▶ P2 filing assets ──▶ P3 gate deletion + packet trial ──▶ FILED
```

Nothing else is on it. P4 (last mile), P5 (year-port machine) and P6 (P2-profile completeness) are
parallel or later.

### P1 — Schedule 1-A, end to end

| task | what it is | state |
|---|---|---|
| T1 | the per-year table, rounding as a parameter | **done** |
| T2 | the struct: 48 line labels, 52 leaves, 4 worksheets, conformance KAT | **done** |
| T3 | the input surface — 19 eligibility declarations | **done** (form section held, see below) |
| T3a | lines 4b / 5 / 14b, which have no input path | **done** — 5 and 14b refuse |
| T4 | compute, transcribed line by line | **done** — all four jump branches pinned |
| T5 | wiring: L38 → 1040 line 13b; L37 → Form 6251 line 1a | **done** |
| T6 | worked examples, five filing statuses, mutation-verify every guard | **in progress** |
| T7 | the two-oracle census, per part | **not started** |

**Held back deliberately, marked in code:** the **input-form section** for Schedule 1-A (~22 Fields
plus per-vehicle repeating-row addressing across four files). `schedule_1a` sits in the coverage KAT's
`EXEMPT_PREFIXES` with a comment saying plainly that this is a KNOWN GAP — unlike every other prefix
there, which are information-return import surfaces. The plan is explicit that **prompt wording is the
deliverable, not plumbing**: *"a wrong prompt is a wrong return that every test passes."* Nothing is
reachable through the gap while TY2025 is fail-closed.

### P2 — the filing assets

**TY2025 bundled artifacts: 15 templates / 15 maps** (TY2024 has 17/17). Built by a ten-agent fan-out,
one form each, then audited: **0 Critical / 2 Important / 5 Minor / 2 Nit**, all folded.

| still missing | why it is not a mapping problem |
|---|---|
| `f1040s1` (Schedule 1) | **no archived TY2025 authority PDF** — only the 2024 version is in the manifest |
| `f8275` | same; only `f8275r` (the R variant) is archived for 2025 |
| `f8995a` | same |

Closing those three is a **form-authority pipeline fetch**, needing the `irs-prior` URL form and a
checksum — in September 2026 the plain `irs-pdf` URL may already serve a TY2026 draft, and putting the
wrong year into committed authority data is the failure that pipeline exists to prevent.

★ `f6251`'s TY2025 map is committed and field-verified but **deliberately not wired**: TY2025 split
line 1 into 1a/1b, which needs `Form6251Map` and the fill logic to change together.

### P3 — delete the gate, drive a packet end to end

Blocked on P1 and P2. `ty2025_full_return_must_stay_fail_closed_until_complete` is untouched and must
stay so until TY2025 `FullReturnParams` land, which is **after** P2.

---

## 2. What the instruments now police

Added while working the phases, because a claim nobody executes is not a claim:

- **`map_pdf_conformance`** walks `crates/btctax-forms/forms/` on the filesystem and checks **1699
  field references across 37 committed maps** — previously two hand-listed forms. A map committed
  tomorrow is covered with nobody remembering to add it. Both checkers were watched RED on planted
  defects.
- **`Form6251Map::for_year` / `Form8995AMap::for_year`** replaced hardcoded `ty2024()` calls made with
  `year` in scope. A 2024 map on a 2025 PDF does not fail — it puts **line 11, the AMT itself, in line
  10's box**. An unmapped year now refuses.
- **Schedule 1-A's eligibility surface** fails closed by construction, and the guarantee is asserted on
  the path an import actually takes (serde), not only on `Default`.

---

## 3. The honest calendar

From the plan, unchanged by this work: **TY2025 by 2026-10-15 is not reachable.** The commitment is
**TY2025 filed with btctax after 2026-10-15**, and **TY2026 filed on time in the 2027 season**, made
cheap by P5's year-port machine.

None of the 2026-09-05 correctness work (FR-39, FR-45, the adapter cycle) was on the critical path.
It was engine and importer correctness, and it does not move TY2025.
