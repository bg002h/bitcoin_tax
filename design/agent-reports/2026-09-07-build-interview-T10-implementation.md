# T10 — the trailer (§5.4): direct deposit, phone, spouse IP PIN, foreign address

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD
`ed754dd9`. Nothing committed; nothing pushed; no subagents; no `git checkout --` / `restore` /
`stash` (every plant reverted from a `cp` backup).

---

## 0. Commands, with real output

```
$ make check
     Summary [  19.757s] 3491 tests run: 3491 passed, 12 skipped

$ cargo fmt --all                                        # clean
$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.11s

$ cargo run -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [...], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0), 17 not line-bound (ratchet 17)

$ cargo run -p xtask -- census-join
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned; no forbidden shape

$ cargo run -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one decided (268 entries)
```

**The cells were MEASURED, not typed:**

```
$ cargo run -p xtask -- dump-fields crates/btctax-forms/forms/2024/f1040.pdf | grep -E "c2_4|c2_5|f2_25|f2_26|f2_36|f2_37|f1_15|f1_16|f1_17"
p1     36.0, 594.0- 258.5, 608.0  text  topmostSubform[0].Page1[0].Address_ReadOrder[0].f1_15[0]
p1    260.2, 594.0- 402.5, 608.0  text  topmostSubform[0].Page1[0].Address_ReadOrder[0].f1_16[0]
p1    404.2, 594.0- 467.2, 608.0  text  topmostSubform[0].Page1[0].Address_ReadOrder[0].f1_17[0]
p2    465.2, 469.5- 473.2, 477.5  btn   topmostSubform[0].Page2[0].c2_4[0]  on=["1"]
p2    172.8, 456.5- 302.4, 467.5  text  topmostSubform[0].Page2[0].RoutingNo[0].f2_25[0]  maxlen=9
p2    377.4, 457.0- 385.4, 465.0  btn   topmostSubform[0].Page2[0].c2_5[0]  on=["1"]
p2    435.0, 457.0- 443.0, 465.0  btn   topmostSubform[0].Page2[0].c2_5[1]  on=["2"]
p2    172.8, 444.5- 417.6, 455.5  text  topmostSubform[0].Page2[0].AccountNo[0].f2_26[0]  maxlen=17
p2    504.0, 270.0- 576.0, 282.0  text  topmostSubform[0].Page2[0].f2_36[0]  maxlen=6
p2    134.8, 258.0- 272.9, 270.0  text  topmostSubform[0].Page2[0].f2_37[0]
```

`c2_5[0]` is at x=377.4 and `c2_5[1]` at x=435.0; the printed row is
*"b Routing number   c Type: Checking   Savings"* (`f1040--2024.txt:116`), left to right — so
`c2_5[0]` is **Checking** (on `1`) and `c2_5[1]` is **Savings** (on `2`). The two on-states are
distinct, so the mapping is corroborated independently of the x-order.

---

## 1. What landed, per numbered item of the brief

### 1. `direct_deposit` + the routing validator + the 35b–d cells + the conditional advisory

| piece | where |
|---|---|
| `HouseholdHeader::direct_deposit: Option<DirectDeposit>` | `return_inputs.rs` |
| `DirectDeposit { routing: String, kind: DepositAccountKind, account: String }` — **no `#[serde(default)]` on any field** (§4.3). ★ **Corrected by seam review I-2:** `kind` is now `Option<DepositAccountKind>`, `create` leaves it `None`, and line 35c refuses until the filer chooses — see the fold report | `return_inputs.rs` |
| `DepositAccountKind { Checking, Savings }` — no `Default`, no `#[serde(other)]` | `return_inputs.rs` |
| `RoutingNumber` / `AccountNumber` / `BankNumberError` / `PrintedDirectDeposit` | `packet.rs` |
| `RefuseReason::DirectDepositNumberMalformed { cell: DirectDepositCell, why: BankNumberError }` + `screen_direct_deposit` (a **value** rule, refuses on BOTH tiers) | `return_refuse.rs` |
| `ReturnHeader::direct_deposit`, canonicalized at the print boundary; `HeaderError::Bank` | `packet.rs` |
| `DirectDepositCells` map section + `Form1040Map::direct_deposit` | `map.rs` |
| `[direct_deposit]` in `forms/2024/f1040.map.toml`; the four census entries retired | map TOML |
| `push_direct_deposit` | `form1040_full.rs` |
| `SectionId::DirectDeposit` (OptionalSingleton) + `DdRouting` / `DdKind` / `DdAccount` | `seam.rs`, `sections.rs`, `spec/mod.rs` |
| `Advisory::RefundByPaperCheck` now `refund > 0 && direct_deposit.is_none()` | `advisories.rs` |

**The validator's rules, with cites.**

| rule | source | verdict |
|---|---|---|
| nine digits | *"The routing number must be nine digits."* — `i1040gi--2025.txt:23967` | refuse `WrongLength(n)` |
| first two digits 01–12 or 21–32 | *"The first two digits must be 01 through 12 or 21 through 32."* — `:23968-23969` | refuse `BadPrefix(p)` |
| the ABA check digit `3(d1+d4+d7)+7(d2+d5+d8)+(d3+d6+d9) ≡ 0 (mod 10)` | **NOT in the IRS instruction** — see §3 deviation D-3 | refuse `BadCheckDigit` |
| account: *"up to 17 characters (both numbers and letters). Include hyphens but omit spaces and special symbols."* | `:24054-24059` | refuse `WrongLength(n>17)` / `BadCharacter(c)`; spaces stripped |

**Line 35a's Form 8888 box stays `unmodeled`** and its `covered_by` MOVED from
`Advisory::RefundByPaperCheck` to `Advisory::UnmodeledReturnOptionsOmitted` — see D-5.

### 2. `phone` — collected and printed in the signature block

`header.phone: String` → `FieldId::AddrPhone` (Address section) → `ReturnHeader::phone` →
`cells.phone` (`f2_37`). Cite: *"Phone no."*, `f1040--2024.txt:137`. `f2_37`'s census entry retired.
Not coerced into any shape — btctax knows no dialling plan.

### 3. `spouse_ip_pin` — secret, asymmetric, printed

`header.spouse_ip_pin: Option<String>` → `FieldId::SpIpPin` (`FieldKind::Secret`, spouse-gated) →
`ReturnHeader::spouse_ip_pin: Option<IpPin>` → `cells.spouse_ip_pin` (`f2_36`). Cite:
*"If the IRS sent your spouse an Identity Protection PIN, enter it here (see inst.)"*,
`f1040--2024.txt:133-135`. Every mirror of the taxpayer's handling:

- `set` takes only `SecretEntry`, `get` returns `SecretView` (presence, zero digits);
- an empty entry clears to `None`, never `Some("")` (the I-2 export-brick);
- `scrub_pii` runs it through the **same** `scrub_ip_pin` carve-out — valid → DROPPED, never a
  fabricated credential;
- `income show`'s `mask_pii` redacts it to `***`;
- the `income import` scrub-marker guard is unchanged and covers it (it is a pre-parse text scan).

**The T9 lesson, applied.** `HouseholdHeader::spouse_ip_pin_if_live()` is the **one reader**: the
seam's `get` and `ReturnHeader::build` both go through it, so a PIN left behind by a deleted spouse
cannot print into the *"If the IRS sent your spouse…"* cell of a return with no spouse. The Spouse
section's `delete` also clears the leaf, so nothing is left at rest either.

### 4. The foreign address block

`header.foreign_country` / `_province` / `_postal_code: String` → `AddrForeignCountry` /
`AddrForeignProvince` / `AddrForeignPostalCode` → `ReturnHeader::foreign_address:
Option<ForeignAddress>` → `f1_15` / `f1_16` / `f1_17`. Cite: *"Foreign country name | Foreign
province/state/county | Foreign postal code"*, `f1040--2024.txt:22`, printed under *"If you have a
foreign address, also complete spaces below."*

**§5.4's liveness rule is structural, not screened.** `HouseholdHeader::foreign_address_is_live()` is
the one reader (the two seam `Field`s and `ReturnHeader::build`), and `ReturnHeader` carries **one
`Option`** rather than three strings — so *"a province with no country"* is unrepresentable at the
print boundary rather than caught there. All three census entries retired.

`design/forms/FIELD_PROVENANCE.md` §6a gains a dated CLOSED note listing the nine retired cells (the
measurement itself is left as taken; rewriting a recon's numbers would hide when the projection was
made).

### 5. Fixtures

| fixture | change |
|---|---|
| `coverage.rs::maximal_fixture` (§5.7) | `direct_deposit` **present**, `foreign_country = "Elbonia"` (the liveness primer for the other two), `spouse_ip_pin = Some("000000")` |
| `scrub_axis::maximal_sentinel` | all six leaves non-default; routing `111111118`, account `SENTINEL-ACCT-1` |
| `full_return_forms::t10_trailer_header` (new) | one mutator per kill, so each states exactly one thing |
| `return_refuse::param_free_fixtures` | `DirectDepositNumberMalformed` fixture, routing `250250025` |
| `open_next_year_t4b` | year N carrying all six leaves |
| `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml` | regenerated — **+4 empty keys only** (`foreign_country`, `foreign_postal_code`, `foreign_province`, `phone`); the two `Option`s serialize away |

**Synthetic identifiers used.** `123456780` (digits 1–8 sequential, ninth DERIVED as the ABA check
digit those eight force; prefix 12) — the same family as this repo's `123-45-6789` / `12-3456789`.
`111111118` (repeated-digit, ninth derived; prefix 11) in the scrub-axis sentinel, which must differ
from `SCRUB_ROUTING` or the field drops out of the derived axis. `250250025` — **the IRS's own
sample-check routing number** (`i1040gi--2025.txt:23965`), used only where a check-digit FAILURE is
wanted. No real bank's routing number is in the tree.

---

## 2. The kills, each with its planted defect and the observed red

Every plant applied to the real source, the real test run, the red pasted, then restored from a `cp`
backup. Twenty-one plants; the sixteen new tests are named in §5.

| # | plant | test | observed red (verbatim) |
|---|---|---|---|
| P1 | `RoutingNumber::canonical`'s prefix check replaced by `if false` | `the_routing_prefix_rule_is_the_instructions_two_ranges_and_their_edges` | `assertion left == right failed: prefix 00 is outside "01 through 12 or 21 through 32": 000000000` |
| P2 | the ABA check-digit branch replaced by `if false` | `t10_bank_numbers` (2 of 5) | `assertion left == right failed: the IRS's own sample-check number carries a wrong check digit — that is the point of a sample` **and** `assertion left == right failed: "250250025" must fail the print boundary, not print` |
| P3 | `if refund > Usd::ZERO && …is_none()` → `if refund > Usd::ZERO` | `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` | `panicked at advisories.rs:3014` (row 3: `!fires(dec!(1234.56), block())`) |
| P4 | the same guard → `if ri.header.direct_deposit.is_none()` | same | `assertion failed: !fires(Usd::ZERO, None)` |
| P5 | `FOREIGN ADDRESS` restored to the return-options advisory | `the_advisory_does_not_name_a_cell_t10_now_fills` | `the advisory still names "FOREIGN ADDRESS", which T10 collects and the emitter prints: …` |
| P6 | `SpIpPin`'s `get` returns the raw digits via `SecretView::set_masked` | `every_secret_field_is_asymmetric_written_never_read_back` | `SecretView::set_masked was given a string with a 5+ digit run (a raw secret?): "987654321"` — the seam's own guard fired first |
| P6b | the same, constructing `SecretView::Set { masked }` directly to bypass that guard | same | `SpIpPin: the secret view returned the whole value: Set { masked: "987654321" }` |
| P7 | `mask_pii`'s spouse-PIN branch replaced by `if false` | `income_show_redacts_the_spouse_ip_pin_and_the_bank_numbers` | `assertion left == right failed  left: Some("654321")` |
| P8 | `foreign_address_is_live()` → `true` in `ReturnHeader::build` | `the_foreign_address_prints_only_under_a_country` | `the foreign province must not print without a country — half an address is not an address  left: Some("Mud Province")  right: None` |
| P9 | the map's 35c `checking`/`savings` FQNs+on-states transposed | `the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box` | `Checking: line 35c's own box, at its own measured on-state  left: None  right: Some("1")` |
| P10 | the opener seeds `spouse_ip_pin` and `direct_deposit` | `the_trailer_splits_identity_from_the_per_year_credentials` | `an IP PIN is issued per year — carrying December's would print a void credential  left: Some("654321")  right: None` |
| P11 | the opener's four identity `clone()`s deleted | same | `assertion left == right failed  left: ""  right: "555-0100"` |
| P12 | `screen_direct_deposit`'s body removed from the source census's `named` set | `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` | `the fixture table and the source census must name the same param-free rules` — left carries `"DirectDepositNumberMalformed"`, right does not |
| P13 | the `screen_direct_deposit(ri)` call removed from `screen_inputs_tiered` | same | `the commit gate must reach DirectDepositNumberMalformed on its own fixture  left: None  right: Some("DirectDepositNumberMalformed")` |
| P14 | `scrub_routing`'s `Ok` arm returns the filer's real number | `scrub_axis::matrix::every_replaced_field_preserves_its_class_in_every_representable_state` | `§3.3's matrix has a row for header.direct_deposit.routing, which is NOT in the derived axis — either the fixture stopped being maximal or scrub stopped replacing this field` |
| P15 | `scrub_routing`'s `BadCheckDigit` arm returns a VALID stand-in | same | `header.direct_deposit.routing @ malformed: scrubbing changed WHETHER (or why) the return refuses. … or, worse, FILES where the original could not.  left: Some(DirectDepositNumberMalformed { cell: Routing, why: BadCheckDigit })  right: None` |
| P16 | the map's `routing` FQN points at the 35d account widget | `the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box` | `Checking: line 35b must carry the nine bare digits  left: None  right: Some("123456780")` |
| P17 | `#[serde(default)]` added to `DirectDeposit::routing` | `the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named` | `a direct-deposit block with no routing number must not parse — it is a mistyped row, not a lawful state` |
| P18 | the emitter's spouse-IP-PIN write deleted | `the_spouse_ip_pin_prints_and_never_without_a_spouse` | `the spouse's IP PIN cell (f2_36) must carry the six digits  left: None  right: Some("654321")` |
| P19 | the emitter's phone write deleted | `the_phone_number_prints_in_the_signature_block_and_nowhere_else` | `assertion left == right failed  left: None  right: Some("555-0100")` |
| P20 | `push_direct_deposit` fabricates a block when the return gave none | `no_direct_deposit_leaves_the_whole_refund_block_blank` | `35b routing untouched  left: Some("123456780")  right: None` |
| P21 | the map's `phone` cell points at `f2_38` (email), which is still censused | `field_census::census_accounts_for_every_field` | `2024/f1040: ["…f2_38[0]"] are BOTH mapped and censused — a field cannot be one we fill and one we deliberately leave blank` |

**Two more reds were observed WITHOUT a plant**, because the standing instruments bit during the
build and are recorded as the same evidence:

- `coverage.rs`: `set failed for AddrForeignProvince in Address: NoSuchRow` — the mutate-and-diff KAT
  refusing an unprimed fixture, which is what forced the liveness primer.
- `scrub_axis`: `the derived axis contains header.direct_deposit.account but §3.3's matrix has no row
  for it. A field was added to scrub_pii without anyone deciding its class behaviour.`

**Where a kill CALLS the instrument** (the T9/T8 shadow test): P12 and P13 are the two halves of the
same guarantee — P12 proves the *census* can see the new rule, P13 proves the *rule* is wired. P12 is
the one that matters: `screen_direct_deposit` lives in its own helper, and the source census reads
`screen_inputs_tiered`'s body plus two named helpers. A third helper joined it, so without the
one-line addition to `named` the whole rule would have been uncensused **while the test still printed
OK** — the exact shape B1 exists for.

---

## 3. Deviations, each with reasoning

**D-1 — the TY2025 map gets NO T10 cells, and that is deliberate.** The brief says *"T10 adds the
trailer and foreign cells for 2025 through the label reader the same way"*. It cannot honestly. The
TY2025 map has no `[header]`, and `fill_form_1040_full_with_map` refuses at
`map.header.as_ref().ok_or_else(…)` — *"the TY2025 1040 map has no [header] block — a full return
cannot file an unnamed 1040"* — **before any cell on either page is written**. A mapped cell nothing
writes is the *"blank because nothing populated it"* defect wearing a map entry, which is T8's own
recorded reason for leaving the TY2025 grid rows (1)–(4) unmapped. The cells therefore stay on the
field census's `UNCENSUSED` register — `(2025, "f1040", 171)`, unchanged, `UNCENSUSED_ENTRIES = 5`,
`UNCENSUSED_FIELDS = 285` — which is the honest home for a cell with no writer yet. The task that
maps the TY2025 identity block owns them, and FR-84 (the TY2025 name cells) is already filed against
it. **Nothing in T10 is TY2024-only by design; only the MAP is.**

**D-2 — `LEAF_SOURCE` gains no entry.** The brief's settled-facts block names *"T1's `LEAF_SOURCE`
(`FilerRecords` for these)"*. `LEAF_SOURCE` is a partition of **money** leaves, and every T10 leaf is
a `String`, an `Option<String>` or an enum — there is no `Usd` anywhere in the trailer. Its KAT runs
**both directions**, so a prefix matching no money leaf REDS. Adding one would have broken the
guarantee it exists to hold.

**D-3 — the ABA check digit is applied, and the IRS instruction does not state it. Stated in the
source rather than hidden.** `RoutingNumber::canonical`'s doc says so in as many words. The reasons
it is applied anyway: the ninth digit of an ABA routing transit number exists for no other purpose;
the instruction's own consequences for a mistyped number are severe and silent (*"You haven't given a
valid account number"* under **Reasons Your Direct Deposit Request Will Be Rejected**,
`i1040gi--2025.txt:24031-24052`, `:24049-24050`; and *"The IRS isn't responsible for a lost refund if
you enter the wrong account information."*, `:24023-24026`).

> ★★★ **CORRECTED BY THE T10 SEAM REVIEW (I-3), 2026-09-07 — the sentence that stood here was false
> in all three of its claims and is retained struck through, because the argument it made is the one
> a future maintainer must NOT reason from.**
>
> ~~"and **the failure mode of applying it is safe** — a refused number is simply not stored, the
> return files with no deposit block, and `RefundByPaperCheck` tells the filer in words. Nothing is
> blocked and no figure moves."~~
>
> The number **is** stored (the seam's `set` writes the raw string with no validation); the return
> does **not** file (`screen_direct_deposit` is a VALUE rule outside the answered-ness tier, so it
> refuses on both tiers, `resolve.rs` fails closed on it, and `ReturnHeader::build` refuses again at
> the print boundary); and `RefundByPaperCheck` **never fires**, because advisories are computed only
> after the screen passes and its guard is `direct_deposit.is_none()` — a present-but-malformed block
> silences it.
>
> **The rule is still right, on the true terms: this refusal BLOCKS A RETURN, and is defensible
> because it is loud, cell-anchored and self-clearing** — it names the cell, the rule and both
> remedies, the input form anchors the cursor on the failing field, and deleting the block makes the
> return file and the advisory speak. What must not be carried anywhere else is the inference the
> false version licensed: *"a btctax-only validity rule costs nothing, because a failure just means a
> paper check."* Every such rule blocks a return until the filer acts, and adding one to another cell
> has to be argued on its own near-zero false-refusal rate, its own remedy and its own unrecoverable
> harm from accepting.
>
> Source of record: `crates/btctax-core/src/tax/packet.rs` (`RoutingNumber::canonical`'s doc), with
> `advisories::tests::a_malformed_routing_number_blocks_the_return_and_the_paper_check_notice_stays_silent`
> asserting the corrected behaviour in its own order. The same false claim also went into the T10
> commit message (`f8768e93`), which cannot be rewritten; the review ledger and this note are its
> correction of record.

The one thing that cannot happen is a wrong number printed on a filed return.

> ★★★ **A finding worth its own line: the routing number the IRS PRINTS ON ITS OWN SAMPLE CHECK does
> not satisfy the ABA check digit.** `250250025` — `3(2+2+0) + 7(5+5+2) + (0+0+5) = 101`, and
> `101 mod 10 = 1`. Which is exactly right for a worked example: the IRS chose a number no bank can
> hold. It is therefore the one routing number in this repo *documented* to fail rule 3 rather than
> merely observed to, and it is what `scrub` mints for a filer whose own entry failed the same rule,
> and what the `DirectDepositNumberMalformed` fixture uses. It also means rule 3 is **btctax's**, not
> the IRS's, which is why D-3 is written down.

**D-4 — an ACCOUNT-number validator was built, and the brief named only the routing one.** The 35d
cell is `/MaxLen 17`; an 18-character entry would be **silently truncated on the printed page** — a
wrong account number that looks right, invisible in the emitted PDF and to both oracles. It refuses
instead. The rule is the instruction's sentence and nothing more (`:24054-24059`), including *"omit
spaces"*, which is obeyed for the filer (transcription, not a guess).

**D-5 — `Advisory::UnmodeledReturnOptionsOmitted`'s list went from SEVEN administrative cells to
FIVE, and line 35a's Form 8888 box JOINED it.** Three of the seven (the foreign address, the spouse's
IP PIN, the phone number) are collected and printed now; naming them would tell a filer who supplied
them that btctax never asked, and send them to write on a form that already carries the value.
Separately, the Form 8888 box's census `covered_by` had to move: it pointed at
`RefundByPaperCheck`, and that advisory is **conditional** since T10 — so a filer who gives a deposit
block and wants a split refund would have been "covered" by a notice they never see.
`UnmodeledReturnOptionsOmitted` is unconditional on a computed full return, so it can carry it.
`the_advisory_does_not_name_a_cell_t10_now_fills` asserts both directions.

**D-6 — `RefundByPaperCheck`'s exit was changed to name a surface that can actually take the
answer.** The first draft said *"(`btctax income answer`, or the tax-inputs editor)"*. `income answer`
walks `FORM_QUESTIONS`, `SKIPPABLE_QUESTIONS` and `DEPENDENT_GATES` and **nothing else** — the
direct-deposit block is class (B) and is in no registry (R14), so that command can never mention it.
Naming it would have been a brick, and this repo's advisories are held to *"it must name the action
that actually works"* (`advisories.rs`'s own `income answer || income import` assertion). The notice
now names `btctax tui-edit` (then `T` on the year) and `btctax income import`, and the test asserts
`income answer` is **absent**.

**D-7 — `scrub_refusal::the_malformed_classes_survive_the_artifact_boundary` was tightened, not
worked around.** Its leak check was `!file.contains('Z')` — a whole-file search for one common
letter. A new foreign-province stand-in tripped it, i.e. it went red on the STAND-IN rather than on a
leak. It now asserts `!emitted.contains("678Z")` (the tail of the fixture's own malformed SSN, which
no stand-in could contain by accident) plus `!landed.header.taxpayer.ssn.contains('Z')`. Strictly
more precise, still a whole-file scan. The stand-in was also made Z-free as defence in depth.

**D-8 — `phone` sits in the **Address** section, not Taxpayer.** It prints on page 2, but it is the
same thing the filer is typing (how to reach them) and §4.1 lists it in that section's line.

**D-9 — the accessors are new, and are the T9 fix applied before the defect.**
`HouseholdHeader::spouse_ip_pin_if_live()` and `::foreign_address_is_live()` exist so that no
liveness conjunct is ever re-typed at a second site: the seam and `ReturnHeader::build` both read
them, which is why P8 (dropping one) reds at the *emitter*.

**D-10 — the TUI's Address section glyph moved `✓` → `…` in the walkthrough golden.** Four new
lawfully-blank `Text` fields, and `section_glyph` shows `…` when any live field is unanswered. This is
the same state `Taxpayer` has already had (its IP PIN is blank) and `Payments` and `Carryforwards`
have. Nothing gates on the glyph; refusals are shown as `!` and the blocking list comes from
`screen_inputs`. Recorded as an observed, intended consequence rather than fixed.

---

## 4. Pinned numbers moved: old → new, with cause

| pin | old | new | cause |
|---|---|---|---|
| `coverage.rs` `field_count` | 271 | **279** | eight new `Field`s: `SpIpPin`, `AddrForeignCountry`, `AddrForeignProvince`, `AddrForeignPostalCode`, `AddrPhone`, `DdRouting`, `DdKind`, `DdAccount` |
| `coverage.rs` `covered.len()` | 271 | **279** | the same eight; every one distinctly covered |
| `xtask census-join` unmodeled entries | 283 | **274** | **−9**: `f1_15`, `f1_16`, `f1_17`, `f2_25`, `c2_5[0]`, `c2_5[1]`, `f2_26`, `f2_36`, `f2_37` retired from the TY2024 `f1040` census |
| TUI `section_idx` clamp | 23 | **24** | `SectionId::DirectDeposit` joins `form_spec()` |
| `every_secret_field_…` sweep count | *(new)* | **6** | the form's secrets, enumerated from `form_spec()`: three SSNs, R5's seller-financed-mortgage payer SSN, and both IP PINs |
| `make check` | 3475 passed, 12 skipped | **3491 passed, 12 skipped** | +16, exactly the sixteen new tests in §5 |
| the return-options advisory's count word | SEVEN | **FIVE** | D-5 |

**Deliberately UNCHANGED, and verified so:** `line-coverage` 375 / 18 forms / 31 exceptions (ratchet
31) / 0 unverifiable / 17 not line-bound; `stop-list` 8 + 4 + 91; `prompt-check` 88; `box-census`
268 / 19 / 9; `EXEMPT_PREFIX_CEILING` 5 and `EXEMPT_PREFIXES.len()` 5; `UNCENSUSED_ENTRIES` 5 and
`UNCENSUSED_FIELDS` 285 (D-1). **No tax figure anywhere moved** — the whole of `docs/examples.md`'s
diff is the two advisory texts plus six new `income show` keys, all empty or `null`.

---

## 5. The sixteen new tests

`btctax-core` — `tax::packet::t10_bank_numbers::{the_routing_prefix_rule_is_the_instructions_two_ranges_and_their_edges, the_irs_sample_check_routing_number_fails_the_aba_check_digit, a_routing_number_is_nine_digits_and_nothing_but_digits, an_account_number_is_the_instructions_sentence, the_header_build_refuses_a_deposit_block_that_cannot_route}`;
`tax::advisories::tests::{the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given, the_advisory_does_not_name_a_cell_t10_now_fills}`.

`btctax-input-form` — `spec::sections::tests::every_secret_field_is_asymmetric_written_never_read_back`.

`btctax-cli` — `cmd::tax::tests::{income_show_redacts_the_spouse_ip_pin_and_the_bank_numbers, the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named}`;
`open_next_year_t4b::the_trailer_splits_identity_from_the_per_year_credentials`.

`btctax-forms` — `full_return_forms::{the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box, no_direct_deposit_leaves_the_whole_refund_block_blank, the_spouse_ip_pin_prints_and_never_without_a_spouse, the_phone_number_prints_in_the_signature_block_and_nowhere_else, the_foreign_address_prints_only_under_a_country}`.

Suite lines per crate come from the one `make check` run above (3491/3491); no suite was run twice.

---

## 6. Citation drift found (measured, not assumed)

The controller's five re-measurements were correct and were used. Every other line number I relied on
was re-measured at HEAD `ed754dd9` with `git show HEAD:<file>`:

| citation | says | actually at `ed754dd9` |
|---|---|---|
| brief: `forms/2024/f1040.map.toml:94` (the UNMAPPED note) | :94 | **:94 ✓** — `# UNMAPPED ON PURPOSE: the direct-deposit block (35b routing f2_25, 35c/d account f2_26) — v1 never` |
| SPEC §5.4: *"secret, asymmetric per `seam.rs:220-234`"* | `seam.rs:220-234` | **wrong file** — `:220-234` is `SKIPPABLE_QUESTIONS`' `FieldId` variants (`DonationsHadRestrictions`, …). The IP-PIN `Field` is `spec/sections.rs:288`; the `Secret`/`SecretView` types are `seam.rs:599-660` |
| SPEC §5.4: `HouseholdHeader (S, ':233')` | `return_inputs.rs:233` | **`:876`** |
| SPEC §5: *"All on `ReturnInputs` (`return_inputs.rs:916`)"* | `:916` | **`:1878`** |
| SPEC R14: `RefundByPaperCheck` (`advisories.rs:172`) | `:172` | **`:224`** (the controller's correction; `:172` is a `}`) |
| SPEC R14: `no_option_money_leaf_is_bound_with_underscore` (`classifier.rs:28,913`) | `:913` | **`:1483`** |
| SPEC §5.7: `coverage.rs:88, maximal_fixture` | `:88` | **`:88` ✓** |
| SPEC §5.4 / brief: the routing rule at `i1040gi--2025.txt:23963-24000` | a 37-line span | the sentence is **`:23967-23969`**; line 35c is `:23988-23994`; line 35d's account rule is **`:24054-24059`** (not inside the cited span at all) |

Every citation written into the source by this task was measured before it was typed.

---

## 7. Follow-up candidates (not filed — the controller owns `FOLLOWUPS.md`)

1. **TY2025 trailer + identity block.** D-1. The nine cells this task mapped for TY2024 have TY2025
   counterparts that stay on the `UNCENSUSED` register until a task writes a TY2025 `[header]`. Owning
   phase: whichever task closes FR-84.
2. **Secret handling (logged, never gating, per the owner ruling).** The routing and account numbers
   are `FieldKind::Text`, not `Secret`, so the tax-inputs editor **echoes them on screen** and shows
   them unmasked on revisit. This is deliberate — a `Secret` field's `get` returns presence only, and
   a filer must be able to check a routing number against their cheque — but it is a real asymmetry
   with the IP PINs and is recorded. They *are* scrubbed (`scrub_routing`/`scrub_account`), masked by
   `income show`, and never carried across a year.
3. **`docs/man/btctax-tui-edit.1`'s TAX INPUTS MODE enumeration is stale** and was already stale
   before T10: it names the header, W-2s, Schedule A, dependents, Payments, declarations and
   skippables, and omits T5's six 1099 sections, T9's Form 1098 / 8b / home-sale sections, T16's three
   HSA sections and now T10's Direct deposit. It is hand-authored roff, not generated. Owning phase:
   **T12** (which owns man pages).
4. **Owner question Q4 remains OPEN, and nothing here decides it.** The block is an OptionalSingleton:
   either answer is expressible, absence is read as *no instruction given* (never as *"I want a paper
   check"*), and `RefundByPaperCheck` says exactly that out loud. If Q4 comes back *"always a check"*,
   nothing needs removing — the filer simply never creates the section.

---

## 8. Closing gate

```
$ make check
     Summary [  19.757s] 3491 tests run: 3491 passed, 12 skipped
```

`cargo fmt --all` clean; `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
--all-features -- -D warnings` clean. Nothing committed, nothing pushed; 29 files modified, no
untracked files, no plant residue (`git status --porcelain` lists modifications only).
