# SPEC review — `design/SPEC_input_form.md` (r3 fold), independent Fable pass r2

*Persisted VERBATIM (STANDARD_WORKFLOW §2). Reviewer: Fable (same independent reviewer, re-reviewing its own
r1 findings' fold). Persisted 2026-07-14 against HEAD `ea66351`. Folded in spec r4.*

---

I've verified the fold against the current spec text and the source. All originals check out except two new gaps the fold introduced, both around the `parked`/`Fresh` machinery.

## VERDICT: 0 Critical / 2 Important

The r2 findings are genuinely resolved (not gestured at) — confirmations at the end. But the C-1 and I-9 folds each left an integration seam that reopens the exact class of defect they closed.

---

## NEW — IMPORTANT

**NI-1. §5.7 / §6.1 / §9 — `save_draft` silently downgrades a `parked=1` row to WIP the moment the user edits a parked return, stripping the C-1 protection mid-edit.**
The `parked` flag lives only in the DDL (`parked INTEGER NOT NULL DEFAULT 0`, §6.1), but the API that writes drafts cannot carry it: `save_draft(sess, year, ri)` (§5.7) has no `parked` parameter, and `Loaded::Draft(ReturnInputs)` carries no `parked` field, so the renderer never learns a loaded draft was parked. Trace:

1. Park 2024 (§9) → draft row `parked=1`, no committed row; it is the sole copy of a screened return, SSNs included.
2. Reopen the form → `load` returns `Draft(ri)` (no parked signal). §9 itself says "the parked blob *becomes* the WIP."
3. User edits one field → autosave fires `save_draft(sess, 2024, ri)` → the upsert writes the row with `parked` at its DDL default `0` (nothing preserves it).
4. Protection is now gone silently: an external `income import` (§6.2) deletes it as WIP, and a stale-version resume (§6.3) discards it — the precise sole-copy data-loss C-1 exists to prevent, reopened for the whole edit window before re-commit.

The two paths C-1 explicitly names (external write, stale version) *are* closed for a pristine parked row; this is the adjacent path the fold opened. Fix: `save_draft` must preserve the existing `parked` flag (read-modify-write, or an explicit param); `Loaded::Draft` must carry `parked` so the renderer round-trips it (and the §9A status line / toggle can read it); and the spec must state that editing a parked return keeps `parked=1` until a successful re-commit consumes the row — resolving the tension with §9's loose "becomes the WIP."

**NI-2. §5.7 / §6 / §6.1 — the I-9 "commit blocked until `filing_status` chosen" guarantee is assigned to a function that structurally cannot enforce it, and the in-scope web renderer would ship the laundered `Single`.**
`filing_status` is a non-`Option`, frozen enum (`types.rs`, OUT of scope to change), so any `ReturnInputs` handed to `commit(sess, year, &RI, table, params)` has a valid variant — `commit` cannot distinguish "chosen `Single`" from "defaulted `Single`." Yet §6 ("commit → require filing_status chosen (I-9) → screen_inputs") and §6.1 ("commit … first requires `filing_status` explicitly chosen") place the check *inside* commit. The only place chosen-ness exists is renderer state (§5.8 says so), but the spec never states that **both** renderers must enforce it — and the web front-end is a day-one seam consumer (§4/§13). A web renderer that synthesizes `ReturnInputs::default()` and POSTs a commit reintroduces exactly the I-9 laundering, now relocated past the gate. Worse, a renderer "chosen" bool is answered-ness held by convention — the project's one named architectural sin (`answeredness-invariant`), walking back in through the form.
Fix: make it constructive, not conventional. The working model is `Option<ReturnInputs>` (`None` on `Loaded::Fresh`; the first accepted `Edit` *must* set `filing_status`, which materializes the RI — this also resolves the unstated question of how `Fresh` yields the `&mut ReturnInputs` that `apply` requires). Then "chosen" ≡ "RI exists," `commit` only ever receives a materialized RI, and both renderers inherit the guard by construction. State this as the contract; drop the impossible check from `commit`.

---

## MINOR (new, from the fold)

- **M-a. §5.2 vs §5.7 — contradictory `FieldKind` definitions.** The I-8 fold added `Bool` to `FieldKind`/`FieldValue` in §5.7, but §5.2's enumeration still reads `{ Money, Text, TriState, Date, Enum, Secret }` — no `Bool`. An implementer reading §5.2 as the definition cannot represent `presidential_fund_*`. Update §5.2.
- **M-b. §9A — the left-pane section order omits `Payments`.** M-1 pulled Payments into scope (§2/§5.7 `SectionId`/§5.8), but §9A's ordered list is still `ReturnOptions → … → Schedule A? → Declarations → Skippables` with no Payments slot. The renderer has no defined placement for the new section.
- **M-c. §7 — `attribute()`'s exhaustive match has no concrete `Anchor` for `NonCryptoNoncashGift`.** The map cell gives prose ("see the honesty note"), not an `Anchor`; the exhaustive `match` still needs an arm. It is reachable from v1 form data (a `CapGainProp*`/`OrdinaryProp*` gift > $500 in `ScheduleACharitable` triggers it at `report` — `return_1040.rs:598-609`, verified), so `Section(ScheduleACharitable)` is the honest anchor (or `NotInForm`). Assign one.
- **M-d. §6.2 — the parked-year refuse message names a nonexistent remedy.** "year {y} holds a parked full return; toggle it back or **discard it first**" — no "discard a parked draft" command is defined anywhere in the spec. The toggle-back exit does exist, so it is not a brick, but either define the discard path or reword (the repo's own doctrine is "a refusal with no exit is a brick").

---

## Confirmations — every r2 finding verified resolved against source

- **C-1:** DDL gains `parked`; §6.1 prose, §6.2 RULE (refuse, not delete), §6.3 (refuse-and-reimport) all give `parked=1` committed-row semantics. The two originally-named loss paths are closed. Park's own delete correctly bypasses the coherence hook. **Residual: NI-1 above.**
- **I-1** ip_pin S + coverage-KAT assertion ✓. **I-2** Secret out / SecretEntry in, §10 asymmetry carve-out ✓. **I-3** PrivateActivityBondAmt → NotInForm (1099 box, `return_refuse.rs:734,769`) ✓. **I-4** SingleEmployerExcessSs → W2s box4 (`:702`), removed from NotInForm ✓. **I-5** ScheduleBPart3Unanswered → both, exactness qualified (`questions.rs:120,135`) ✓. **I-6** income answer in RULE + §10 ✓. **I-7** autosave = save_draft + Vault::save, debounced (`vault.rs:231-245`) ✓. **I-8** Bool kind ✓ (modulo M-a). **I-9** Loaded::Fresh + commit block + confirm ✓ (modulo NI-2). **I-10** DeleteSection(ScheduleA) resets itemize_election (`return_1040.rs:391,397`) ✓. **I-11** commit TY2024-only, NoTables ✓.
- **M-1** Payments in §2/SectionId/§5.8; exempt list drops payments ✓ (modulo M-b). **M-2** SALT anchors verified vs `income_tax_salt` (`return_1040.rs:115-122`) ✓. **M-3** deferred legs added ✓. **M-4** NonCryptoNoncashGift in honesty note ✓ (modulo M-c anchor). **M-5** owned Choice(String) ✓. **M-6** ClearField per kind, Enum→SetError ✓. **M-7** btctax-input-form throughout ✓. **N-1..4** ✓.
- **§7 exhaustiveness:** re-counted — 37 variants, 37 placed, each once, no double-listing. Genuinely exhaustive.
- **§5.8 inventory:** complete with ip_pin + Payments; no deleted field resurrected; enum lists exact.

The two Important items are integration seams in the new `parked`/`Fresh` machinery, not disagreements with the design — both closable with the stated fixes without disturbing the resolved findings.
