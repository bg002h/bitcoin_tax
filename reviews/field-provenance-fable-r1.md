# Field provenance — Fable consult r1 (2026-07-30)

**Artifact:** `design/forms/FIELD_PROVENANCE.md` (§1–§6a; §6b on crypto-shredding was written AFTER
dispatch and is NOT covered by this consult).
**Brief:** the one question — what structure should record, per field, WHY it is blank, and does it
belong in the same system as the question registry.
**Reviewer:** Fable, independent, read-only, 13 tool calls; verified the measurements itself.
**Status:** persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2.

---

## Verbatim output

```
RECOMMENDATION: hybrid — keep the field-census unit and the six intents, but restructure into TWO LAYERS: a static per-(form,year) census that records the RULE for every FQN, and a per-return resolution derived from it. The flat six-way conflates the layers: filled/declined/refused are per-RETURN outcomes; not-ours/artifact/never-decided are per-FIELD facts. Written flat, "declined" goes into a static file where it is false for the next filer who answers yes.

TAXONOMY: Static census — every AcroForm FQN resolves to exactly one of:
  1. computed(rule) — we derive and write it
  2. collected(input) — filer-supplied value written verbatim (split from computed: testimony vs arithmetic — §G-11's "no stated zero from unstated inputs" needs this distinction)
  3. asked(QuestionId) — gated by an interview answer; resolves per return to filled, or lawfully blank ("declined" is the no-branch at emit time, not a category anyone writes down)
  4. not-ours, SPLIT: (4a) filer-by-hand — signature/date, carries a "must be completed before filing" duty to surface (an unsigned return is not a return, §6061); (4b) third-party — preparer block, 8283 donee/appraiser. The third-party DESIGNEE is neither: it is an askable class-B election and belongs under asked()
  5. unmodeled(advisory) — benefit out of btctax's scope, forgone by using it, must name its advisory (EIC lines 27–30, direct-deposit 35b–d — the map comment already says this in prose) — MISSING from §3's six
  6. artifact — 1-pt spacer/preprinted-constant cells verify.rs already knows about; no decision encodable
"Not applicable" is per-return liveness of a rule, not a static entry; "refused" is whole-return (screen_inputs), so it folds into asked(); "nothing ever decided" = an FQN absent from the census = the completeness test's red, never an entry.

RECORD-THE-DECLINE: Protection on net, and mostly a non-event — because the durable record ALREADY EXISTS: ReturnInputs persists `foreign_accounts: Some(false)` today, and must (the answered-ness invariant requires it). The census adds structure, not new testimonial content; refusing to hold it would be the active choice, and would protect only a filer who answered falsely — not a lawful design goal, and preparer practice (§6107 workpapers, Form 8867, Circular 230) retains exactly this record; an honest one supports §6664(c) reasonable cause. Two design consequences: (i) keep the census DERIVED — computed at emit from ReturnInputs + registry + map, never a second persisted store that can drift; (ii) the declined pointer must reference the QUESTION ONLY and deref the answer at emit — §3's "pointer to the question and its answer" must not copy the answer (stale-answer hazard), and if diligence evidence is ever the goal, persist a prompt-text hash with each answer, because the depreciation prompt alone has been rewritten three times and a bare QuestionId misstates what was asked.

- One system or two: TWO stores, ONE link, cross-checked. The registry is year-stable tax semantics in btctax-core; the census is per-(form,year) PDF fact and belongs beside — ideally inside — the .map.toml, so `no_unmapped_filled` and the new completeness check read the same universe and cannot disagree. Two tests close it: (map keys ∪ census keys) == the AcroForm FQN set exactly (the missing direction), and every referenced QuestionId resolves against `QuestionId::ALL` (or is compile-checked if the census is Rust).
- Granularity: the question granularity is the FORM's own gating granularity — transcribe its skip-instructions ("didn't pay vehicle-loan interest → skip 22–30"), never invent a coarser compound. One question may retire many FIELDS but only one PREDICATE the form itself states; HsaActivity's compound is legitimate because Form 8889's filing trigger states it compoundly, and Schedule B asks accounts and trusts separately, so they stay separate. Too-coarse's failure mode: a compound "no" spanning distinct legal predicates fabricates precision the filer never swore to. Per-field questions are never needed — provenance is per-field, questions are per-gate, the link is many-to-one.
- The shape should generalize "one question → N fields" to "one DECISION → N fields": the filing-status group is five checkboxes accounted for by one computed selection (four blank-because-sibling-filled), same many-to-one shape as declined, and the record should share it.
- Measurement spot-checked and corroborated: `dump-fields` gives f1040 141, f1040s1 69, f8949 244, f1040sb 72 (68 mapped) — matching §6's table; the counts are trustworthy.
- The label reader keeps exactly the job §2 assigns it (human-readable addresses), and its §6a trailer mis-attribution is fixed for free: region (header/body/trailer) becomes derivable from the census's not-ours entries rather than a geometry patch.

SEQUENCING: Census first, and honestly so. The 496 unaccounted fields are UNMAPPED — the emitter already leaves them genuinely blank, so classifying them changes no rendering and needs nothing from §G-11; the note's "§G-11 blocks the honest version" is overdrawn by one level. What §G-11 actually gates is honest rendering on the MAPPED 662 — a `Usd::ZERO` manufactured from unstated inputs — and the census audit of those 662 (which computed() rules can fire on unstated inputs) is precisely §G-11's worklist. So: census (pure accounting, lands now) → §G-11 spec consuming the census's mapped-field audit → any remapping that follows.

WHAT WOULD MAKE THIS WRONG: The advice assumes the census can stay a derived view over ReturnInputs plus the current binary — if proving what the filer was asked AT ANSWER TIME ever becomes load-bearing (diligence evidence after a prompt rewrite), the persisted prompt-hash layer stops being optional and the "no second store" rule must bend that far.
```

---

## Verification (orchestrator, before folding)

Load-bearing claims re-run rather than accepted. **All four hold** — a marked improvement on the ④
consult, whose single error was a fabricated measurement.

| claim | verdict | evidence |
|---|---|---|
| `ReturnInputs` already persists `Some(false)` for a declined declaration | ✅ | `return_inputs.rs:466` `foreign_accounts: Option<bool>`; `:608` asserts a sparse input keeps the tri-state "unknown" |
| `verify.rs` already knows the **artifact** class | ✅ | `verify.rs:269`, verbatim: *"1-pt preprinted-constant spacer fields are never map targets"* |
| `QuestionId::ALL` exists as a completeness anchor | ✅ | `questions.rs:82`, doc'd as *"the anchor the completeness test iterates"* |
| the third-party designee "belongs under `asked()`" | ✅ (as a recommendation) | grep finds **no** designee modelling anywhere — correctly phrased as where it *should* go, not as an existing fact |

### ★★ It corrected the briefer, and the correction is accepted

The note claimed **"§G-11 blocks the honest version of this."** The consult calls that *"overdrawn by
one level"*, and it is right: the 496 unaccounted fields are **unmapped**, so the emitter already
leaves them genuinely blank — classifying them changes no rendering and needs nothing from §G-11.
What §G-11 actually gates is honest rendering of the **mapped 662** (a `Usd::ZERO` manufactured from
unstated inputs), and the census audit of those 662 *is* §G-11's worklist. Sequencing therefore
inverts: **census first**, and it unblocks §G-11 rather than waiting on it.

### ★★★ NOT COVERED — the crypto-shred requirement arrived after dispatch

`design/forms/FIELD_PROVENANCE.md` §6b (owner, same day, post-dispatch) requires interview answers to
be **cryptographically deletable** after filing. That lands **exactly on this consult's own named
breaking assumption**:

> *"if proving what the filer was asked AT ANSWER TIME ever becomes load-bearing … the persisted
> prompt-hash layer stops being optional and the 'no second store' rule must bend that far."*

A derived census resolves `asked(QuestionId)` by dereferencing the answer in `ReturnInputs`. **Shred
the answer and the provenance becomes unresolvable — a lawfully-blank field silently becomes an
unaccounted one**, which is the very defect the census exists to catch. So the shred requirement
forces a small persisted residue: *the decision's EXISTENCE, separate from its CONTENT* ("declined
2026-04-15, detail shredded"), which is precisely the bend the consult anticipated. Unresolved; it is
the first question for any follow-up round.
