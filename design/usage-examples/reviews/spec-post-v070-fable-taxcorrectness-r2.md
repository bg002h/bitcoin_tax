# Fable adversarial TAX-CORRECTNESS review — SPEC_post_v070_product_cycle.md (r2 re-review)

**Reviewer:** Fable (adversarial tax-correctness red-team; independent of the author and of the
general design/completeness reviewer). Same reviewer as r1
(`spec-post-v070-fable-taxcorrectness-r1.md`).
**Artifact:** `design/usage-examples/SPEC_post_v070_product_cycle.md` **r2** @ `bdd8cbd`,
branch `feat/post-v070-product-cycle`.
**Lens (unchanged, narrow):** (A) math-path contamination vs the §1 invariant; (B) answered-ness
false-negatives on the pseudo disclosure surfaces; (C) valid-return-blocking refusals.
**Method:** every r1 Critical/Important re-traced against current source; every `[T-*]` fold tag in
r2 read in place; the new r2 text (3.3a sign table, 3.3b tz carve-out, 3.3c EIN normalize, 3.2
validator-mirrors-resolver, four-surface suffix) re-hunted from scratch. Files re-read this pass:
`cmd/tax.rs` (write-back + report paths), `btctax-tui/src/tabs/tax.rs`, `btctax-tui/src/unlock.rs`
(`build_snapshot`), `session.rs` (`resolve_screened`/`resolve_all_screened`),
`btctax-core/src/project/resolve.rs` (passes 1c/1d/1e, pseudo phases), `btctax-cli/src/resolve.rs`
(placeholder inject + `Provenance`), `btctax-core/src/state.rs:282`, `main.rs` (all
`parse_usd_arg` sites), `cmd/reconcile.rs` (single-verb + bulk append paths). Authority hierarchy
observed: statute/reg only as law.

---

## Part 1 — r1 Critical/Important resolution table

| r1 ID | Status | Basis |
|-------|--------|-------|
| C1 (write-carryover laundering) | **RESOLVED** | §3.1 clause 4 refuses fail-closed (nonzero exit, persist nothing) before `apply_carryover_writeback` (`tax.rs:507`), on **the** §3.1 predicate; KAT (d) pins byte-identical year+1 inputs + a mutation clause. Predicate/channel audit below. |
| C2 (silent TUI Tax tab) | **RESOLVED** | §3.1 surface 3 threads `snap.state.pseudo_active()` into `render_tax_content` (`tabs/tax.rs:55-121`); threading verified reachable (`Snapshot.state` is `LedgerState`; `pseudo_active` at `btctax-core/src/state.rs:282`; export-modal precedent `tui/src/lib.rs:263-311`). Bonus coverage: `tabs/tax.rs::render` is the App-free entry the **editor** crate also calls (`tabs/tax.rs:18-23`), so the fix covers both TUIs' Tax tabs. TUI golden update pinned. One residual predicate-width caveat filed as R2-M2 (Minor — no live defect, see below). |
| I1 (set-fmv exempt) | **RESOLVED** | §3.2 exempts `set-fmv` with the engine's own rationale (`resolve.rs:564-568/593-597` — "deliberately keeps latest-seq-wins with NO duplicate blocker"; re-verified verbatim this pass) and pins the duplicate refusal to the three first-wins verbs with correct cites (`:694-709`, `:746-762`, `:807-821` — all re-verified: Hard `DecisionConflict` + first-wins). Success-KAT "second `set-fmv`" present. (List omits a fourth first-wins verb — filed as R2-M1, Minor.) |
| I2 (pseudo-OFF / void-aware predicate) | **RESOLVED** (with one new adjacent defect, R2-I1) | §3.2 pins: pseudo-OFF view, live (non-voided, non-synthetic) decisions, never the tainted projection (`session.rs:556-562` correctly identified as the taint source); §3.6 carries the rider (pseudo-defaulted events list as decidable); KATs include void-then-re-decide and first-real-classify-over-a-pseudo-default — both directions. The pseudo-taint channel I raised is closed. However the *specific formulation* of the pseudo-OFF view ("raw event log for existence/type") introduces a new false-refuse — see **R2-I1**. |
| I3 (EIN-shape appraiser TIN) | **RESOLVED** | §3.3c: `--appraiser-tin` accepts EIN-shape OR SSN-shape, citing 26 CFR 301.6109-1(a)(1)(i) (correct law: a TIN is SSN/ITIN/ATIN/**EIN**); ITIN passes SSN-shape; masked refused; PTIN gets its own shape or explicit exclusion; validation at the `set_donation_details` choke point (`reconcile.rs:1162`) so the TUI-edit form path is covered. Matches the field's contract (`cli.rs:653`). |
| I4 (price-based FMV warn) | **RESOLVED** | §3.3d pins the price-based formula `FMV > 100 × (outflow_sats/1e8) × close` and explicitly rejects the cost-basis reading (naming the $0-basis common case); the no-price fallback ("skip the warn") is stated explicitly with the silent-death rationale — exactly the fix I demanded. Warn-only retained (correct — no class-A contact). One refinement filed as R2-N1 (event-date close beats "recent"). |
| I5 (dual-report suffix) | **RESOLVED** | §3.1 surface 2 threads the same bool into `render_dual_report` (`render.rs:1173`) and suffixes "TOTAL TAX (L24)" (`:1229`) + "Absolute TOTAL TAX …" (`:1247`) — the filer-transcribed lines. The placeholder-estimate path (`tax.rs:272-279`) note is carried. Anchors re-verified. |

r1 Minors/Nits — fold spot-check (non-gating): M1→3.3a per-flag table (present); M2→3.3b
(receipt-date+tz in the message, same-day allowed, PLAN option preserved); M3→3.3c (normalize +
"omit `--donee-ein`" message + choke point); M4→§4 M-1 blast-radius enumeration pinned into the
KAT; M5→3.5 (1-vs-2 documented, "key on non-zero", both deliberate exit-0 non-triggers, stale
`tax_report.rs:780` doc); N1→KAT (a) byte-diff golden guard; N2→leading space kept, clause 1;
N3→3.4 (harvest arms + `InvalidTarget→NoLots`); N4→banner text points rows→`report`,
advisory→`verify`. All folded faithfully.

### C1 deep-verification detail (the questions the re-review charter poses)

**Is the refuse predicate the banner predicate, and is that the right gate?** Yes. §3.1 clause 4
refuses "when the predicate holds", and §3.1 defines exactly one predicate
(`pseudo_active() OR PseudoPlaceholder`). At the gate site the `PseudoPlaceholder` half is
additionally **structurally unreachable**: `write_back_carryover` already refuses any
non-`ReturnInputs` provenance (`tax.rs:478-483`, re-verified this pass), and the placeholder
provenance (`btctax-cli/src/resolve.rs:121-128`, `Provenance::PseudoPlaceholder` `:30`) can never
equal `ReturnInputs`. So the operative half is `pseudo_active()` — which is precisely the C1 taint
(synthetic lots/FMVs in `state` feeding `assemble_absolute(&ri, &state, …)` at `tax.rs:486`).
Including the OR is harmless and keeps one predicate app-wide. Correct.

**Could a tainted carryover slip through another persist path?** Audited every non-test writer to
the stored `ReturnInputs` (grep `return_inputs::set`):
1. `tax.rs:509` — the gated write-back itself. Closed by clause 4.
2. `tax.rs:101` (`income import`) — whole-blob upsert that **preserves** a prior
   `CarryProvenance::Computed` carryover across a re-import (`tax.rs:66-100`). This is a
   *derivative* channel: it can only preserve what `--write-carryover` previously wrote. With the
   clause-4 gate, no pseudo-tainted `Computed` carryover can ever exist to be preserved. Closed
   transitively. (TOML-supplied carryover is the user's own assertion — `User` provenance — not
   laundering.)
3. `answer.rs:212` / `input_form_store.rs:299` — interactive input-form authoring; user-typed
   values only (the input-form attribute map treats the carryover-in leg as a deferred *form
   field*, `btctax-input-form/src/attribute.rs:196` — nothing seeds from a computed year). Not a
   laundering channel.
`apply_carryover_writeback` has exactly one production caller (`tax.rs:508`). `--force` only
overrides the user-carryover clobber guard downstream of the gate. No other persist path exists.

**Early-mutation check:** `coherence_clear_or_refuse` (`tax.rs:454`) mutates the in-memory DB
before the gate would fire, but persistence happens only at `s.save()` (`tax.rs:510`); an `Err`
return discards it. KAT (d)'s byte-identical assertion operationally pins this. Sound.

### C2 deep-verification detail

`render_tax_content` computes from `snap.profiles.get(&year)` resolved in `build_snapshot`
(`unlock.rs:171-219`) via `session.resolve_all_screened` (`session.rs:488-519`) — which passes the
stored `pseudo_reconcile` to `resolve_and_screen`, so I hunted the placeholder-profile channel on
this surface: it is **unreachable**. `resolve_all_screened` enumerates only years with a stored
`TaxProfile` or `ReturnInputs` (`tax_profile::years ∪ return_inputs::years`, `session.rs:497-498`);
a placeholder-eligible year (pseudo on, nothing stored) is never enumerated, so `snap.profiles`
never holds a `PseudoPlaceholder` profile, and the tab renders `compute_tax_year(…, None, …)` →
NOT COMPUTABLE — a reason, never a number (`tabs/tax.rs:59-63/68-70`). Hence clause 3's
`pseudo_active()`-only predicate is sound *today* — but only by an unstated structural invariant;
see R2-M2.

---

## Part 2 — new findings on the r2 text

### R2-I1 (Important, class C): §3.2's pinned pseudo-OFF view falsely refuses targets whose type comes from a live real `ClassifyRaw` — contradicting the resolver and the spec's own mirror mandate

§3.2 pins: "Record-time validation MUST consult a **pseudo-OFF** view: **the raw event log for
existence/type** + the persisted (void-folded) decision log for duplicates."

The type half is wrong. The resolver validates every target against the **effective** payload,
not the raw one: `applied.get(target).unwrap_or(&raw.payload)` (`ClassifyInbound`
`resolve.rs:728-730`-adjacent, `ReclassifyIncome` `:789-791`, `ManualFmv` pass 1d `:575-577`),
where `applied` is populated by live **real** `ClassifyRaw` rewrites in pass 1c (`resolve.rs:543-560`)
*before* pseudo Phase A adds synthetic ones (`:934-953`). So an `Unclassified` row that a real
`ClassifyRaw` decision turned into `Income` is a **valid** target for `set-fmv` (ManualFmv→Income)
and `reclassify-income` — the resolver accepts it — while a raw-log type check refuses it as
wrong-type.

This is not exotic: it is the **post-`pseudo approve` correction workflow**. `pseudo approve`
persists real `ClassifyRaw` zero-value placeholders (`resolve.rs:223` documents the shape); the
sanctioned way to put the *true* FMV on that row afterwards is `set-fmv` — the exact decision a
correct income figure needs (§61/§83-style FMV-at-receipt inclusion; the number feeds ordinary
income). Under the pinned rule, record-time validation refuses it ("target is not Income" per the
raw log) although the resolver would honor it. That is a valid-return-blocking false refuse —
the harm class the spec's own §1 amendment (`[G-§1]`) elevates to invariant-harm — and it directly
contradicts §3.2's final bullet ("validator-mirrors-resolver … if they ever disagree, the
record-time layer is the wrong one"). The r2 text is internally inconsistent: one bullet mandates
mirroring, an earlier bullet pins a view that does not mirror. The KAT set cannot catch it — no
KAT exercises a `ClassifyRaw`'d target.

The refusal is loud and a workaround exists (void the ClassifyRaw, re-classify-raw with a
corrected payload), so this is Important, not Critical — but it must not survive into the PLAN.

**Required fix (textual):** replace "the raw event log for existence/type" with "the **effective
payload under live real decisions** (raw log + void-folded real `ClassifyRaw` rewrites; synthetics
excluded) for existence/type" — i.e., exactly the shadow-projection option the mirror mandate
already names — and add a success KAT: `set-fmv` (and `reclassify-income`) on a target whose
Income type comes from a live real `ClassifyRaw` is ACCEPTED; the same target with the
`ClassifyRaw` *voided* is refused wrong-type. Mutation reds.

### R2-M1 (Minor): the first-wins verb list omits `ClassifyRaw`; the validation choke point is unpinned (bulk paths bypass)

Pass 1c is a fourth first-wins verb: a `ClassifyRaw` on an already-overridden target raises
`DecisionConflict` and the first rewrite wins (`resolve.rs:543-560`), and the verb is CLI-exposed
(`reconcile classify-raw`, help-tested at `cli.rs:1036-1040`). §3.2's refusal list
(ClassifyInbound/ReclassifyOutflow/ReclassifyIncome) leaves the record-then-conflict trap open for
it. No wrong number and no false refuse (the resolver still hard-blocks loudly) — a coverage gap,
not a correctness defect. Relatedly, §3.2 never names *where* the validation lives: the single-verb
append fns (`reconcile.rs:41/62/85/110/1136`) are the natural choke point, but the bulk `apply_*`
paths append directly via their own `append_decision` loops (`reconcile.rs:286/395/438` etc.) and
would bypass a single-verb-sited validator. Bulk refs are plan-generated (not user-typed), so the
bypass is mostly benign — but say so: pin the choke point and explicitly scope bulk in or out, so
the PLAN can neither silently skip it nor invent a validate-batch-then-append-batch shape (whose
intra-batch adjudication could diverge from the resolver's ascending-seq first-wins).

### R2-M2 (Minor): the TUI surface's narrower predicate is sound only by an unstated invariant — state it or thread the full predicate

§3.1's predicate is `pseudo_active() OR PseudoPlaceholder`; clause 3 threads only
`snap.state.pseudo_active()`. As verified above, that is safe today solely because
`resolve_all_screened` never enumerates an un-stored year (`session.rs:497-498`), so a
`PseudoPlaceholder` profile can never reach `snap.profiles` and the Tax tab fail-closes to NOT
COMPUTABLE. That justification lives in an enumeration set two crates away — answered-ness held by
convention, not construction, which is this codebase's one recurring architectural defect class. A
future snapshot-builder change (e.g., giving the TUI parity with the CLI's placeholder-estimate
path) would silently reopen the placeholder false-negative on surface 3 with no KAT to red.
**Fix:** one sentence in §3.1 clause 3 stating the invariant that licenses the narrower signal,
plus a KAT: pseudo on + no stored profile/inputs for the selected year → the TUI Tax tab shows NOT
COMPUTABLE (a reason, never a number).

### R2-M3 (Minor): 3.3a sign-table site gaps

(i) The `what-if sell`/`what-if harvest` **ad-hoc profile money flags** — `--income`, `--magi`,
`--carryforward-in` (`main.rs:347-353` and `:421-427`) — appear in no table row. `--carryforward-in`
is a loss *magnitude* (a negative is nonsense-in / wrong-planning-estimate-out); `--income`/`--magi`
raise the same per-field questions the tax-profile row defers to the PLAN. Planning-only surface
(no filed-form contact), hence Minor — but the table claims to be the per-flag policy over all
~25 sites, so the PLAN needs these rows. (ii) The tax-profile row's cited range `main.rs:852-885`
is stale: the money fields run through `:907`, and `:890-912` shows three of them
(`--w2-ss-wages`, `--w2-medicare-wages`, `--schedule-c-expenses`) **already negative-guarded** —
cite them as the in-repo precedent and exclude them from re-work. (`--prior-taxable-gifts` at
`:126-139` is likewise already guarded — correctly cited as the pattern precedent.)

### R2-N1 (Nit): use the outflow-date close, not "recent-dataset-close", as the warn multiplier

FMV for a charitable contribution is value **at the contribution date** (26 CFR
§1.170A-1(c)(2)); the daily-close dataset covers that date via the same `session.prices()`. The
"recent" close only misbehaves after a >100× post-event price move (false warn on a >100×
collapse), and it is warn-only — hence Nit — but the event-date close is strictly more principled
at identical cost. Keep the stated skip-on-no-price fallback either way.

### R2-N2 (Nit): hyphenless donee-EIN normalization has an inherent SSN ambiguity — note it, don't "fix" it

Normalizing `123456789` to EIN-shape necessarily also accepts an unhyphenated SSN (the SSN-shape
refuse only fires on the hyphenated form). No shape-level check can distinguish them; the
normalization is still the right call (refusing hyphenless would refuse real unformatted EINs —
my r1 M3). Add one sentence acknowledging the asymmetry so a future pass doesn't "harden" it into
a false refuse.

### R2-N3 (Nit): name the threaded bool after the predicate, not one disjunct

§3.1 clause 1 says "thread `pseudo_active` into `TaxYearReport`" — the field must carry the full
predicate (`pseudo_active() OR PseudoPlaceholder`), and naming it `pseudo_active` invites an
implementer to thread `state.pseudo_active()` literally and drop the OR. KAT (b) would red that
mutation (good — the guard holds), but call the field `pseudo_contributed` (or similar) so the
name cannot argue with the predicate.

---

## Re-hunt summary against the three charter classes (r2 text)

- **(A) Math-path contamination:** none new. The r1 class-A trace stands; r2's additions (banner
  threading, refusals, warn, exit codes) are all read-only or refuse-only, and KAT (a)'s byte-diff
  golden guard operationalizes the §1 invariant for the disclosure change.
- **(B) Answered-ness false-negatives:** the two r1 Criticals are closed (C1 gate verified
  channel-complete across every `ReturnInputs` writer; C2 verified threaded and reachable). The
  one residual is structural-by-convention, not live (R2-M2). The four-surface suffix has **no
  false-positive channel that misleads**: the vault-wide over-fire direction was endorsed in r1
  §8, the r2 banner text is now a true vault-level statement, and every case where the suffix
  fires on a "real-looking" number (placeholder profile; displaced HIFO selection) is a number
  genuinely riding a synthetic assumption.
- **(C) Valid-return-blocking refusals:** one new (R2-I1 — the raw-log type check refuses the
  sanctioned post-approve `set-fmv`/`reclassify-income` correction). The 3.3a table wrongly
  refuses no legitimately-negative flag (the only legitimate negatives — `--other-net-capital-gain`
  and friends — sit in the deferred-per-field row with "default allow"); the tz carve-out and
  EIN normalization open no refusal hole.

---

**VERDICT: 0 Critical / 1 Important (R2-I1) / 3 Minor / 3 Nit — NOT green. C1, C2, and I1–I5 are
all genuinely resolved; the single blocking item is a one-sentence re-pin of §3.2's pseudo-OFF
view (effective-payload-under-live-real-decisions, not raw-log, for existence/type) plus its
ClassifyRaw'd-target success KAT. Fold and re-review.**
