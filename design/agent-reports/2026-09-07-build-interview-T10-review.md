# Seam review — interview build T10 (the trailer), at `f8768e93`

Independent adversarial build review, own worktree
`/scratch/code/bitcoin_tax/.claude/worktrees/agent-a06f22a0e2c838c0d`, `main` @ `f8768e93`.
Every plant made here and reverted from a `cp` backup; `git status --porcelain` empty at exit;
nothing committed, nothing pushed, no subagents.

**Environment note — the stated exceptions did NOT apply.** Both `crates/btctax-forms/forms/2024/`
and `forms/2025/` carry their `f1040.pdf` in this worktree, and the whole of `btctax-forms` ran
green with `CARGO_TARGET_DIR` redirected (370 tests, 0 failed). So no emitter claim is marked
unverified: the read-back KATs really executed against the bundled templates.

---

## Commands

```
$ export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review

$ cargo nextest run --locked -p btctax-forms -E 'binary(field_census)'
     Summary [   0.227s] 5 tests run: 5 passed, 0 skipped

$ cargo nextest run --locked -p btctax-core -E 'test(/t10_bank_numbers|paper_check_advisory|advisory_does_not_name_a_cell/)'
     Summary [   0.005s] 7 tests run: 7 passed, 1348 skipped

$ cargo nextest run --locked -p btctax-forms \
    -E 'test(/direct_deposit_block|no_direct_deposit_leaves|spouse_ip_pin_prints|phone_number_prints|foreign_address_prints/)'
     Summary [   0.356s] 5 tests run: 5 passed, 369 skipped

$ cargo nextest run --locked -p btctax-input-form
     Summary [   0.091s] 73 tests run: 73 passed, 0 skipped

$ cargo nextest run --locked -p btctax-core -p btctax-input-form -p btctax-forms -p btctax-cli
     Summary [  12.801s] 2611 tests run: 2611 passed, 5 skipped
```

Kills run (each planted, observed red, reverted): **K1** ABA check-digit branch → `if false`;
**K3** `SpIpPin`'s `get` constructs `SecretView::Set` from the raw digits; **K4**
`ReturnHeader::build` reads `ri.header.spouse_ip_pin` instead of `spouse_ip_pin_if_live()`;
**K5** `screen_direct_deposit` moved *inside* `if tier.unanswered_refuses`; **K6**
`[direct_deposit]` deleted from the TY2024 map (cells re-censused so the census gate stays
consistent). Probes run (fixture/behaviour, not defect plants): **P-a** the T10 leaves added to
`every_leaf_the_seed_carries_is_named_in_the_report`'s fixture; **P-b** a test asserting the
behaviour `RoutingNumber::canonical`'s doc comment describes; **P-c** the seam's real
`create` + `set` + `screen_param_free` + `ReturnHeader::build` on an untouched line 35c.

**Verified independently, as the dispatch asked:**

- `UNCENSUSED_FIELDS = 285` / `UNCENSUSED_ENTRIES = 5` — **unchanged and correct**. All five
  register lines are TY2025 (`crates/btctax-forms/tests/field_census.rs:75-101`); T10 mapped only
  TY2024 cells, which live in that map's own `[census]`, not the register. All five register tests
  pass, including `the_uncensused_register_may_only_shrink`.
- **The nine retired TY2024 entries are exactly the nine cells T10 mapped — no more, no fewer.**
  From the map diff: `f1_15`, `f1_16`, `f1_17` (foreign row), `f2_25`, `c2_5[0]`, `c2_5[1]`,
  `f2_26` (35b–d), `f2_36` (spouse IP PIN), `f2_37` (phone). `c2_4` (the Form 8888 box) stays
  censused with only its `covered_by` changed; `f2_38` (email) stays. `census-join` 283 → 274 = −9,
  consistent.
- **Every citation the build report leans on re-measured at `f8768e93`:** `i1040gi--2025.txt:23967`
  = *"The routing number must be nine digits."*; `:23968-23969` = the 01–12 / 21–32 sentence;
  `:24054-24059` = the account-number sentence, verbatim; `f1040--2024.txt:22` = the foreign row,
  `:116` = *"Direct deposit? b Routing number c Type: Checking Savings"*, `:133-135` = the spouse
  IP PIN caption, `:137` = *"Phone no."*. All correct.
- **Brief drift:** the brief's `i1040gi--2025.txt:23963-24000` is a 37-line span whose *start* is
  right (`:23963` = "Line 35b") but which does not contain the line-35d account rule at all
  (`:24054-24059`). The build report already recorded this; noted as drift, not as a finding.

---

## Summary

The one question — *can a refund be routed, a secret be read back, or a header cell be printed from
anything but what the filer entered this year?* — comes back **no for the secret, no for the
routing number, and YES twice for a header cell**:

1. the opener carries the phone and the whole foreign-address row across the year boundary while
   the report that exists to bound its own *"Everything else is blank"* claim does not name them,
   and the guard for exactly that is blind because its fixture never populates the new leaves; and
2. line 35c's account-type box is printed from a value `create` wrote and nothing ever asks the
   filer to confirm.

A third Important is that the written justification for the one rule the IRS does not state (D-3)
describes a fail-safe degradation the code does not perform — it blocks the return instead.

The secret seam is sound and its sweep is real (enumerated from `form_spec()`, count pinned at 6,
reds in both directions). The validator is a faithful transcription plus one stated deviation. The
census bookkeeping is exact. Sixteen new tests, and every kill I planted reproduced red.

**Counts: C=0 I=3 M=4 N=1**

---

## Findings

### I-1 — the opener carries the phone and the foreign address, and its own "everything else is blank" claim does not name them

**Where.** `crates/btctax-cli/src/open_next_year.rs:411-434` (the four new `clone()`s in `seed`)
vs. `:288-319` (`CARRIED_IDENTITY`); guard at
`crates/btctax-cli/tests/open_next_year_t4b.rs:1370-1439`.

**What is wrong.** T10 added four carried leaves to `seed` — `header.phone`,
`header.foreign_country`, `header.foreign_province`, `header.foreign_postal_code` — and added no
phrase to `CARRIED_IDENTITY`. The address phrase's prefix is `header.address_`, which matches none
of them. So `Opened::render()` prints

> `Carried from TY2024 — CONFIRM each: the filing status …, your name and SSN, your mailing address,
> each employer and payer …`
> `Everything else is blank and every question is unanswered.`

while a foreign country, province, postal code and phone number have all crossed. That is the exact
I-1 defect `CARRIED_IDENTITY` and `every_leaf_the_seed_carries_is_named_in_the_report` were built
for — *"four surfaces asserted 'every box is blank and every question is unanswered' while five
fields crossed"* — reintroduced, with the guard reporting OK.

The guard reports OK because **its fixture pre-answers it**: `vault_with_year_n`
(`open_next_year_t4b.rs:47-73`) and `a_year_with_money_in_it` (`:87-…`) set no phone and no foreign
address, so the four leaves never differ from the blank return and the `changed` walk never sees
them. T10's own opener test (`the_trailer_splits_identity_from_the_per_year_credentials`,
`:1293-1327`) *does* populate all four — but it asserts only the seeded values and never touches
`opened.carried_identity` or `render()`, so the two tests between them cover the carry and miss the
announcement. This is the T9-Critical / T8-I-1 shape again: a fixture that cannot reach the gate.

**Why it matters on a filed return.** A filer with a foreign address in TY2024 who moves to the US
opens TY2025, reads *"CONFIRM each: … your mailing address"*, corrects the four domestic lines, and
is told nothing about the foreign row. `foreign_address_is_live()` reads only `foreign_country`,
which is still `"Elbonia"`, so `ReturnHeader::build` yields `Some(ForeignAddress { … })` and
`f1_15`/`f1_16`/`f1_17` print last year's foreign address on this year's domestic return — a header
cell printed from a stale carry, which is the brief's own Critical/Important line.

**Evidence.** Probe P-a: the four leaves added to the guard's fixture, nothing else changed.

```
$ cargo nextest run --locked -p btctax-cli -E 'test(every_leaf_the_seed_carries_is_named_in_the_report)'
thread 'every_leaf_the_seed_carries_is_named_in_the_report' panicked at
crates/btctax-cli/tests/open_next_year_t4b.rs:1427:13:
the seed writes `header.phone`, and no phrase in the opener's report names it — a filer reading
"everything else is blank" would have no reason to look. Report:
Opened TY2025 from TY2024.
  Carried from TY2024 — CONFIRM each: the filing status (…), your name and SSN, your mailing
  address, each employer and payer, by name and EIN/TIN, with every box blank, the carryforwards
  computed on that return, and which year this one was opened from.
  Everything else is blank and every question is unanswered.
```

and with the phone removed from the probe so the walk reaches the next leaf:

```
    the seed writes `header.foreign_country`, and no phrase in the opener's report names it — a
    filer reading "everything else is blank" would have no reason to look.
     Summary [   0.149s] 1 test run: 0 passed, 1 failed, 813 skipped
```

**Minimal change.** Two phrases in `CARRIED_IDENTITY` (and the mirror list in the test's `NAMED`):
`("your phone number", &["header.phone"], false)` and
`("your foreign address, if you have one", &["header.foreign_"], false)` — the second matters more
than the first, because it is the one that changes what prints. Then extend the guard's fixture
(`a_year_with_money_in_it`, or the `every_leaf_…` mutator) to populate all four, so the guard can
see the class at all rather than only the next omission.

---

### I-2 — line 35c is printed from an account type the filer never chose

**Where.** `crates/btctax-input-form/src/spec/sections.rs:774-795` (`DIRECT_DEPOSIT`'s
`OptionalSingleton::create`); `crates/btctax-core/src/tax/return_refuse.rs:2069-2091`
(`screen_direct_deposit`); `crates/btctax-forms/src/form1040_full.rs:463-478`
(`push_direct_deposit`); `crates/btctax-core/src/tax/classifier.rs` (the block-level
`c.exempt(direct_deposit, Class::BenefitClaim, …)`).

**What is wrong.** `create` writes `kind: DepositAccountKind::Checking`. `DepositAccountKind` has no
`None` and no "unconfirmed" state, `screen_direct_deposit` validates only the routing and account
strings, and `push_direct_deposit`'s `match` prints whichever box the enum holds. So a filer who
presses `c` on the Direct deposit section, types the two numbers off their cheque and never touches
the *"Account type (line 35c)"* row files a return with the **Checking** box checked — testimony
they never gave.

The instruction is explicit about the consequence: *"Check the appropriate box for the type of
account. Don't check more than one box. … **You must check the correct box to ensure your deposit is
accepted.**"* (`i1040gi--2025.txt:23988-23994`.) A savings-account filer who leaves the default gets
the deposit rejected and the refund delayed.

The seam's own comment offers the safety argument — *"`Checking` is not a guess about the filer's
account; it is the starting position of a two-way control the filer must confirm, and the block does
not reach a printed page until its numbers do"*. The first half asserts a confirmation step that
does not exist, and the second half is true but irrelevant: the block reaches the page the moment
the *numbers* are valid, carrying the unconfirmed *type* with it. The mechanism that hides it is the
block-level exemption: the classifier exempts the whole `direct_deposit` `Option` as class (B), so
the answered-ness machinery never looks inside it at a leaf that has no unanswered state — the
"widening an exemption is never the safe edit" shape.

**Evidence.** Probe P-c drives the real `create`, the real `set`s, the real screen and the real
print boundary. The type row is never touched.

```
$ cargo nextest run --locked -p btctax-input-form -E 'test(probe_the_account_type_box)' --no-capture
PROBE screen_param_free = None
PROBE stored kind = Checking
PROBE printed 35c box = Checking
```

**Minimal change.** Give the type an unanswered state and refuse on it, mirroring the routing
number's `Missing` leg that `create` already relies on for the two strings: make the stored field
`Option<DepositAccountKind>` (still no `#[serde(default)]`, §4.3), have `create` leave it `None`,
and add a `DirectDepositCell::Kind` leg to `screen_direct_deposit` so a block whose type is
unconfirmed refuses exactly as one with an empty routing number does. `push_direct_deposit`'s
exhaustive `match` then cannot print a box nobody chose, and `attribute.rs` already has the anchor
shape to point the cursor at `FieldId::DdKind`.

---

### I-3 — the written justification for the ABA rule describes a failure mode the code does not have

**Where.** `crates/btctax-core/src/tax/packet.rs:268-276` (the ★★ paragraph of
`RoutingNumber::canonical`'s doc), repeated in the T10 commit message and in
`design/agent-reports/2026-09-07-build-interview-T10-implementation.md` §3 D-3
(*"it fails safe (a refused number just means a paper check)"*).

**What is wrong.** The paragraph is the whole argument for applying a validity rule the IRS
instruction does not state, and all three of its factual claims are false:

> *"A number that fails any of the three rules **is not stored**, so the return **files with NO
> deposit block** — and `Advisory::RefundByPaperCheck` then **tells the filer** … **Nothing is
> blocked** and no figure moves"*

The number *is* stored (the seam's `set` writes the raw string with no validation —
`sections.rs:705-713`); the return does **not** file, because `screen_direct_deposit` is a value
rule outside the answered-ness tier (`return_refuse.rs:2459-2462`) and `resolve.rs:95-102` fails
closed on any input-screenable refusal; and `RefundByPaperCheck` never fires, because advisories are
computed only after the screen passes and its guard is `direct_deposit.is_none()` anyway.

The same commit states the truth 60 lines away — *"an empty routing number REFUSES
(`DirectDepositNumberMalformed`, the `Missing` leg), which is the point: a half-created block is
visible as a refusal"* (`sections.rs:780-785`) — so the two comments contradict each other.

This is not a behaviour bug: refusing loudly is *better* than the degradation the doc describes, and
the refusal names the cell, the rule and both remedies. It is an unsound assumption at the exact
point a future maintainer decides whether the rule may stay or be extended. "This class of rule
costs nothing because a failure just means a paper check" is the sentence that would license adding
btctax-only validity rules to other cells — every one of which would in fact block a return.

**Evidence.** Probe P-b: a test asserting exactly what the doc says.

```
$ cargo nextest run --locked -p btctax-core -E 'test(probe_a_bad_check_digit)'
thread '…::probe_a_bad_check_digit_degrades_to_a_paper_check_notice' panicked at
crates/btctax-core/src/tax/advisories.rs:2995:9:
PROBE(b): the doc says NOTHING IS BLOCKED, but the screen returned
Some(DirectDepositNumberMalformed { cell: Routing, why: BadCheckDigit })
     Summary [   0.005s] 1 test run: 0 passed, 1 failed, 1355 skipped
```

(The probe's first assertion — that the number *is* stored — passed before this one fired.)

**Minimal change.** Replace the ★★ paragraph with the real failure mode, which is a *better*
argument, not a worse one: *"A number failing any of the three rules refuses the return on both
tiers (`RefuseReason::DirectDepositNumberMalformed`) and again at the print boundary
(`HeaderError::Bank`). Nothing files until the filer corrects the number or deletes the block, and
the refusal states both remedies; a deleted block then files with `RefundByPaperCheck`. The rule may
therefore refuse rather than warn because the refusal is loud, cell-anchored and self-clearing — not
because it degrades silently."* Correct the same sentence in the build report's D-3 and carry the
correction into whatever `FOLLOWUPS`/roadmap entry quotes it.

---

### M-1 — a year whose map has no `[direct_deposit]` drops the filer's numbers in silence, with the compensating advisory also silent

**Where.** `crates/btctax-forms/src/form1040_full.rs:130-137`.

**What is wrong.** The write is `if let Some(cells) = map.direct_deposit.as_ref()`. If a year's map
lacks the section while `header.direct_deposit` is `Some`, lines 35b–35d print blank, no error is
raised, and `RefundByPaperCheck` is *also* silent — it reads `ReturnInputs`, not the map, so
`direct_deposit.is_some()` suppresses it. Neither half of the biconditional the comment right above
asserts (*"a map with no `[direct_deposit]` writes nothing … and a return with no `direct_deposit`
writes nothing and gets `Advisory::RefundByPaperCheck` instead"*) holds in that state. The same
function already has the right pattern eleven lines down — `need(&map.line1a, "line1a", y)?` refuses
when a money cell is missing.

**Reachability, and why it is Minor rather than Important.** Not reachable at `f8768e93`: TY2024 is
the only year that emits a full 1040 (TY2025's map has no `[header]` and `full_return_for(2025)` is
`None`), and TY2024 declares the block. K6 confirms the only emitting year is covered:

```
# [direct_deposit] deleted from forms/2024/f1040.map.toml, the four cells re-censused
$ cargo nextest run --locked -p btctax-forms
   FAIL btctax-forms::full_return_forms the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box
     Summary [   2.854s] 193/370 tests run: 192 passed, 1 failed, 4 skipped
```

But that KAT is TY2024-specific. The very next task on this roadmap (FR-84, the TY2025 identity
block) writes a TY2025 `[header]`; if it does not also write `[direct_deposit]`, a TY2025 filer's
routing and account numbers are dropped with no notice and **nothing reds** — the field census is
satisfied, because those cells staying on the `UNCENSUSED` register is legal.

**Minimal change.** Make the pair a hard error rather than a silent skip:

```rust
match (map.direct_deposit.as_ref(), header.direct_deposit.as_ref()) {
    (Some(cells), _) => push_direct_deposit(&mut writes, &mut placements, cells, header),
    (None, Some(_)) => return Err(FormsError::Geometry(format!(
        "the TY{y} 1040 map has no [direct_deposit] block, and this return carries a deposit \
         instruction — printing it blank would drop the filer's routing and account numbers \
         with no notice (RefundByPaperCheck is silent when a block is present)"
    ))),
    (None, None) => {}
}
```

---

### M-2 — `Advisory::RefundByPaperCheck`'s doc comment is stale

**Where.** `crates/btctax-core/src/tax/advisories.rs:228-230`.

> *"§3.4 / SPEC §9.2 conservative omission: v1 never fills the 1040 direct-deposit block (L35b–d),
> so a refund arrives as a paper check."*

Contradicted by the variant's own `message()` (`:607-615`), by the fire condition (`:1363`) and by
`LIMITATIONS.md`, all of which T10 updated. `the_advisory_does_not_name_a_cell_t10_now_fills`
asserts the *message* no longer claims this but does not read the doc comment.

**Minimal change.** Rewrite as *"fires when the return is due a refund and the filer gave no
direct-deposit instruction (§5.4/T10); the block itself is collected and printed when they do."*

---

### M-3 — clearing the foreign country leaves the province and postal code at rest, and any later country resurrects them

**Where.** `crates/btctax-input-form/src/spec/sections.rs:592-604` (`AddrForeignCountry`'s `set`).

Setting the country to `""` makes the other two dead (`get` → `None`, `set` → `NoSuchRow`) but does
not clear them, so entering *any* country later brings the old province and postal code back live —
and they then print. Compare the Spouse section's `delete`, which does clear `spouse_ip_pin`, and
the same commit's reasoning for it. Bounded because the values are visible in the editor the moment
they become live, so a filer changing countries sees them.

**Minimal change.** Have `AddrForeignCountry`'s `set` clear `foreign_province` and
`foreign_postal_code` when the new value is empty, mirroring `SPOUSE`'s `delete`.

---

### M-4 — secret handling (logged, never gating, per the owner ruling 2026-08-27)

The routing and account numbers are `FieldKind::Text`, not `Secret`, so the tax-inputs editor echoes
them on screen and shows them unmasked on every revisit — an asymmetry with the two IP PINs sitting
two sections away. The builder recorded this himself (build report §7 item 2) and the reasoning
given (a filer must be able to read a routing number back against their cheque) is sound. Confirmed,
and confirmed bounded: they *are* scrubbed (`scrub_routing`/`scrub_account`, class-preserving),
masked by `income show` (`mask_pii`, with a kill), masked in `Debug`
(`RoutingNumber(*********)` / `AccountNumber(*****)`), and never carried across a year. Logged as a
follow-up with its reproduction; not blocking.

---

### N-1 — "nothing on the return reads it" is said of a field that prints on the return

`sections.rs:660` (`AddrPhone`'s help: *"Optional: a blank is lawful and nothing on the return reads
it"*) and `scrub.rs` (*"nothing reads it, so only the blank/non-blank shape has to survive"*). Both
mean "no computation consumes it", but the phone *is* printed, in the signature block at `f2_37` —
which is the whole point of collecting it, and a filer reading the help could conclude the opposite.
Suggest *"no figure on the return is computed from it; it prints in the signature block."*

---

## Verdicts on the three adjudication questions

**1. The TY2025 map deliberately gets no T10 cells — is that right, and is the refusal reached?**
**Right, and the refusal is reached — twice over.** `crates/btctax-forms/forms/2025/f1040.map.toml`
has no `[header]`, and `fill_form_1040_full_with_map`'s *first statement*
(`form1040_full.rs:96-100`) is `map.header.as_ref().ok_or_else(…)`, before `pdf::load` and before
any write — so no TY2025 cell can be written by any path through that function. Belt and braces:
`full_return_for(2025)` is `None`, so the TY2025 full-return path is not reachable at all
(`tax_report.rs:1911` records this as by design). Mapping the cells would therefore be T8's recorded
*"blank because nothing populated it"* defect with a map entry in front of it, and the `UNCENSUSED`
register at `(2025, "f1040", 171)` / 285 is the honest home. **One condition on the verdict:** see
M-1 — the moment FR-84 writes a TY2025 `[header]`, omitting `[direct_deposit]` stops being honest
and becomes a silent drop of the filer's numbers with the compensating advisory suppressed, and
nothing reds. Fix M-1 before or with that task, not after.

**2. Rule 3 (the ABA check digit) is btctax's rule, not the instruction's — correct to add?**
**Yes, and the asymmetry runs strongly toward adding it — but the record of *why* is wrong (I-3).**
The two directions are not close. A false refusal of a genuine routing number is essentially
impossible: the ninth digit of an ABA routing transit number *is* the check digit, assigned with the
number, so every routing number a bank can hold satisfies it — including the "different routing
number for direct deposits" the instruction itself warns some institutions issue. Against that, the
check catches the great majority of single-digit and adjacent-transposition typos, and the
instruction's own consequence for a mistyped number is unrecoverable: *"You haven't given a valid
account number"* under **Reasons Your Direct Deposit Request Will Be Rejected** (`:24049-24050`) and
*"The IRS isn't responsible for a lost refund if you enter the wrong account information."*
(`:24023-24026`). Refusing the IRS's own printed sample `250250025` is not a counter-example — I
re-derived it independently, `3(2+2+0)+7(5+5+2)+(0+0+5) = 101`, `101 mod 10 = 1` — that number is
manufactured precisely so no bank can hold it.

**The filer is told, and told well.** Not silently given a paper check: `screen_direct_deposit`
refuses on both tiers with a message naming the cell, the rule and both remedies (*"Correct it, or
delete the direct-deposit block: a return with none is complete, and the refund then arrives as a
paper check"*), `attribute.rs:105-112` anchors the TUI cursor on `DdRouting` or `DdAccount`
depending on which cell failed, and `ReturnHeader::build` fails the print as a second boundary. That
is the right design. It is simply not the design the source says it is — hence I-3, which is a
correction to the *justification*, not to the rule.

**3. `RefundByPaperCheck`'s exit was moved off `btctax income answer` — does the new exit reach a
surface that can collect a routing number?** **Yes, verified, and the exclusion of `income answer`
is correct.** `sections::DIRECT_DEPOSIT` is registered in `form_spec()`
(`spec/mod.rs:56-59`); the TUI's section-cursor clamp moved 23 → 24 to admit it
(`btctax-tui-edit/src/main.rs:10720`); and `c` creates an `OptionalSingleton` through the generic
handler, the same one `tax_inputs_c_creates_x_deletes_schedule_a_resetting_itemize` exercises for
Schedule A — so `btctax tui-edit` → `T` on the year → Direct deposit → `c` yields three live fields,
one of which is *"Routing number (line 35b)"*. My probe P-c drove that exact path (`create` then the
real `set`s) and reached a printed block, which is the strongest available evidence that it is not a
brick. The second named surface holds too: `the_trailer_round_trips_through_the_toml_wire_and_a_
misspelt_key_is_named` parses `[header.direct_deposit]` and refuses a half-block. And `income
answer` genuinely cannot reach it — it walks `FORM_QUESTIONS`, `SKIPPABLE_QUESTIONS` and
`DEPENDENT_GATES` only, and the block is class (B) in no registry — so naming it would have been the
T7 brick. Correct call.

---

## Seams checked clean

- **Seam 1 — the validator.** Prefix rule is the instruction's two ranges with the edges tested
  (00/13/20/33/99 refuse, 01/12/21/32 pass); eight and ten digits refuse `WrongLength`; a non-digit
  refuses `BadCharacter`; whitespace is stripped, nothing else; `250250025` refuses `BadCheckDigit`
  and `250250024` passes. Messages name the rule and its cite. K1 (`if false` on the check-digit
  branch) reds two tests: *"the IRS's own sample-check number carries a wrong check digit — that is
  the point of a sample"* and *"250250025 must fail the print boundary, not print"*. The account
  rule is the instruction's sentence and nothing more; 18 characters refuse rather than truncate,
  and `verify_flat`'s `/MaxLen` leg (`verify.rs:425-438`) is a second, PDF-derived backstop that
  refuses a cell overflow on the read-back rather than letting a viewer truncate it.
- **Seam 2 — the advisory.** The 4-row truth table is real and all four rows are asserted (refund +
  no block → fires; no refund + no block → silent; refund + block → silent; no refund + block →
  silent), plus the wording assertions in both directions. `UnmodeledReturnOptionsOmitted` is
  unconditional (`advisories.rs:1350`), so the Form 8888 box's `covered_by` move is sound — and
  `census-join` validates that a `covered_by` names a variant that exists
  (`xtask/src/census_join.rs:588-599`). The export path derives its advisories from the same
  `advisories_for` call (`admin.rs:1706`), so report and packet cannot disagree.
- **Seam 3 — the secret.** `SpIpPin` is `FieldKind::Secret`; `set` refuses a `Text`; `get` returns
  presence only and, being an IP PIN, reveals zero digits. The sweep is enumerated from
  `form_spec()` with the count pinned at 6, so it reds on an addition and on a removal. K3 (a `get`
  that constructs `SecretView::Set` from the raw digits, bypassing the `set_masked` guard) reds:
  *"SpIpPin: the secret view returned the whole value: Set { masked: "987654321" }"*. K4 (dropping
  `spouse_ip_pin_if_live()` at the print boundary) reds at the emitter: *"a spouse IP PIN left
  behind by a deleted spouse must never reach the page"*. `scrub_pii` runs it through the same
  `scrub_ip_pin` carve-out (valid → dropped, never fabricated), `mask_pii` redacts it to `***`, and
  a grep of every committed golden, doc and fixture for the T10 sentinels (`123456780`, `111111118`,
  `SENTINEL-ACCT`, `ACCT-000123`, `250250025`) finds them only in the build report and the IRS
  extracts — no emitted artifact carries one.
- **Seam 4 — the cells.** Every FQN and on-state measured off the PDF (`dump-fields` output pasted
  in the map), and the two 35c on-states are distinct (`1`/`2`), which corroborates
  checking-vs-savings independently of the x-order. The nine retired census entries are exactly the
  nine mapped cells. `census_accounts_for_every_field` catches a cell that is both mapped and
  censused, and K6 shows the TY2024 read-back KAT catches the map section being removed. All five
  emitter KATs read back off a filled fixture against the real bundled template.
- **Seam 5 — liveness and the opener.** `foreign_address_is_live()` is the one reader, and
  `ReturnHeader::foreign_address` is a single `Option`, so half an address is unrepresentable at the
  print boundary rather than caught there. The two dependent fields' `live`, `get` and `set` all ask
  the accessor, so a US filer is never asked them. `direct_deposit` is optional and blocks nothing.
  The opener's *split* is right and asserted in both directions on one seeded year — but its
  *announcement* is not: see I-1.
- **Seam 6 — `LEAF_SOURCE` and the classifier.** D-2 is correct and I verified the mechanism rather
  than the claim: `LEAF_SOURCE`'s KAT audits both directions and reds on *"a prefix matching no
  money leaf"* (`provenance.rs:941`), and every T10 leaf is a `String`, `Option<String>` or enum —
  adding an entry would have broken the guarantee. `classify_header`'s destructuring names all six
  new leaves (five scalars waved past by the `_` rule, `direct_deposit` explicitly exempted class
  (B)), so a seventh would not compile silently. `income import` parses every new key and refuses a
  misspelt one and a half-block. The no-brick property is untouched: T10 adds no registry question,
  and its one new refusal fires only on a block the filer created, which `tui-edit` and `income
  import` can both clear — and K5 shows the census reds if that rule is moved inside the
  answered-ness tier where `income import` could not reach it (*"`income import` screens on a year
  with NO package — DirectDepositNumberMalformed must fire there too"*).

Counts: C=0 I=3 M=4 N=1
