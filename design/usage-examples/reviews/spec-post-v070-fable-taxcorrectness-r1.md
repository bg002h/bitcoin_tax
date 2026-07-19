# Fable adversarial TAX-CORRECTNESS review — SPEC_post_v070_product_cycle.md (r1)

**Reviewer:** Fable (adversarial tax-correctness red-team; independent of the author and of the
general design/completeness reviewer).
**Artifact:** `design/usage-examples/SPEC_post_v070_product_cycle.md` @ `feat/post-v070-product-cycle`.
**Lens (narrow, by charter):** (A) math-path contamination vs the §1 invariant; (B) answered-ness
false-negatives on the pseudo disclosure surfaces; (C) valid-return-blocking refusals. General
design/completeness findings are deliberately out of scope here.
**Method:** every §9 anchor was re-read against current source on this branch (state.rs, fold.rs,
resolve.rs (core + cli), tax.rs, render.rs, main.rs, cli.rs, reconcile.rs, eventref.rs, whatif.rs,
persistence.rs, tabs/tax.rs, xtask/examples.rs, tui lib.rs, tui-edit form.rs). Authority hierarchy
observed: only statute/reg cited as law; forms/instructions cited as forms, not law.

---

## Findings table

| ID | Sev | Item | One-line |
|----|-----|------|----------|
| C1 | **Critical** | UX-P4-1 / §3.1 + §3.5 | `report --tax-year --write-carryover` persists pseudo-contaminated carryover figures into year+1's stored ReturnInputs with **no pseudo gate and no taint** — a laundering channel the banner never covers; next year's clean-looking number rides fiction with `pseudo_active() == false`. |
| C2 | **Critical** | UX-P4-1 / §3.1 | The §3.1 claim "the one silent surface" is **false**: the viewing TUI's Tax tab renders the same pseudo-contributed "TOTAL federal tax attributable" with zero pseudo disclosure, and the specced fix (CLI `render_tax_outcome` only) leaves that false-negative disclosure live after the gated item closes. |
| I1 | **Important** | UX-P4-3 / §3.2 | The blanket "exact-duplicate re-decide → refuse" contradicts the engine's **deliberate last-wins design for `ManualFmv`** — refusing a second `set-fmv` blocks the sanctioned FMV-correction flow that a correct income figure legitimately needs. |
| I2 | **Important** | UX-P4-3+P4-4 / §3.2–3.3 | The specced "project-then-validate" precedent validates against the **pseudo-tainted projection**; under `pseudo on`, synthetic defaults make targets look already-decided/retyped, so the refusals would block the **real correcting decisions** — the exact remedy the UX-P4-1 banner instructs. Predicates must be pinned to a pseudo-OFF projection / the persisted (void-folded) decision log. |
| I3 | **Important** | UX-P4-4 / §3.3 | Restricting `--appraiser-tin` to SSN-shape refuses a legitimate **EIN-shaped appraiser TIN** — contradicting 26 CFR 301.6109-1(a)(1)(i) (a TIN is an SSN/ITIN/ATIN/**EIN**) and the field's own contract (`cli.rs:653` "Appraiser TIN/SSN/EIN"). Blocks a valid Form 8283 Section B. |
| I4 | **Important** | UX-P1-3 / §3.3 | The warn threshold "FMV > 100× the lot's **cost-basis-implied value**" is degenerate at basis $0 (the app's own conservative default → every such donation warns) and contradicts the spec's own "high-appreciation gifts are real" rationale; it also silently diverges from the `sats/1e8 × recent-close` formula named one sentence earlier. |
| I5 | **Important** | UX-P4-1 / §3.1 | The `[PSEUDO]` suffix hook (`render_tax_outcome`) misses the dual-report absolute-return totals ("TOTAL TAX (L24)", "Absolute TOTAL TAX …") rendered by `render_dual_report` — the most filing-authoritative scraped lines in the same stdout go unsuffixed, defeating the spec's own scraped-line rationale. |
| M1 | Minor | UX-P4-4 / §3.3 | Pin the negative-basis guard **per-flag** (precedent `main.rs:126-139`), never inside the shared `parse_usd_arg` (~25 call sites incl. tax-profile MAGI, where a negative value is not tax-nonsense). The refusal itself is tax-correct: no legitimate negative cost basis exists (§1012; §1016 adjustments floor at zero; §301(c)(2)–(3) excess-of-basis is gain, never negative basis). Zero must stay allowed (it is the app's own conservative default). |
| M2 | Minor | UX-P4-4 / §3.3 | Acquired-strictly-after-receipt refusal is sound at instant granularity but compares **calendar dates from different sources**: a truthful acquisition an instant before receipt can carry a statement date one day after the receipt tax-date (tz skew). Refusal message must print the receipt date (and its tz basis) so the user can enter the consistent date; consider allow-plus-one-day-with-advisory. Same-day equality correctly allowed. |
| M3 | Minor | UX-P4-4 / §3.3 | Donee-EIN shape check: **normalize** hyphenless 9-digit input (`123456789` is a valid EIN, just unformatted) instead of refusing; refusal message should say "omit `--donee-ein` if the donee has none" (field is optional — `cli.rs:646` — which already covers treaty-charity donees without EINs). Refusing SSN-shaped donee ids is tax-correct (§170(c): an individual is not a qualified donee). Also pin the choke point at `set_donation_details` so the TUI-edit form path (`form.rs:1364-1420`) is covered, not just CLI arg parsing. |
| M4 | Minor | M-1 / §4 | `preserve_order` is a **workspace-global feature flip**; the spec's acceptance (indexmap MSRV + net-isolation) omits a blast-radius audit. Verified safe today (fingerprints are hand-rolled bytes, `persistence.rs:25-55`; typed-struct serde is order-independent; the only `serde_json::Value` sites are the `income show` display tree, input-form coverage tooling, and update-prices API parsing) — pin that enumeration into the KAT so a future Value-map iteration feeding persistence or a byte-compared artifact can't sneak in. |
| M5 | Minor | UX-P4-10 / §3.5 | §8's open question answered: **no in-repo contract assumes exit 0** (no Makefile/CI use; xtask journeys display `[exit N]` only when non-zero and no current journey renders a NOT COMPUTABLE report). The 0/1/2 map is coherent but note the asymmetry: resolver-level `Uncomputable` (profile/inputs refusal) exits **2** while engine-level `NotComputable` exits **1** — both mean "no filing-ready number"; document "key on non-zero". Also document two deliberate non-triggers: dual-report absolute-refusal-with-delta-computed stays exit 0, and pseudo-active stays exit 0 (the banner is the signal; a non-zero would break the estimate workflow). |
| N1 | Nit | UX-P4-1 KAT | The §3.1 KAT compares two *different* vaults; add the operational §1-invariant guard: the existing tax-report golden's byte-diff across the fix shows **only** banner/suffix line insertions, all dollar figures byte-identical. |
| N2 | Nit | UX-P4-1 | Suffix parser-safety verified: ` [PSEUDO]` is space-separated after `fmt_money` output — a last-field scraper reads `[PSEUDO]` (fails loud, good); no corrupted-number risk. Keep the leading space. |
| N3 | Nit | UX-P4-9 | Verified both values in scope at the single raise site (`whatif.rs:234-236`); carrying fields on `WhatIfError::NoLots` also touches the harvest mappings (`whatif.rs:530` and `:534` — note `InvalidTarget` maps to `HarvestStatus::NoLots` too). Display-only; no tax contact. Optionally name pending-transfer sats in the message (they are correctly excluded from "available" but a user comparing against holdings will wonder). |
| N4 | Nit | UX-P4-1 banner text | "Run 'btctax verify' for the [PSEUDO] rows" is a wrong pointer: `verify` shows the advisory; the flagged **rows** render in bare `report` (`render.rs:240/318/355/367`). Point at both. |

---

## Critical findings — detail

### C1 (Critical, class B): `--write-carryover` launders a pseudo-contaminated figure into a persisted, unflagged tax input

The exact false-negative this review was chartered to hunt: a synthetic lot feeding a tax-year's
number **with no banner able to fire**.

Trace (all verified on current source):

- `report --tax-year <Y> --write-carryover` runs `write_back_carryover`
  (`crates/btctax-cli/src/main.rs:179-181` → `crates/btctax-cli/src/cmd/tax.rs:444-517`).
- It projects the ledger (`tax.rs:455`) — under `pseudo on` that state contains synthetic $0-basis
  lots, synthetic FMVs, and pseudo-tainted donation legs — then computes the **absolute return**
  from that state: `assemble_absolute(&ri, &state, …)` (`tax.rs:486`).
- **No pseudo gate exists anywhere in the chain.** `resolve_and_screen` uses its
  `pseudo_reconcile` argument *only* to enable the placeholder-profile fallback
  (`crates/btctax-cli/src/resolve.rs:120-128`); `screen_absolute`/`screen_compute_dependent`
  check QBI/AMT/TI≤0, not pseudo. `PseudoReconcileActive` is an **Advisory**, not Hard
  (`state.rs:100-102`), so `compute_tax_year`'s hard-blocker gate does not fire — pseudo mode's
  entire purpose is to clear the would-be blockers.
- `apply_carryover_writeback` then **persists** the charitable carryover items and QBI REIT/PTP
  carryforward into year+1's stored `ReturnInputs` (`tax.rs:507-509`). `ReturnInputs` carries no
  pseudo taint field. Crypto gains from fictional $0-basis lots inflate AGI → move the §170(b)
  percentage ceilings → change the charitable carryover-out; pseudo-tainted `RemovalLeg`s feed
  `claimed_deduction` directly (`state.rs:196-199` documents the taint; the write-back drops it).
- Next year: the user resolves the pseudo rows or turns the mode off; `pseudo_synthetic_count == 0`;
  the new UX-P4-1 banner (predicate `state.pseudo_active()`, per §3.1) **correctly does not fire** —
  yet year+1's computed return rides carryover-in derived from deliberately-fictional data. The
  fold's own advisory text promises "export/forms are BLOCKED while this is active"
  (`fold.rs:401-404`); the write-back is neither an export nor a form, so the promise is silently
  false for the one path that *persists* a computed figure.

Tax rationale: §170(b)(1)/§170(d)(1) carryover and the §199A(c)(2) REIT/PTP carryforward are
return positions in year+1. A figure derived from a deliberately-fictional basis, stored as an
unflagged input, is a wrong legal position waiting to be filed — and it is invisible to every
disclosure surface this spec builds, including the new banner.

Spec impact: §3.1's problem statement ("The one silent surface is the primary number-bearing one")
is incomplete, and the §1 framing ("never the math") obscures that this cycle's own Important-gated
disclosure item leaves a *persistence* channel open. **Required:** UX-P4-1's decision must gain a
third clause — `--write-carryover` refuses (fail-closed, consistent with the export gate) when
`state.pseudo_active()`, with a KAT (pseudo-active vault + `--write-carryover` → non-zero exit,
year+1 inputs unchanged). A warn-and-write is not acceptable for a persisted input; this app's
posture is fail-closed on exactly this class.

### C2 (Critical, class B): the viewing TUI's Tax tab is a second silent authoritative surface; the fix scope misses it

- `crates/btctax-tui/src/tabs/tax.rs:55-121` (`render_tax_content`) calls `compute_tax_year` over
  the same projected `snap.state` and prints "TOTAL federal tax attributable (delta)"
  (`tabs/tax.rs:93`) plus the year's charitable deduction total (`:116-120`).
- There is **no pseudo handling anywhere in the viewing TUI's tabs** (grep over `tabs/*.rs`: the
  only `pseudo` hits are test fixtures setting `pseudo: false`). No banner, no `[PSEUDO]` row
  markers, no status-line indicator. The view TUI's sole pseudo surface is the export typed-attest
  modal (`tui/src/lib.rs:263-311`) — export-time only, exactly the "other surfaces disclose"
  argument §3.1 itself rejects for the CLI. (The *edit* TUI does show the count —
  `tui-edit/src/draw_edit.rs:97` — the view TUI does not.)
- The specced fix threads `pseudo_active` into `TaxYearReport` → `render_tax_outcome` (CLI only).
  Post-fix, a user with `pseudo on` opens the TUI Tax tab and reads a clean, authoritative total
  computed over fictional lots. The false-negative disclosure — the answered-ness class this
  cycle's only Important-gated UX item exists to close — survives on a primary surface, while §3.1
  affirmatively claims the class is closed ("The one silent surface").

**Required:** either extend §3.1's scope to `render_tax_content` (the same one-signal reuse:
`snap.state.pseudo_active()` is already reachable — the TUI export modal uses it) with a TUI KAT,
or correct the §3.1 claim and file the TUI banner as a phase-owned item **inside this cycle's
correctness cluster** (per the per-phase burndown rule, it cannot ride past the phase that owns
UX-P4-1).

---

## Important findings — detail

### I1 (Important, class C→A): duplicate-refusal must not cover `set-fmv` — the engine's last-wins there is deliberate

`resolve.rs:564-568`: "ManualFmv deliberately keeps latest-seq-wins with NO duplicate blocker — a
valid re-pointing of an FMV is a correction flow, not a conflict"; `:593-597` implements it. The
spec's §3.2 rule ("exact-duplicate re-decide (same target, same op) → refuse: already decided —
void decision|N first") together with §9.3's verb list (which includes `set_fmv`,
`reconcile.rs:85`) would refuse the second `set-fmv` on the same event. That blocks the sanctioned
correction path for a value that **feeds the computed figure** (income FMV → ordinary income →
tax): the correct return needs the corrected FMV, and today it needs no void. The refusal cannot
produce a wrong number by itself (it is loud, and void-then-redecide still works), but it converts
a designed one-step correction into a two-step one against the engine's documented semantics — and
if the record-time predicate and the resolver ever disagree, the record-time layer is the one
that's wrong.

For the first-wins verbs the refusal is *correct and verified consistent with the engine*:
`ClassifyInbound` (`resolve.rs:694-709`), `ReclassifyOutflow` (`:746-762`), `ReclassifyIncome`
(`:807-821`) all raise Hard `DecisionConflict` + first-wins on a duplicate, and the fold's own
remediation text already teaches void-first (`fold.rs:1030-1033`). **Fix:** pin the duplicate
refusal to the first-wins verbs only; `set-fmv` keeps last-wins (optionally echo the engine's
"re-pointing" comment in its `--help`). Also answers §8: warn-and-proceed for duplicates elsewhere
would recreate the trap — refuse is right *for those verbs*.

### I2 (Important, class C): validation predicates must be pinned pseudo-safe (and void-aware)

The spec's cited precedent validates against the **projected** state
(`set_donation_details`, `reconcile.rs:1170` `session.project()`), and `session.project()` uses the
stored config **including `pseudo_reconcile`** (`session.rs:556-562`). Under `pseudo on` the
projection is built as if the synthetic decisions were real: Phase A rewrites `Unclassified` rows'
effective payload (`resolve.rs:943-949` `applied.insert`), Phase B fills `inbound_class`
(`:966-976`), Phase C fills `manual_fmv` (`:1008-1021`). The blockers a validator would naturally
key on (`UnknownBasisInbound`, `FmvMissing`) are exactly what pseudo clears.

Consequence: a projected-state duplicate/type check refuses the user's **real** correcting decision
("already decided" / "wrong type" / "nothing needs classification") — precisely the remedy the new
UX-P4-1 banner instructs ("resolve them before filing"). The engine itself is safe by construction
(real decisions are collected *before* synthetics fill gaps — pass 1d/1e at `resolve.rs:563-780`
precedes the pseudo block at `:933`; `:939-941` skips any target a real decision governs). Only the
new CLI-side layer can get this wrong, and the spec currently leaves it free to.

**Pin in the spec (three clauses):**
1. Record-time validation projects with **pseudo forced OFF** (or equivalently consults the raw
   event log for existence/type and the persisted decision log for duplicates) — never the
   pseudo-tainted projection.
2. "Already decided" means a **live (non-voided) persisted** decision — a naive raw-log scan that
   counts voided priors would break the spec's own remedy ("void decision|N first" then re-decide).
3. Rider on UX-P4-11: `events list` must list pseudo-defaulted events as **decidable** (they have
   no persisted decision), or the banner's remedy path loses its sanctioned discovery verb.

### I3 (Important, class C): appraiser-TIN shape — EIN is a lawful TIN

26 CFR 301.6109-1(a)(1)(i) defines a TIN as an SSN, ITIN, ATIN, **or EIN**; nothing in §170(f)(11),
§6695A, or the regs restricts the Form 8283 appraiser identifying number to SSN shape. Appraisal
practices routinely furnish an EIN. The app's own field contract says so: `cli.rs:653` "Appraiser
TIN/SSN/EIN (Part III §6695A; satisfies the TIN-or-PTIN requirement)" (PTIN has its own flag,
`cli.rs:656-658`). The spec's shape map (`\d{2}-\d{7}` for donee / `\d{3}-\d{2}-\d{4}` for
appraiser-tin) as written refuses a valid EIN-shaped appraiser TIN → blocks a compliant Section-B
filing. **Fix:** `--appraiser-tin` accepts **either** shape (ITIN `9xx-xx-xxxx` already passes
SSN-shape — correct; masked `***-**-1234` correctly refused). Validate at the
`set_donation_details` choke point so the TUI-edit form path is covered (see M3).

### I4 (Important, unsound assumption): the P1-3 warn threshold as pinned misfires on the app's own default

Two formulas appear in §3.3 and they are not the same: "FMV wildly exceeds `sats/1e8 ×
recent-close`" (price-based, sound) vs the pinned guard "FMV > 100× the lot's **cost-basis-implied
value**" (basis-based). Read as basis: (a) a **$0-basis lot** — the app's own conservative default
for self-transfer-ins, and every pseudo default — makes the threshold zero, so *every* donation of
such a lot warns, training the user to ignore the one warning built to catch a $100M sats-as-USD
error; (b) a high-appreciation lot (basis $100, FMV $500k = 5,000× basis) warns although the spec's
very next sentence says "high-appreciation gifts are real. (Refuse would be wrong…)". FMV of
donated BTC tracks market by definition, so only the price-based ratio separates typo from truth.
**Fix:** pin the threshold to `FMV > 100 × sats/1e8 × recent close`, and define the no-price
fallback explicitly (no local close at the date ⇒ skip the warn, or warn "unverifiable" — state
which; silence on the fallback is how the guard silently dies). Warn-only remains correct — no
class-A contact.

### I5 (Important, class B partial coverage): the suffix misses the filed-return totals

The spec's suffix rationale is "so a scraped single line still carries the flag" — but the hook is
`render_tax_outcome` only (`render.rs:1018`; delta total at `:1056-1061`). The dual-report block
(`render_dual_report`, `render.rs:1173`) prints "TOTAL TAX (L24)" (`render.rs:1229`) and
"Absolute TOTAL TAX (this filed return, WITH crypto)" (`render.rs:1247`) — the lines a filer would
actually transcribe onto a return — with no suffix. The unconditional top banner mitigates for a
human reading the whole output, but by the spec's own scraped-line logic the absolute totals need
the flag *more* than the delta does. **Fix:** suffix the dual-report headline totals when
`pseudo_active` (thread the same bool into `render_dual_report`), or state in §3.1 why banner-only
coverage is accepted for that block. (Also note the placeholder-profile estimate path — `tax.rs:272-279`
— renders through the same block-free delta path; the banner+suffix cover it. Verified.)

---

## Class-A trace: per-change math-path verdicts (the §1 invariant)

| Change | Math-path contact | Verdict |
|---|---|---|
| UX-P4-1 | `pseudo_active: bool` added to `TaxYearReport` (`cmd/tax.rs:235-251` — Debug-only struct, no serde, not exported) + render threading. Reads state; writes nothing. | Clean (subject to N1's golden guard). |
| UX-P4-3 | Refusal replaces record-then-exclude(+Hard-block). For first-wins verbs the excluded decision never influenced the fold, so the correct-return projection is identical; the refusal only removes the interim NOT-COMPUTABLE detour. | Clean **iff** I1 (set-fmv exempt) and I2 (pseudo-safe predicates) are folded — else it blocks decisions a correct return needs. |
| UX-P4-4 | Refuses inputs no correct return can contain (negative basis — §1012/§1016/§301(c); acquired-after-receipt — impossible at instant granularity). | Clean with I3/M2/M3 carve-outs; M1 pins guard placement. |
| UX-P1-3 | stderr WARN only. | Clean (I4 fixes the formula, not the invariant). |
| UX-P4-5 | stderr WARN; packet bytes unchanged. Refusing to honor `--forms` slices of a coordinated packet is the tax-sound call (a lone 8949 from a jointly-computed return would misstate). | Clean. |
| UX-P4-6 | New display line from `stats.sigma_pending` (existing accumulator, `state.rs:257-261`). | Clean. |
| UX-P4-7 | Preview formatter; must not be reused by any CSV/form writer (the `[R0-I4]` screen-only rule, `render.rs:57-62`, is the precedent to cite in the plan). | Clean. |
| UX-P4-8 | Error-message context. | Clean. |
| UX-P4-9 | Error variant gains data; single raise site `whatif.rs:234-236`; harvest arms `:530/:534` mechanical. | Clean (N3). |
| UX-P4-10 | Exit code after printing; `NotComputable` renders identically. Exit 1 on NOT COMPUTABLE *exposes* (not hides) the no-filing-ready-number state — fail-loud, correct. | Clean (M5 documents the 1-vs-2 seam + the two deliberate exit-0 cases). |
| UX-P4-11 | Additive read-only verb + a caption line (report golden churn, regenerated). | Clean (I2 rider: pseudo-defaulted events are decidable). |
| UX-P4-12(b–i) | Messages; (i) whichever gate placement is chosen must not change *which year's* packet is exported — placement moves the check, never the stored value. | Clean if that sentence is kept in the plan. |
| M-1 | `preserve_order` verified off the math path: conflict fingerprints are hand-rolled bytes (`persistence.rs:25-55`), typed-struct serde is field-order-declared, remaining `Value` sites are display/tooling/API-parse. | Clean (M4 pins the audit). |
| UX-P1-7/8/10, UX-P2-1, UX-P3-2, N-R1 | Docs/journeys, test matcher, PDF styling, lint hardening. | No product math surface. |

## §8 open questions — this reviewer's answers

- **3.1 predicate:** the projection-wide `pseudo_active()` is the **sound** choice — endorse. A
  per-year row-flag predicate (`any leg/income/removal in year Y with .pseudo`) has a genuine
  false-negative channel: a synthetic can make year Y *computable at all* (clearing a would-be Hard
  blocker) or shift pool composition without any flagged row landing in Y. Projection-wide can only
  over-fire (a Z-only synthetic banners year Y) — the safe direction for a disclosure. Do not
  narrow it. Banner + suffix: keep both, and extend the suffix per I5.
- **3.2 refuse vs warn:** refuse is correct for the first-wins verbs (matches Hard
  `DecisionConflict` semantics); `set-fmv` must be exempt (I1). Warn-and-proceed on duplicates
  would re-create the false-success trap.
- **3.5:** no in-repo script keys on `report`'s exit code (verified: no Makefile/CI usage; xtask
  journeys print `[exit N]` only when non-zero, and no current journey shows a NOT COMPUTABLE
  report — zero golden churn from this change today). 1 = computed-but-refused vs 2 =
  bad-invocation/uncomputable-profile is coherent; document "non-zero ⇒ no filing-ready number"
  (M5).
- **3.6:** `events list` alone is sufficient for this cycle *provided* it is pseudo-aware (I2
  rider); the in-report ref hint can stay deferred (golden churn without a correctness payoff).

## Checked and explicitly NOT flagged

- **`pseudo approve` clearing the banner** while an attested $0-basis figure still feeds the year:
  verified as the designed, user-mandated attest-gate semantics (the synthetic becomes a persisted,
  user-owned decision; "what you see == what you approve", `resolve.rs:209-213`). Not a defect.
- **Negative basis carve-outs:** none exist in law for a capital-asset lot (adjustments floor at
  zero; return-of-capital in excess of basis is gain — §301(c)(2)-(3), §733 — never negative
  basis). The refusal is safe; only zero must survive (it does — spec refuses strictly-negative).
- **Exit 1 hiding an informational state (charter question B):** it does not — NOT COMPUTABLE is
  precisely "no filing-ready number exists"; a success code was the false signal.

---

**VERDICT: 2 Critical / 5 Important / 5 Minor / 4 Nit — NOT green. C1 and C2 (plus I1–I5) must be
folded into the spec and re-reviewed before the PLAN.**
