# T4b — the year-N+1 opener: implementation report

Brief: `design/agent-reports/BRIEF-build-interview-T4b.md`. Contract: `SPEC_interview.md` r2 **R10
part 4**, **§7 row T4b**, **R11/T4**, **R10 parts 1–3 / T1**, **§6 J-27**.
Branch `main`, shared tree, dispatch HEAD `68467dde`. **Nothing committed, nothing pushed.**

Status: **all four numbered deliverables landed.** Full validation green (suite table at the end),
`cargo fmt --all -- --check` clean, `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace
--all-targets --all-features -- -D warnings` clean. 17 kills, each watched RED on a planted defect
(red text quoted below).

---

## 1. `open_next_year` in `btctax-cli` (a module of its own)

**`crates/btctax-cli/src/open_next_year.rs`** (490 lines), signature

```rust
pub fn open_next_year(sess: &mut Session, from: i32, discard_draft: bool) -> Result<Opened, CliError>
```

`Opened { from, to, identities: Vec<Identity>, carried: Vec<String>, not_stamped: Option<String>,
not_carried: Option<String> }`.

**The seed is built by copying IN, never by blanking OUT.** `seed(prior, to)` starts from
`ReturnInputs::default()` and copies only the identity fields; everything else — every money box,
every `PerYear` gate, the whole census, the whole `answer_log` — is default *because nothing wrote
it*. The two directions are not the same mistake: a copy-and-blank forgets a field by leaving year
N's answer in it and nothing reds; a copy-in forgets a field by leaving it blank, which the census,
the registry and `screen_inputs` already refuse. This is the design decision the whole item rests on.

Carried: `filing_status`; taxpayer/spouse `first_name`/`last_name`/`ssn`/`date_of_birth`; the mailing
address; each dependent's `name`/`ssn`/`relationship`/`date_of_birth`; each W-2 employer + EIN; each
1099-INT/DIV/G/B payer + TIN; each Form 1099-DA venue key with **both cohort slots unanswered**.
Deliberately NOT carried: the **IP PIN** (the IRS issues a new one each filing season, so last year's
is not merely unconfirmed but wrong), `occupation`, every header tri-state, the document census,
Schedule A/C/1/1-A, payments, and every dollar but the carryforwards.

**The carryforwards come off year N's FROZEN RETURN, through shared code.** `write_back_carryover`
was split: everything from `load_events_and_project` through `apply_carryover_writeback` is now
`cmd::tax::roll_carryover_onto(&Session, year, destination: impl FnOnce(&Session) -> Result<…>,
force)`, returning `RolledCarryover { updated, ar, ri }`. The opener passes its seed as the
destination; `write_back_carryover` passes the closure that fetches year+1's committed row **at the
point it always did**, so the ORDER in which its refusals fire is byte-identical (that is why the
destination is a closure and not a value). Every gate the write-back has — no package for the year,
non-`ReturnInputs` provenance, a pseudo-tainted ledger, a `NotComputable` ledger, `screen_absolute`,
the Reg §1.170A-7 restriction refusal, the §170(f)(8) CWA refusal, `capital_loss_roll_is_grounded` —
therefore binds the opener too, with one definition.

`restamp_from_prior_return` then rewrites every `CarryProvenance::Computed` the write-back stamped as
`ComputedFromPriorReturn { year: N }`. An **ungrounded** capital-loss roll is never stamped by the
write-back and so is never re-stamped: `User` on a zero still means *never asked*, which is what keeps
`BenefitCarryoversNotStated` honest.

**Refusals, nothing written:** year N+1 holds a parked draft, or a WIP draft holding work without
`--discard-draft` (T4's rule, T4's flag); year N has no committed row; year N+1 already has a
committed row. The draft-coherence check runs FIRST (T4's M-1 reason: a parked year has no committed
row, so the other two messages would shadow its remedy) and DELETES NOTHING — `coherence_clear` runs
next to the write.

**The write is `save_draft`, never `return_inputs::set`** — `return_inputs::get(N+1)` still answers
`None` after a successful open (R11).

**When year N's chain cannot be computed, the identities still seed and NOTHING is stamped**, with
`Opened.not_carried` naming why. Stamping `{0,0}` + `ComputedFromPriorReturn` there would silence
next year's advisory about a carryover the filer may genuinely have — the r3 I-4 damage, one year
over.

## 2. Each identity as its own tri-state prompt — and the model I chose

**Chosen: the prompt list is per IDENTITY; the ANSWER is the kind's census row.** The brief asked for
this to be decided and recorded, so:

- A new `FormQuestion` per row is not available: `FORM_QUESTIONS` is a `&'static` array and cannot
  grow an entry per employer. Making it dynamic needs a new `AnswerKey` kind and therefore a second
  path into `answer_log`, which R10.3 forbids ("`record_answer` is the one writer").
- So `Opened.identities` carries one `Identity { census_row, prompt, answer }` per identity, each
  with R10.4's sentence in the form's words — *"Last year Acme Tooling (EIN 12-3456789) issued you a
  Form W-2. Did Acme Tooling issue one for 2025?"* — and the answer is `documents.<row>`, which is
  `None` on the seed for every one of them.
- **A dependent's SSN is never printed** (the prompt is name + relationship). `income show` masks a
  dependent's SSN; the payer identifiers it does print are a business's EIN/TIN, which `income show`
  does not mask and which R10.4's own example sentence prints.

**"A No removes the pre-named row" required a mechanism, and it is one definition.** Without it the
opener manufactures a brick: it pre-names rows the filer never typed, the filer truthfully answers
*"no W-2 this year"*, and `screen_inputs` then refuses the year for a contradiction (`Some(false)`
beside transcribed rows) that `income answer` offers no way to clear. So:

- new in `btctax-core/src/tax/document_census.rs`: `row_is_pre_named`, `drop_pre_named_rows`,
  **`answer_row(ri, row, v)`**;
- **all eighteen** census `FormQuestion::set` closures now delegate to `answer_row`, so `income
  answer`, the input form and any future surface share the rule;
- "pre-named" is a **comparison against the row's own identity-only seed**, exactly like
  `input_form_store::draft_is_disposable` — never a list of boxes to check for zero. A box added to
  `W2` tomorrow is covered the day it is added, and the direction is fail-CLOSED: anything the filer
  typed (a figure, or only a `transcribed_on` date) makes the row unequal to its seed, so it is KEPT
  and the contradiction refusal still fires. **Nothing this removes was ever testimony.**

## 3. The TUI entry action

- `editor.rs`: `EditorApp.open_next_year: Option<OpenNextYearState { from, to, blocked_by_draft, done }>`.
- `main.rs`: Browse key **`n`**; `open_open_next_year` (the offer), `run_open_next_year`,
  `handle_open_next_year_key`, dispatched with the flows (modals → flows → this → Browse) so `q`/Esc
  never fall through to a quit arm.
- `edit/persist.rs`: `form_open_next_year_offered(session, to)` and `form_open_next_year(session,
  from, discard_draft)` — the KAT-G1 gate confines `Session::conn()`/`save()` to that module, and
  `kat_g1_mechanized_source_gate` passes.
- The offer predicate is the brief's: year N+1 has **no committed row and no draft**, and year N
  **has** a committed row. Pressing `n` elsewhere says which of the three facts is false rather than
  doing nothing silently.
- `draw_edit.rs`: `draw_open_next_year` — three states on one surface. The OFFER says *"every box
  arrives blank and every question unanswered: last year's answer is not testimony for this year"*
  BEFORE the filer presses anything, because the surprise this surface could produce is a filer
  expecting last year's return to be copied. The PAYLOAD-CONFIRM names what the draft holds and takes
  `X`. The REPORT is the identity questions.
- Help overlay gained the `n` line; `kat_keymap_overlay_lists_every_browse_char_binding` holds it.

## 4. Retention untouched

Year N is read and never written; nothing is deleted from it; no shred (§G-14 is not built here).
Held by `nothing_about_year_n_changes`, which compares the whole stored row before and after.

---

## Deviations from the brief

1. **Signature.** `open_next_year(sess, from, discard_draft)` — the brief's own refusal list requires
   the `--discard-draft` flag, which the two-argument signature it quotes cannot carry.
2. **The household header is seeded**, which the brief's explicit list (payers, dependents, venues)
   does not mention. R10.4's *"every `Durable` fact (a DOB) shown"* is unsatisfiable otherwise — the
   DOBs live on `header.taxpayer`/`header.spouse` — so name, SSN, DOB and the mailing address cross
   as identity while every header tri-state stays `None`.
3. **`filing_status` is carried.** The field has no `None`, so *not* carrying it asserts **Single** —
   a fabricated answer, and the wrong one for every MFJ filer. Carried as the starting point the
   year's own form asks the filer to confirm; `draft_is_disposable` already treats *"a tax year and a
   filing status"* as opening a year rather than as work. It writes no `AnswerRecord`.
4. **`Opened.not_stamped`** was added beyond the brief: T9's rule (and `write_back_carryover`'s own
   B-1) is that the carryover which is NOT written is **named**, never merely omitted, or the filer
   reads four carryovers into a list of three. Found by walking the journey with the real binary.
5. **The `usage: ` prefix is stripped** from an embedded chain refusal — inside the opener's sentence
   it reads as though the filer mistyped the command.
6. **`Dependent { .. }` in the seed names every field with no `..Default::default()` tail**, so T7's
   seventeen §152 gates make this literal fail to COMPILE and a human decides identity-vs-declaration
   for each. (clippy's `needless_update` forced the choice; this is the better half of it.)
7. **Test scaffolding promoted to the library.** `provenance::leaf_walk` (`walk`, `set_at`,
   `money_leaves`, plus new `at` and `nonzero_money_leaves`) moved out of `#[cfg(test)]` into a
   `#[doc(hidden)] pub mod`, for the same reason `tax::testonly` is a plain `pub mod`: T4b's kill
   needs T1's type-driven money detector from another crate, and a second copy would be a second
   thing to keep true. The existing `LEAF_SOURCE` KAT now imports it and is unchanged.

## Kills — every one watched RED on a planted defect

All plants applied to a `cp` backup and reverted from it; the tree was verified clean and green
after each. Full transcripts in the session scratchpad; the discriminating line of each:

| # | kill | planted defect | red |
|---|---|---|---|
| 1 | `two_payers_yield_exactly_two_identity_prompts_both_unanswered_and_every_box_default` (R10.4 verbatim) | no document identity is prompted for | ``assertion `left == right` failed: two payers, two prompts: []`` |
| 2 | `no_money_leaf_of_the_seed_is_nonzero_except_the_carryforwards` | the seed carries `payments` forward | `payments.estimated_tax_payments = 1500 crossed the year boundary and is not a carryforward — R10.4: NEVER a carried amount.` |
| 3 | same, provenance half | the re-stamp removed | ``assertion `left == right` failed: and it says which RETURN it came off`` |
| 4 | `nothing_in_the_seed_is_stamped_merely_computed` | the re-stamp removed | ``these carried figures are stamped `Computed` … instead of `ComputedFromPriorReturn`: [".capital_loss_carryforward_in_provenance", ".charitable_carryover_…]`` |
| 5 | `every_per_year_gate_on_the_seed_is_unanswered` | `foreign_accounts` carried | ``ForeignAccounts carries an answer nobody gave this year (every DECLARATION is PerYear)`` |
| 6 | `a_durable_fact_is_shown_but_unanswered_until_the_filer_confirms_it` (record half) | `answer_log` carried | ``and it is NOT answered: the opener carries no record at all — {Skippable(DobTaxpayer): AnswerRecord { answered_on: 2025-03-01, … }}`` |
| 7 | same (shown half) | the DOB not seeded | ``assertion `left == right` failed: the durable fact is SHOWN`` |
| 8 | `the_carryforward_is_read_off_the_frozen_return_and_not_off_year_ns_inputs` | the seed takes year N's **inputs** | ``the seed takes the RETURN's carryover-OUT, which absorbed the §1211(b) $3,000`` |
| 9 | `the_open_writes_a_draft_and_no_committed_row` | `return_inputs::set` instead of `save_draft` | ``the opener never calls `return_inputs::set` `` |
| 10 | `a_committed_row_on_the_year_being_opened_refuses_and_writes_nothing` | the refusal removed | ``2025 already exists: Opened { from: 2024, to: 2025, … }`` |
| 11 | `a_year_n_with_no_committed_row_refuses` | the refusal made unreachable | ``there is nothing to open from: Opened { from: 2024, to: 2025, identities: [], carried: [], … }`` |
| 12 | `a_non_trivial_draft_on_the_year_being_opened_refuses_and_survives` | `discard_draft` forced true | ``a draft holding an interview is not superseded on a note: Opened { … }`` |
| 13 | `a_parked_draft_on_the_year_being_opened_refuses_even_with_discard_draft` | the whole coherence check removed | ``a parked return is never seeded over: Opened { … }`` |
| 14 | `dependents_and_venues_are_prompts_too_and_no_ssn_is_printed` | the venue key not seeded | ``assertion `left == right` failed: the venue key is seeded`` |
| 15 | `a_capital_loss_roll_the_gate_skipped_is_named_and_not_silently_omitted` | the skip silently omitted | ``and the skip is NAMED, not omitted`` |
| 16 | `the_open_next_year_action_is_absent_when_year_n_has_no_committed_row` (TUI) | the offer stops requiring year N | ``no year 2024 return ⇒ no confirmation opens`` |
| 17 | `kat_keymap_overlay_lists_every_browse_char_binding` | the `n` help line removed | ``these Browse keys are bound in main.rs but absent from the KEYMAP overlay … add a line for each: ['n']`` |

Plus, in core: `answering_a_census_row_no_drops_the_pre_named_rows_and_keeps_a_transcribed_one` and
`every_kind_with_a_section_drops_its_pre_named_rows` both red on removing the drop from `answer_row`
(``left: ["Acme", "Beta"] / right: ["Beta"]``).

### The false PASS this found, and the guard that closes it

The first version of kill #2 **passed under its own plant**: the year-N fixture's `payments` were all
zero, so copying them forward produced no non-zero leaf. The fixture was rebuilt to carry money in
structurally different places (W-2 boxes, two interest payers, a dividend payer, a 1099-G, a Schedule
A, Schedule 1, payments, a carried loss) and the test now opens with a **vacuity guard**:

> the year-N fixture must realize **≥ 20** non-zero money leaves, else *"the seed carries none of
> them"* means nothing.

**Measured: 21** at the time of writing (floor set just below it, the `money.len() >= 100`
discipline). It also asserts `opened.not_carried == None` — a premise, so the carryforward half
cannot pass on an opener that carries nothing.

## Pinned numbers moved

| number | old → new | cause |
|---|---|---|
| year-N fixture non-zero money-leaf floor | *(new)* → `>= 20`, measured **21** | the vacuity guard above |
| `docs/man/btctax-income-open-next-year.1` | *(new page)* | `docs::tests::manpage_covers_every_subcommand` red on the new subcommand; regenerated with `cargo run -p xtask -- docs`, which also added 3 lines to `docs/man/btctax-income.1` |

No existing pinned number changed. `write_back_carryover`'s behaviour is unchanged and its tests are
green untouched (the extraction preserved refusal ORDER by threading the destination as a closure).

## Journey walk with the real binary

Built `btctax` and drove a scratch vault: `init` → `income import --year 2024` → `income
open-next-year --from 2024`. Four things the tests did not show:

1. On an **unanswered** year N the opener still seeds the identities and reports *"★ NOT CARRIED …"*
   with year N's own refusal and its remedy. Correct, and the reason the chain failure is not a
   refusal.
2. **Re-running the opener refuses**, naming the seeded draft's own pre-named rows (*"1 Form W-2
   row(s), 1 Form 1099-INT row(s)"*). Conservative and correct — `--discard-draft` is the exit — but
   see the follow-up below.
3. A blanket *"no"* pass over year N's census left `documents.w2 = Some(false)` beside a
   **transcribed** W-2 and the chain refused for the contradiction: the fail-closed half of
   `row_is_pre_named` working in the field.
4. After answering year N truthfully, the carried line prints and `income show --year 2025` still
   reports no committed row while `income answer --year 2025` answers the draft.

Deliverable 4 came out of step 2 of this walk.

## Candidate follow-ups (filed nowhere — the controller's call)

- **FR-?? (owning phase: the interview build).** A draft that is exactly a fresh *opener seed* is
  reported to the filer as work (*"1 Form W-2 row(s)…"*) when they re-run the opener. It is
  truthful — those rows exist — and refusing is the safe direction, but a filer who opened the year
  by accident must pass `--discard-draft` to fix it. The honest widening would be a predicate for
  *"equal to the seed `open_next_year` would produce for this year"*, which is a comparison and not a
  list; deliberately not attempted here (widening a disposability exemption is never the safe edit).
- **FR-?? (owning phase: T7).** The per-identity prompt for a **dependent** has no answer surface
  today (`Identity.answer` is `None` by construction, and the seeded row is simply pending). It
  becomes real when T7 lands `DEPENDENT_GATES`; the `Dependent` literal in `seed` will fail to
  compile then, which is the intended forcing function.

## Suite lines (whole validation surface, one run each)

| crate | result |
|---|---|
| btctax-core | 1260 tests run: **1260 passed**, 0 skipped |
| btctax-cli | 768 tests run: **768 passed**, 1 skipped |
| btctax-tui-edit | 387 tests run: **387 passed**, 2 skipped |
| btctax-input-form | 68 tests run: **68 passed**, 0 skipped |
| btctax-tui | 160 tests run: **160 passed**, 2 skipped |
| btctax-forms | 354 tests run: **354 passed**, 4 skipped |
| btctax-adapters | 103 tests run: **103 passed**, 0 skipped |
| btctax-store | 45 tests run: **45 passed**, 0 skipped |
| btctax-oracle-harness | 5 tests run: **5 passed**, 1 skipped |
| xtask | 154 tests run: **154 passed**, 1 skipped |
| btctax-update-prices | 5 tests run: **5 passed**, 1 skipped |

`cargo fmt --all -- --check` clean. `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace
--all-targets --all-features -- -D warnings` clean.

## Files touched

New: `crates/btctax-cli/src/open_next_year.rs` (490), `crates/btctax-cli/tests/open_next_year_t4b.rs`
(896), `docs/man/btctax-income-open-next-year.1`.
Modified: `crates/btctax-cli/{cli.rs, lib.rs, main.rs, cmd/tax.rs}`,
`crates/btctax-core/src/tax/{document_census.rs, provenance.rs, questions.rs}`,
`crates/btctax-tui-edit/src/{editor.rs, main.rs, draw_edit.rs, edit/persist.rs}`,
`docs/man/btctax-income.1`.
(`CONTINUITY.md`, `design/ROADMAP_STATUS.md` and the other `design/agent-reports/` files in `git
status` are not mine.)
