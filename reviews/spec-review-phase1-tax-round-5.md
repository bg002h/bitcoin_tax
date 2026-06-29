# Review — SPEC_foundation.md v0.5, Tax-Correctness (Round 5)

- **Artifact:** `design/SPEC_foundation.md` (v0.5)
- **Reviewer:** independent tax-correctness reviewer, fresh context; verified against verbatim archived primary text.
- **Date:** 2026-06-28
- **Verdict:** **0 Critical, 0 Important — tax gate HOLDS.** Both tax-interacting round-4 folds (R4-M1, R4-M2) tax-correct; gift/donation non-recognition + §1015(a)/§1223(2) dual-basis intact; all TP citations resolve. 2 new Minor + 1 Nit, non-blocking and conservative in direction.
- Persisted per STANDARD_WORKFLOW §2.

---

## Part 1 — Two tax-interacting R4 folds verified against the archive
- **TAX-R4-M1 (§3.11 cite + self-transfer exclusion) — CORRECT.** `RevProc_2024-28.txt:360-364` §3.11: "'Transfer' means the conveyance, other than a sale or disposition, … by one taxpayer **to another taxpayer**, including a completed gift, donation, contribution, or distribution." §7.4 includes Gift/Donation (enumerated) and excludes a confirmed own→own self-transfer (not "to another taxpayer"). §3.11 is a Section-3 definition, so §5.02(4)(a)'s "transfer" inherits it. Purposively sound (a 2025 self-transfer post-dates the snapshot and carries per-wallet basis, TP7). Conservative direction.
- **TAX-R4-M2 / ENG-IMPORTANT-1 (made-date time-bar) — CORRECT.** `RevProc_2024-28.txt:595-621` §5.02(4): complete specific-unit allocations "before the earlier of: (a) … first sale, disposition, or transfer … on or after January 1, 2025, or (b) … due date (including by extension) …". v0.5 renames the static field `as_of_date` (2025-01-01) and defines made-date = event `utc_timestamp`; §7.4 fires when a first-2025 event is earlier than the made-date (a) OR made-date after the unextended due date (b). Matches §5.02(4) against a date that can actually fail (the old `effective_date`==2025-01-01 never could). `RevProc_2024-28.txt:525-540` §5.02(2)(a): allocation "complete on the date that the taxpayer's books and records first record …" — the made-date is a proxy the app can't independently observe, which is exactly why the attestation is required.
- **Return-due-date prong + `timely_allocation_attested` — SOUND.** App knows only the unextended due date + imported dispositions; absent attestation it over-fires → Path A (always valid); the persisted, auditable attestation records the user's representation about books/extension facts only they know. The app makes no legal determination. Honest architecture.

## Part 2 — Gift/donation non-recognition + dual basis intact
TP10 (zero-gain `Removal`; §1015/§170(e)(1)/(f)(11)(C)/CCA 202302012/§3.11) and TP11 (§1015(a) carryover + dual-basis FMV loss-basis; §1223(2) conditional tacking — verified verbatim `26USC_s1015.html:20`, `26USC_s1223.html`) intact. R4-N1 IRS-determination note folded into TP11.

## Part 3 — All TP citations resolve
Re-verified Notice 2014-21, §1001(a)/(c), §1015(a), §1.1015-1(a)(1)-(3), §1223(2), §170(e)/(f)(11)(C), CCA 202302012, RevProc §§3.11/5.02(4)/5.02(5). Global-method wording matches §5.02(5)(a)/(b) verbatim (`RevProc_2024-28.txt:630-642`). No drift.

## CRITICAL — None.
## IMPORTANT — None. Tax at 0 Critical / 0 Important.

## MINOR (new; non-blocking, conservative)
- **R5-M1 — Global/ProRata uses the "later of" deadline; §7.4 firing logic is the "earlier of."** §7.4 prose distinguishes specific-unit ("earlier of," §5.02(4)) from global ("later of §5.02(4)(a)/(b)," §5.02(5)(b)), but the guard-(1) firing condition is written as earlier-of, while `method` admits `ProRata` (≈global). Applying earlier-of to ProRata can only **over-fire** (→ inert → Path A, valid) — no wrong-number risk; only a possibly-forfeited Path-B ProRata optimization absent attestation. Recommend the guard key off `method` (earlier-of for ActualPosition/specific-unit; later-of for ProRata/global).
- **R5-M2 — §7.4 trigger silent on a config-(b) self-transfer fee mini-disposition.** Under TP8 (b) a self-transfer network fee is a taxable mini-disposition of fee-sats — a 2025 disposition; §5.02(4)(a) bars on "first sale, **disposition**, or transfer." §7.4 enumerates Sell/Spend/Gift/Donation/§3.11-transfer but not the (b) fee mini-disposition; if omitted, direction is **under-fire** (Path B governs when arguably time-barred → could change basis if ProRata Path-B ≠ actual position). Held to Minor: config (b) is non-default + user-mandated-against (TP8 "do not change default"); fee is de-minimis; TP8 flags "limited guidance"; conservation guard pins the 2025-01-01 numbers; Path-A/attestation backstops. Recommend §7.4 state explicitly whether a config-(b) fee mini-disposition is in the trigger set.

## NIT
- **R5-N1 — Prong-(a) boundary.** §5.02(4) requires completion strictly "before" the first 2025 event; §7.4 fires only when "earlier than" the made-date, leaving the exact-equality case as valid. Measure-zero (made-date is a 2026 wall-clock creation time; 2025 dispositions carry 2025 timestamps), and the statute mixes "date" vs "date and time" granularity. No action; note for the plan's KAT boundary cases.

## Verdict
v0.5's fold of the two tax-interacting round-4 findings is faithful and tax-correct (§3.11 self-transfer exclusion + made-date time-bar both verified verbatim; §5.02(2)(a) confirms why the attestation is necessary). Gift/donation non-recognition and §1015(a)/§1223(2) dual-basis intact and archive-grounded; all TP citations resolve. The two new Minors are conservative (no wrong tax number) and the Nit is theoretical. **Tax at 0 Critical / 0 Important — gate holds for round 5.**
