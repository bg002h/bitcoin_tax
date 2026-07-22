# Plan review — architecture lens (Fable), round 1

**Artifact:** `design/conservative-filing-approach-b/IMPLEMENTATION_PLAN.md` @ `ad16e3a` (branch `feat/conservative-filing-b`)
**Reviewer:** independent software-architecture lens (Fable). Verified against current source at HEAD; symbols cited, line numbers indicative.

## Verdict

**NOT GREEN — 0 Critical / 6 Important / 4 Minor / 2 Nit.** The plan's per-task source citations are
overwhelmingly accurate (I verified every cited symbol/region in resolve.rs, fold.rs, void.rs, pools.rs,
conservative.rs, event.rs, persistence.rs, admin.rs, render.rs, session.rs, both main.rs, packet.rs ×2,
census/sp2/sp3, form8283/pdf/verify/map, Makefile — the :1085-1114 admit branch, :1356-1382 `allocation_voids`,
:2171 / :3844 catch-alls, `CENSUS_KEYS[14]` :29, the no-`..` destructure packet.rs:36-57, `verify_flat` :337,
`write_basis_methodology_txt` call sites :871/:911 all check out exactly). The blocking findings are
decomposition/wiring gaps: the `PromoteSet` has no owning task and cannot reach the fold as written; the T7
deferred-void adjudication is directed to an insertion point that runs too late to work; two spec-mandated
surfaces (the BG-D9 advisory wiring, the BG-D3 `verify` drift advisory) have no owning task; and Phase 1b has
two buildability/coverage gaps (`Printed8275`, the 8275 year set).

---

## Important

### I-1 (T2/T3/T4/T5/T6) — `PromoteSet` is unowned, type-inconsistent, and cannot reach the fold as written

- **Defect:** the type every leg-builder task consumes is never produced by any task, and the resolve→fold
  threading it needs is unassigned.
- **Concrete failure:** T4/T5/T6 consume "`PromoteSet` (T2/T3: `origin_event_id -> {filed_basis, tranche_sat}`)",
  but T2's Produces block defines only `ComputedFloor`/`filed_basis_for` (no `PromoteSet`/`PromoteEntry`), and
  T3's Produces is `fn live_promotes(...) -> BTreeMap<EventId, Usd>` — **no `tranche_sat`**, which the BG-D4
  denominator (`estimate_share = filed_basis × leg_sat / tranche_sat`) requires in T4 step 3, T5 step 3, and
  T6 step 3. Under the plan's own isolation rule ("a task's `Produces` block is the only place a later task
  learns its neighbors' exact names/types"), the T4 implementer cannot even name the type. Worse, T4's "the
  caller has it from resolve/fold ctx" is false against current source: `make_disposal_legs` (fold.rs:122),
  `make_removal_legs` (:225) and `consume_fee` (:323) are called from `fold_event` (:554), whose only context
  is `FoldCtx { config, elections, selections }` (fold.rs:21-25), populated in `fold` (:376) from
  `Resolution` (resolve.rs:201) — neither `Resolution` nor `FoldCtx` carries a promote set, and no task adds
  the field to either. `live_promotes` is a resolve-internal `fn` in T3; nothing exports it.
- **Code fact:** `FoldCtx` fold.rs:21; `Resolution` resolve.rs:201-202; `fold(res, prices, config)` fold.rs:376;
  the six builder call sites fold.rs:362/:635/:641/:832/:1118/:1122/:1195/:1199.
- **In-plan fix:** give the type ONE owner. Recommended: T3's Produces defines
  `pub struct PromoteEntry { filed_basis: Usd, tranche_sat: Sat }` and
  `pub type PromoteSet = BTreeMap<EventId, PromoteEntry>` in `conservative_promote.rs` (matching the
  File-Structure map), has `live_promotes` return it (`tranche_sat` = the target `DeclareTranche.sat`), and
  adds `promotes: PromoteSet` to `Resolution`. T4 step 3 then explicitly adds `promotes` to `FoldCtx`, threads
  it in `fold`, and lists all builder call sites to update. Also re-produce `live_promotes`' final signature in
  T7 (its T3 shape has no way to push the T7(c) `DecisionConflict`s — it needs `&mut Vec<Blocker>` or a
  conflicts return).

### I-2 (T7) — the deferred tranche-void adjudication is directed to an insertion point that cannot work

- **Defect:** step (d) says "In step 3 (mirror `allocation_voids` :1356-1382), adjudicate `tranche_voids` …
  else the void applies" — but resolve's section 3 runs AFTER the section-2 timeline build, so an applying
  tranche-void has no effect there.
- **Concrete failure:** the DeclareTranche admit branch (resolve.rs:1087, `if voided.contains(&e.id) { continue }`)
  has already pushed the tranche's `Eff` by the time section 3 runs; `voided.insert` at :1356+ is a no-op for
  the timeline, and `universal_snapshot(&timeline, …)` (resolve.rs:~1287, section 3's conservation/backstop
  input) has already counted the tranche. `allocation_voids` works at :1356-1382 only because a
  `SafeHarborAllocation` is never a timeline `Eff` — there is no timeline-removal machinery to "mirror".
  Executed exactly as written, `both_voids_either_order_converge_no_brick` reds (the $0 lot survives) with no
  in-plan remedy, and the implementer is pushed toward inventing a post-hoc timeline scrub — which would still
  leave the §7.4 snapshot wrong.
- **Code fact:** section order in resolve.rs: 1a :453 → 1b :498 → … → 2 :1071 (admit :1087-1114) → 3 :1240
  (snapshot ~:1287; `allocation_voids` loop :1358-1382).
- **In-plan fix:** adjudicate `tranche_voids` immediately AFTER the pass-1a loop completes (promote-liveness
  depends only on promote-targeted voids, which pass-1a applies inline and unconditionally — the SPEC's own
  acyclicity argument), inserting the applying void's target into `voided` BEFORE step 2; keep the
  conflict-arm blocker copy mirroring `allocation_voids`. This preserves the settled deferred-adjudication
  ruling (adjudicate against the FINAL non-voided-promote set) — only the plan's `:1356-1382` location is
  wrong. Also pin step (b)'s ambiguity: at pass-1a time defer EVERY void whose target is a `DeclareTranche`
  that has ANY promote event in the ledger (never evaluate "live" inline — that re-opens arch r2 M-1).

### I-3 (T8/T10) — the BG-D9 prior-year advisory is never wired to any user path (both directions)

- **Defect:** T8 builds `promote_prior_year_advisory` and tests it directly; no task calls it.
- **Concrete failure:** T10 step 3's promote flow renders only the T9 consent screen — it never invokes T8's
  advisory lines (the mandated "requires a Form 1040-X for Y with the 8275 attached" / §6511 / cascade copy),
  and NO task touches the void path at all, though the SPEC mandates "the VOID direction gets the SAME
  advisory" (amend-to-pay). `Direction::Void` has zero callers in the plan. The five existing tranche
  advisories surface via `render.rs`, which has no promote-id/direction context — T8's record-time-shaped API
  can only surface from the record verbs. The whole suite goes green with the advisory as dead code — exactly
  the silent miss the spec's BG-D9 block exists to prevent. The File-Structure map also contradicts T8: it
  assigns `tax/compute.rs` + `cmd/tax.rs` "carryover-cascade naming hooks (T8)", but T8's own Files/commit
  touch only `conservative.rs`.
- **In-plan fix:** add a wiring step to T10 (call + print the `Direction::Promote` lines before the consent
  prompt) and an explicit void-path hook (the CLI void verb / bulk-void path detects a promote-or-
  promoted-tranche target and prints the `Direction::Void` lines) with a CLI-level test each; reconcile the
  file map with T8's Files (either add the compute.rs/cmd/tax.rs steps to T8 or delete the map entry).

### I-4 (coverage) — BG-D3's `verify` drift advisory has no owning task; the self-review's "No gap found" is wrong

- **Defect:** the Global Constraints quote "`verify` flags drift, direction-aware" but no task implements it.
- **Concrete failure:** SPEC §2 BG-D3 mandates "`verify` recomputes and surfaces any drift as an advisory only —
  direction-aware … a stored floor now recomputing above the reference on a not-yet-filed position earns a
  'void + re-promote to the corrected lower number' hint", and §6 pins "the stored number survives a
  price-data change (fold uses stored, **verify flags drift**, direction-aware per N-3)". Grep the plan: the
  only "drift" hits are the constraint blurb and T16's anti-drift destructure. The spec-coverage map claims
  "BG-D3 → T2", but T2 is only compute+refuse. §5 non-goals do not exclude it. This is the
  spec-mandate-without-owner class (project memory: don't defer a spec mandate on false cover).
- **In-plan fix:** add a task (or a T11 sub-step, since it's advisory-surface work) modifying the verify
  surface (`verify_report` path) to recompute `filed_basis_for` per live promote, compare to the stored
  `filed_basis`, and emit the direction-aware advisory; name the §6 KAT (stored-survives-price-change +
  drift-above → re-promote hint).

### I-5 (T15/T16) — `Printed8275` is consumed by both Phase-1b tasks but created by neither

- **Defect:** T15's `fill_form_8275(printed: &Printed8275, …)` and T16's `PrintedForms.f8275: Option<Printed8275>`
  both consume a newtype "in `tax/printed.rs` mirroring `Printed8283Rows`" (printed.rs:135) that no task's
  Files/Produces/commit creates.
- **Concrete failure:** T15's Files list and `git add` are btctax-forms-only; T16 modifies `tax/packet.rs` but
  not `tax/printed.rs`. Executed as scoped, T15 does not compile (`Printed8275` undefined in btctax-core), and
  the Produces-block isolation rule means neither implementer knows who defines it or what it carries from
  `Disclosure8275` (T13).
- **In-plan fix:** add `crates/btctax-core/src/tax/printed.rs` to T15's Files + commit, and a Produces line
  defining `Printed8275` (what it wraps — e.g. the rendered Part I items + Part II string from `Disclosure8275`
  — and its constructor), or move the definition to T13 next to `Disclosure8275`.

### I-6 (T15/T16) — 8275 year coverage vs `SUPPORTED_YEARS` is left open; the shipped default refuses every promoted 2017/2025 export

- **Defect:** T15 authors only `forms/2024/f8275.{pdf,map.toml}` and says "extend to 2017/2025 … **if** the map
  is authored"; T16 gates every export on the PDF artifact.
- **Concrete failure:** `SUPPORTED_YEARS = &[2017, 2024, 2025]` (btctax-forms/src/lib.rs:61). With a 2024-only
  8275 asset, a promoted leg filed in 2025 (the dominant current-year flow) or 2017 makes `fill_full_return`
  abort (all-or-nothing) and the re-pointed BG-D8 gate a PERMANENT refusal — the "complete, shippable 1a+1b
  unit" bricks those years while every plan KAT (2024-fixtured) stays green. Form 8275 is revision-versioned,
  not year-versioned, so nothing prevents full coverage.
- **In-plan fix:** make T15 step 3 mandatory, not conditional: alias the single bundled 8275 revision to every
  year in `SUPPORTED_YEARS` (`f8275_pdf(year)`/map `for_year` return the same asset), and add a per-year fill
  KAT in sp4 + a T16 census/gate KAT for a non-2024 promoted year.

---

## Minor

### M-1 (T1/T2/T3/T6/T7) — `Usd::from_dollars` does not exist

`pub type Usd = Decimal` (conventions.rs:8); no `from_dollars` anywhere in the workspace. Every test snippet
using it fails to compile. Harness precedent is `dec!(…)` (kat_tranche.rs:87). Mechanical fix — sweep the
snippets to `dec!`/`Usd::from`, but do it in the plan text so task-isolated implementers don't each rediscover
it (the self-review's "the exploration's exact symbols" claim is false here).

### M-2 (T4/T11) — SPEC census items 6/7 (parent-doc + parent-test re-scope) have no explicit owner

SPEC §3 item 6 mandates amending parent D-7 ("nothing >$0 ever filed"), the parent Invariant KAT "amended per
BG-D4", and the `event.rs` `DeclareTranche` doc ("v1 declares $0 ONLY (no floor)" — event.rs:~218); item 7
re-scopes every "$0-only" test. T4 step 4 instead asserts the parent Invariant KAT "still green" (unamended),
and T11's `"$0"` grep sweep only probabilistically catches the rest. Assign items 6/7 explicitly (T11 is the
natural home; the parent-KAT amendment belongs in T4).

### M-3 (self-review) — four §6 KATs have no named plan test; the "Every §6 KAT … maps. No gap found" claim overreaches

(a) "STILL fires both record-time refusal directions" for a PROMOTED tranche (T3 pins only the backstop);
(b) "a relocated promoted tranche keeps the tag + the floor" (T4's relocated KAT is disposal-shaped; tag+floor
post-relocation is un-asserted — note relocation preserves `origin_event_id`, fold.rs:801-806, so the keying
holds, but the KAT is mandated); (c) "`safe_harbor_residue` does not project a dangling promote" (T12 adds the
filter with no test); (d) provenance refusal "incl. a mined/earned/airdrop/fork filer" (T10 tests Gift only).
Add the four named tests to their owning tasks.

### M-4 (T3/T7) — SPEC census item 11's `build_op` explicitness is unassigned

Item 11: `build_op`'s `_ => Op::Skip` "happens to do the right thing for a promote but SILENTLY … make it
explicit". T7 makes the void classification explicit; no task adds the explicit `PromoteTranche` arm/comment in
`build_op` (a `PromoteTranche` decision never reaches it — step 2's `_ => continue` skips decisions — which is
exactly why the census wants it documented). One-line arm + comment in T3 step 3.

---

## Nit

### N-1 (T12) — `format!("… {} …", p.target)` won't compile: `EventId` has no `Display`

Use `p.target.canonical()` (identity.rs:86) — the same fn's existing arms do
(`stp.in_event.canonical()`, main.rs:2153-2155).

### N-2 (T5/T6) — path/line drift in two citations

`Consumed` lives at `crates/btctax-core/src/project/pools.rs:291-307` (plan says bare "pools.rs");
`crypto_charitable_gifts` is at return_1040.rs:524 (plan: :535). Symbols exist and behave as claimed; harmless.

---

## Verified-good (for the re-review's benefit; no action)

- T3's rewrite site/shape: admit branch resolve.rs:1087-1114; `Op::Acquire(Acquire)` tuple with
  `usd_cost`/`basis_source` (fold.rs:23-24, mirrored by `overpayment_delta_one`'s swap conservative.rs:306-310);
  `voided: BTreeSet<EventId>` :461. Keying by `lot_id.origin_event_id` survives relocation (fold.rs:801-806
  clones the origin id, bumps only `split_sequence`) and the fee/removal paths.
- T5's TreatmentB claim is true: the mini-disposition routes through `make_disposal_legs` (fold.rs:362), so
  T4's clamp applies by construction; T5's single-site evaporation at the TreatmentC summation (:348-357)
  correctly makes SPEC 8b's three re-home sites inherit.
- T6's single-builder decomposition site (`basis: c.gain_basis`, fold.rs:256) and the consumer census
  (forms.rs:154/:213/8283 path, `claimed_deduction` fold.rs:1231-1276, `crypto_charitable_gifts` → `apply_170b`
  (pub, tax/charitable.rs:106)) all check out; the second-emitter KAT is buildable from integration tests.
- T7's void.rs targets (`is_revocable_payload` :20-35 — `DeclareTranche` currently unconditionally revocable;
  `effective_alloc` closure :74-84), `would_conflict` project/mod.rs:107.
- T10's harness/precedents: `declare_tranche` tranche.rs:123, clap variant cli.rs:886, dispatch main.rs:1162,
  `append_decision` persistence.rs:238, `ATTEST_PHRASE`/`require_attestation` lib.rs:197/:208,
  `pp()`/`now()`/`count()` in declare_tranche_cli.rs, `export_irs_pdf` signature matches T14's calls.
- T12's sites: catch-alls main.rs:2171 (`other =>`), tui-edit :3844 (`_ => ("?", …)` with the 4-tuple),
  `bulk_resolve_payload_summary` false-lead :2083, `safe_harbor_residue` drop-filter session.rs:~705-717;
  inline `mod tests` precedents exist in both binaries.
- Phase 1b hooks: `PrintedForms` packet.rs:421 / `assemble_printed_return` :461; the no-`..` destructure
  btctax-forms/packet.rs:36-57 IS compile-forcing; `CENSUS_KEYS: [&str; 14]` census.rs:29 +
  `census_key_set_is_exactly_14` :91 + the J6 manifest parse :125-135; sp3.rs
  `map_2024_matches_bundled_pdf_fieldset` :448; the fill pipeline symbols all exist
  (`fill_one` form8283.rs:323, `push_free` :303, `FlatPlacement::free` verify.rs:313, `verify_flat` :337,
  `drop_xfa_and_set_needappearances`/`apply_writes`/`strip_nondeterminism` pdf.rs:269/:378/:407,
  `push_identity` cells.rs:126); sp2.rs fault-injection + byte-determinism precedents exist.
- Makefile `check`/`docs`/`examples` targets exist; LIMITATIONS.md:63 and xtask docs.rs:184/:219 are the right
  doc sites; render.rs `write_basis_methodology_txt` call sites :871/:911 exact.
- Task order is otherwise sound: T1→T2→T3→(T4,T5,T6)→T7→T8→T9→T10→(T11,T12)→T13→T14 ‖ T15→T16; no task
  consumes a later task's output (modulo I-1's unowned type).
