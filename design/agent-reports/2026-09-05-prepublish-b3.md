# Pre-publish B3 review — v0.18.0, `origin/main..HEAD` (17 commits, 50 files)

**Reviewer:** independent agent (Fable 5.1), read-only, no subagents. **Date:** 2026-09-05.
**Brief:** the last review before `cargo publish`; scope is the WHOLE unpushed range and the target is
INTERACTION between six parallel-worktree changes merged serially. Individual reports were assumed
honest and not re-audited.

---

## VERDICT

**0 Critical / 1 Important / 3 Minor / 2 Nit — DO NOT PUBLISH** until I-1 is folded and `make check`
re-run.

I-1 is **not an understatement path**: on every input I probed, the filed number moves in the
conservative direction (the reg's own remedy). It blocks on the repo's own severity table — it is a
false claim made by two commands (`verify`, `optimize`) in violation of a rule the SPEC marks
load-bearing (*"no artifact, command, or doc may describe post-hoc selection as compliant"*,
`design/SPEC_lot_optimization_program.md:218`), and a source-level guarantee ("can never diverge")
that the range itself made false. The fix is small (three sites, one predicate) and machine-checkable.
The Minors and Nits do not block and should be filed as follow-ups.

---

## FINDINGS

### I-1 — FR-35 narrowed the timeliness predicate at ONE of its THREE sites; `verify` and `optimize` now call a rejected post-hoc selection `Contemporaneous`

**Where the three copies live**

| site | predicate | granularity after this range |
|---|---|---|
| `crates/btctax-core/src/project/resolve.rs` ~1541-1650 (FR-33 pass, narrowed by FR-35) | `d.utc_timestamp <= e.utc` | **instant** (`OffsetDateTime`) |
| `crates/btctax-core/src/project/compliance.rs:211-216` → `identification_made(date, sel_made, …)` | `tax_date(e.utc_timestamp) <= disposed_at` | **date** (`TaxDate`) |
| `crates/btctax-core/src/optimize.rs:475-483` `persistability` and `:499-518` `proposed_compliance_status` | `selection_made <= sale_date` | **date** (`TaxDate`) |

Before this range all three were date-granular and therefore agreed. FR-35 (`a7ecb7b1`, 09:22, the
controller's solo commit) moved only the first. FR-34 (`6beb1c28`, 09:52, merged 09:53) then made
`compliance::identification_made` the SHARED classifier and wrote into its doc comment: *"One function
means the reported verdict and the filed number can never diverge."* That sentence was true in FR-34's
worktree, which was branched before FR-35 landed, and is false at HEAD.

**The two callers hand the shared function different facts.** The fold passes
`selection.map(|_| date)` over `ctx.selections` — the set AFTER resolve's timeliness `retain`, so a
present selection is timely-or-attested by construction. `disposal_compliance` builds `sel_made` from
EVERY non-voided `LotSelection` decision (compliance.rs:178-195), unfiltered by any §A.4 rejection,
at `TaxDate` granularity. The doc on `sel_made` says "APPLIED to this disposal"; the code says
"recorded against this disposal".

**Machine-checked, not inferred.** Transient probe (written, run under nextest, deleted; tree clean),
fixture from `tests/lot_selection.rs`: three post-2025 lots A($50, Feb) B($90, Mar) C($40, Apr), sale
of 100k sat at **10:00** on 2025-07-01, UNATTESTED `LotSelection` for B recorded at **17:00** the same
day.

```
same-day-LATE, FIFO election : basis_used=50.00  blockers=[LotSelectionPostHoc]                          compliance=[Contemporaneous]
same-day-LATE, no election   : basis_used=90.00  blockers=[LotSelectionPostHoc, IdentificationDefaulted] compliance=[Contemporaneous]
same-day-EARLY (09:00), elect: basis_used=90.00  blockers=[]                                             compliance=[Contemporaneous]   (control — correct)
next-day-LATE, FIFO election : basis_used=50.00  blockers=[LotSelectionPostHoc]                          compliance=[NonCompliant]      (control — agrees)
next-day-LATE, no election   : basis_used=90.00  blockers=[LotSelectionPostHoc, IdentificationDefaulted] compliance=[NonCompliant]      (control — agrees)
```

Rows 1-2 are the defect. The fold rejected the selection as post-hoc and consumed by the method in
force (row 1) or the default (row 2), the blocker text says the selection *"is IGNORED"*, and in the
same `verify` output the §A.5 row for that disposal reads **`contemporaneous`**
(`crates/btctax-cli/src/render.rs:703` → `:237`). Row 2 is the sharpest contradiction: the report
simultaneously says "identified contemporaneously" and "identified by NOTHING — default applied".
Rows 4-5 show the divergence is exactly the window FR-35 opened; a day later the three sites agree.

**`optimize` inherits it.** `optimize.rs:884` takes `disposal_compliance` as `baseline_status`;
when the proposal equals the baseline legs (which, after the drop, are the method-order legs),
`proposed_compliance_status` returns that baseline verbatim (`:507-509`) → the proposal row prints
`Contemporaneous`. Separately, `persistability(wallet, sale_date: TaxDate, made: TaxDate)` tells a
filer who sells in the morning and runs `optimize` that afternoon *"persistable now (made ≤ sale →
Contemporaneous)"* (`render.rs:2361`); an unattested `optimize accept` then persists at `now()`, and
the very next projection drops it as `LotSelectionPostHoc`. The tool's claim and its next action
disagree. (`optimize accept --attest` is unaffected — attested selections bypass the retain.)

**Why only a whole-range reader could see it.** The FR-34 report ITSELF wrote the warning
(`design/agent-reports/2026-09-05-fr34-fix.md:239-241`): *"filed for whoever owns FR-35: this fix hands
`identification_made` a `TaxDate`, so when FR-35 moves timeliness to date-and-time it will need to
widen there and in FR-33's `resolve.rs` pass together — they are now one predicate."* FR-35 was
already on `main` thirty minutes before that sentence was merged, and nobody held both commits at
once. This is the `b94508d` shape from the harness doc: the fix's location was known inside the
branch; the field of view was the failure. (The FR-34 note also undercounts: it is one predicate at
three sites, not two — `optimize.rs` is the third.)

**Direction and severity.** The filed number is conservative on every probed input — the reg's
remedy raises gain. No understatement path. It is Important because (a) `verify` and `optimize` make a
false compliance claim about a post-hoc selection, which the SPEC forbids as a load-bearing rule;
(b) a source-level guarantee introduced in this range ("can never diverge") is false at HEAD; (c) the
`ComplianceStatus::Contemporaneous` variant doc (compliance.rs:16 — "on or before the DAY of sale") and
the `identification_made` doc now describe a standard the code applies at only one of three sites.
It is not Critical because no figure on a filed form is wrong and the filer sees the contradicting
advisory in the same output.

**Fix shape (not prescribing the edit).** Make the ONE predicate FR-34 intended: `identification_made`
takes `OffsetDateTime` for both the deadline and the made-instant (or takes the resolve pass's applied
selection set), `disposal_compliance` reads the disposal's instant rather than `disposed_at`, and
`optimize::persistability` / `proposed_compliance_status` compare instants. B1: one planted test per
site at the same-day-late window (10:00 sale, 17:00 selection) asserting `NonCompliant`/`StandingOrder`
rather than `Contemporaneous`; the existing
`contemporaneous_status_when_selection_made_before_sale` (compliance.rs:175, equal instants 00:00/00:00)
is consistent under both granularities and needs no change.

### M-1 — `IdentificationDefaulted` detail text is wrong in the double-advisory case

`fold.rs` composes *"no specific identification was made for this disposition — no LotSelection covers
it and no MethodElection is in force"*. When FR-33/35 has dropped a late selection (probe row 2), a
`LotSelection` DOES cover it and was rejected; the sentence should say so. Behaviour is intended and
pinned by FR-34's own seam test (`a_dropped_post_hoc_selection_with_no_election_raises_both_advisories`);
only the wording is off. Does not block.

### M-2 — FR-38 refusal in a batch import names the field and value but not the row

`CoreError::ImpossibleValue { field, value, reason }` has no `source_ref`. `append_import_batch` is
atomic (`unchecked_transaction`, by design FR1), so one negative cell rolls back the whole CSV with
`refused: Acquire.usd_cost = -12.34 — …` and no row reference. Fail-closed and correct in direction;
the filer of a 5,000-row export has nowhere to look. No CLI/TUI site matches on `ImpossibleValue`
(grep: zero hits outside core). Does not block.

### M-3 — FR-35 makes every same-day selection late for a bare-date source timestamp

`crates/btctax-adapters/src/parse.rs:140-143` maps a bare `YYYY-MM-DD` to **UTC midnight**. A sale
imported that way has deadline 00:00:00, so an identification genuinely recorded before the real
sale time but after midnight is dropped as post-hoc. Direction: conservative (raises gain), Advisory,
and the attest path exists. The FR-35 commit message does not mention it. Documentation-only;
does not block.

### Nit-1 — `tables.rs` `MARGIN_COLUMN` doc claims a pin that is not in that file

The doc (tables.rs ~1387) says the reader's output *"is asserted to be exactly the 50 labels that
xtask's two geometry witnesses derive"*. `spans()` is reached only through `printed_line`
(tables.rs:1407, :1451); nothing compares its key set to 50. The 50-label assertion lives in
`crates/xtask/src/schedule_1a_membership.rs:137` against `label_reader::witness_text`, a DIFFERENT
reader. The `tables.rs` reader is still B1-tested (the rounding-swap plant and the line-28 reader
guard), so this is a doc overclaim, not a blind instrument.

### Nit-2 — two doc comments made false by this range

`identification_made` ("can never diverge", compliance.rs) and `ComplianceStatus::Contemporaneous`
("on or before the day of sale", compliance.rs:16). Both fall out of the I-1 fold.

---

## THE SEAMS — what only a whole-range reader could see

1. **`resolve.rs` ordering (FR-31 / FR-33 / FR-35) — HOLDS.** FR-31's `SelfTransferDoubleBooked`
   guard is pass 1e-FR31 (resolve.rs:1123-1215), well before §A.4 collection (2c, ~1500). Inside 2c
   the order is: dup → `DecisionConflict` (Hard); targeting + principal conservation →
   `LotSelectionInvalid` (Hard); timeliness `retain` LAST → `LotSelectionPostHoc` (Advisory). FR-35
   changed the TYPES compared (`TaxDate` → `OffsetDateTime`) and moved nothing; the "Advisory must
   never pre-empt a Hard" property is intact. Hard/Advisory classification confirmed at
   `state.rs:130-154`. Both timestamps are true instants: `Eff::utc` is the import event's
   `utc_timestamp` (resolve.rs:1462), decisions' `utc_timestamp` is creation time (event.rs:482).
   `OffsetDateTime` ordering is instant-wise across offsets.

2. **identification / compliance (FR-33 + FR-34 + FR-35) — BROKEN, see I-1.** The whole-range fact is
   that FR-34 built a "shared" classifier around a granularity FR-35 had already changed thirty minutes
   earlier, and FR-34's report said so in a note addressed to a change that was already merged. No
   double-counting of basis (the fold is the only writer of the number); the double-ADVISORY
   (`LotSelectionPostHoc` + `IdentificationDefaulted`) is intended and pinned, with the wording defect
   in M-1.

3. **`printed.rs` `Option<Usd>` conversions (FR-1 / FR-12 / FR-27 / FR-39) — CONSISTENT, no new
   None-as-zero.** Grepped every consumer of `Form1040Lines::line20` and `Form8960Lines::line9d`: the
   only `unwrap_or(ZERO)` sites are the two `Combine` totals (1040 L21 at printed.rs:804, 8960 L11 at
   other_taxes.rs:397) that FR-39 already files as one open question, plus tests. Both `Option`s reach
   the emitter as `Option` (`form1040_full.rs:177`, `form8960.rs:58` → `push_money_opt`), the
   map.toml comments were updated, `attestation.rs:620/1047` assert absence on paper, and
   `line_coverage.rs` re-registers both as blank-capable (`line20: None`, `line9d: None`;
   `Production::Carry`). L22 = L18 − L21 and L12 = L8 − L11 are unaffected by whether the total prints
   0 or blank. Nothing here contradicts a neighbouring blank beyond the FR-39 pair, which is filed and
   owner-owned.

4. **`persistence::insert` (FR-38) under everything else — HOLDS.** Checked every value class the
   other five changes can create: Schedule 1-A creates no events; FR-34/FR-35 create none; FR-37 emits
   only an advisory (`reconcile.rs`, stderr) on POSITIVE oversized values, which the gate accepts;
   `-0` is explicitly accepted (`negative_zero_is_not_a_negative_amount`); the four `ConsentTerm` deltas
   are whitelisted and `Unrealized.hypothetical_reduction` is a clamped `min(...)` ≥ 0
   (`conservative_promote.rs:246-248`). `make check` green at HEAD is the exhaustive witness that no
   committed fixture is refused. The seam is single (`insert` is the only `INSERT INTO events`), the
   `ImportConflict` arm recurses into `new_payload`, and `load_all` is untouched (forward-only, as
   documented). Residue is M-2 only.

5. **`tables.rs::printed_line` rewrite — no other test's meaning changed.** Old reader: first
   physical line whose first word equals the label, then append until a digit-led or blank line. New
   reader: margin-column span map. All pre-existing callers (`each_phase_out_rounds_the_way_its_own_
   printed_line_says_to` for 11/19/28, `the_caps_that_do_not_vary_by_status_print_no_variant` for
   7/15/24, the line-28 reader guard) assert LITERAL text of the returned span, so a semantic drift
   on those labels would have reddened them; they are green at HEAD. The one place the new reader is
   STRICTER — excluding body-text tokens like `"4b and see the instructions…"` at column 10 — is the
   direction that makes the T2 quotation check harder to pass, not easier. Residue is Nit-1.

**FR-35 as the least-scrutinised change.** Read in full. Types and comparison are correct; the
eligibility test stays date-granular per §1.1012-1(j)(6) as stated; self-transfers stay excluded
(unchanged scope); the TUI KAT pin move (12:00 → 11:00 against an 11:33:20 sale) preserves the KAT's
intent. The B1 mutation account in the commit message (truncate BOTH sides to midnight) is the correct
kill. Its defect is not inside the diff — it is the two sibling sites the diff did not touch (I-1),
plus M-3.

---

## WHAT I VERIFIED AND HOW

- `git log/diff origin/main..HEAD`: 17 commits, 50 files, +8354/−351 (matches brief).
- Full diffs read: `resolve.rs`, `pools.rs`, `state.rs`, `compliance.rs`, `fold.rs`, `printed.rs`,
  `return_1040.rs`, `other_taxes.rs`, `advisories.rs`, `tax/mod.rs`, `form1040_full.rs`, `form8960.rs`,
  both `map.toml`s, `persistence.rs`, `lib.rs`, full `payload_polarity.rs`, FR-35 commit `a7ecb7b1`
  in full (resolve/lot_selection/tui main), non-comment lines of `reconcile.rs` and `form.rs`.
- Surrounding un-diffed code read: resolve.rs 1440-1720 (passes 2b/2c/3), compliance.rs whole,
  optimize.rs 475-545 and 868-930, render.rs 690-740, persistence.rs 100-270, tables.rs old vs new
  `printed_line` and 1380-1560 / 1880-1990, line_coverage.rs registrations for line20/line9d/line21.
- **Probe executed** (transient `tests/zz_prepub_probe.rs`, `cargo nextest run -p btctax-core --test
  zz_prepub_probe --no-capture`, 1/1 PASS, file deleted, `git status` clean): five scenarios above.
- Greps: all readers of `disposal_compliance` (tests, `optimize.rs:884`, `render.rs:703`; none in
  forms); all consumers of `line20`/`line9d`; `ImpossibleValue` handlers (none outside core);
  `spans()` uses; `midnight`/bare-date in adapters; FR-39 filed (FOLLOWUPS.md:5716); FR-34 report
  §6 note (lines 239-241); SPEC rule text (`SPEC_lot_optimization_program.md:218`).
- Did NOT re-run `make check`, clippy, fmt, archive-check, authority-manifest, verdict-reach or
  label-census — taken as given per brief.

## WHAT I COULD NOT CHECK

- Any TUI flow live (select-lots with/without attestation after FR-35); reasoning from
  `persist.rs:982` and the KAT pin only.
- The emitted PDFs for the FR-12/FR-27 blank cells — relied on the `attestation.rs` paper-read tests.
- The individual B1 mutation claims in the six agent reports (out of scope per brief).
- Whether the owner regards a `verify`/`optimize` verdict label as an "artifact" under the SPEC
  cross-cutting rule — the SPEC's own words are "artifact, **command**, or doc", so I applied them.
- The Schedule 1-A plan and the 8615 spec review (out of scope).
