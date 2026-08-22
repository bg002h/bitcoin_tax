# Domain lens — $1,000,000 gift to a church (§170 ceilings, Form 8283, appraisal)

Repo: `/scratch/code/bitcoin_tax` @ `f5ba41a1`. All citations are `file:line` against that commit.

**Scope fact established first, because it decides the rest:** `BundledFullReturnTables::load()`
registers **TY2024 only** — `crates/btctax-adapters/src/tax_tables.rs:99-101`
(`by_year.insert(2024, ty2024_full_return())`, one entry). TY2025/TY2026 exist in
`BundledTaxTables` (`:75-79`) but only for the crypto slice. So this vector is a **TY2024 return**,
and the OBBBA §170(p) 0.5%-of-AGI floor and the §68 2/37 haircut (effective for tax years beginning
after 2025) are **out of scope by construction** — do not let a synthesis pass spend budget there.

Arithmetic for the vector (AGI = $1,000,000 wages + $1,000,000 LTCG = **$2,000,000**; the
student-loan deduction is fully phased out, so no adjustment moves it):

| shape of the gift | class | ceiling | allowed on Sch A | carryover-out |
|---|---|---|---|---|
| **$1M cash** | `Cash60` | 60% × $2M = $1,200,000 | **L11 = $1,000,000** | none |
| **$1M long-held BTC** | `CapGainProp30` | min(30%×$2M, 50%×$2M − 0 − 0) = **$600,000** | **L12 = $600,000** | **$400,000**, 2024 vintage, 5 years |

Both figures are what `apply_170b` computes (`crates/btctax-core/src/tax/charitable.rs:141-166`),
and both are correct. §170(e)(1)(B)(ii) does **not** reduce the BTC gift because the donee is a
50%-org, and `crypto_charitable_gifts` deducts FMV for a long-term leg
(`crates/btctax-core/src/tax/return_1040.rs:642-670`). The ceiling ordering
(`60% cash → 50% ordinary → 30% cap-gain`, each later tier capped by the residual 50% room) does not
bind on this vector under **any** reading of §170(b)(1)(G)(iii), because there is only one class in
play; the ordering question is real but latent here.

---

## VERDICT — can this slice FILE today?

**Cash shape: files clean.** Schedule A line 11 = $1,000,000, line 12 = $0, line 13 = $0,
line 14 = $1,000,000; no Form 8283 (the packet attaches one only when the return itemizes **and**
printed L12 > $500 — `crates/btctax-core/src/tax/packet.rs:657-664`,
`crates/btctax-core/src/tax/printed.rs:188`). No refusal in the charitable path fires. The filer is
handed a correct, complete, filable Schedule A **with no warning whatsoever** — which is the problem
described in item 3 below: the one substantiation rule that can vaporise the whole $1,000,000
deduction (§170(f)(8)) is never mentioned on the path that produces zero warnings.

**Bitcoin shape: files, and the figure is right, but the return is INCOMPLETE and one printed entry
is fabricated.** The engine computes $600,000/$400,000 correctly, refuses until the filer answers
the Form 8283 line 5a/5b/5c restriction question
(`crates/btctax-core/src/tax/return_1040.rs:2128-2165`), refuses a non-crypto noncash gift it has no
8283 rows for (`:833-860`), and emits a real Section B 8283 with the "k Digital assets" box checked.
But: it never tells the filer that a $1,000,000 claim triggers §170(f)(11)(D)'s **attach the
qualified appraisal to the return** requirement; it will emit a Section B with a completely blank
Part IV appraiser block on a **warning only, exit 0**; it prints an amount in Section B column (i)
that the instructions tell an individual filer **not to complete**; and it never surfaces the
$400,000 carryover anywhere the filer will see it. The deduction btctax computes is right and the
paper it hands over is not filing-ready — and only one of those four is said out loud.

---

## WHAT IS MISSING

### 1. §170(f)(11)(D) — over $500,000, the qualified appraisal must be **attached**. btctax has no such concept. (Bitcoin shape. CRITICAL.)

**Required:** i8283 (TY2024), `design/forms/extract/i8283--2024.txt:1015-1019`-equivalent /
`i8283--2025.txt:1015-1019`: *"Deduction of more than $500,000. If you are claiming a deduction of
more than $500,000 for an item (or group of similar items) donated to one or more donees, you must
attach the qualified appraisal of the property to your return unless an exception applies."* The
same instructions list it under the §170(f)(11)(A) denial conditions (`i8283--2025.txt:1382-1389`).

**btctax does:** nothing. The only threshold in the codebase is
`QUALIFIED_APPRAISAL_THRESHOLD = $5,000` (`crates/btctax-core/src/tax/tables.rs:190`). `grep` for
`500000` across `btctax-core/src`, `btctax-cli/src`, `btctax-forms/src` returns only unrelated test
literals. The packet manifest (`crates/btctax-cli/src/cmd/admin.rs:960-975`) lists PDFs in
Attachment Sequence order and has no line for a filer-supplied attachment.

**Consequence:** **files an incomplete return.** Under §170(f)(11)(A)(i) the entire $600,000
deduction is disallowable for want of the attachment. This is the single largest exposure in the
whole vector — ~$222,000 of tax at the 37% bracket — and it is invisible on the emitted page,
invisible to both oracles, and invisible to every existing test.

### 2. A Section-B Form 8283 with no appraiser at all is emitted, with exit 0. (Bitcoin shape. CRITICAL.)

**Required:** §170(f)(11)(C)/§6695A — Section B Part IV (Declaration of Appraiser) on the Rev.
12-2023 form; CCA 202302012 (archived at `legal/text/irs-guidance/CCA_202302012.txt`) removes the
publicly-traded-securities exception for crypto, so an appraisal **is** required at $1,000,000.

**btctax does:** `DonationDetails::is_review_complete(Section::B)` correctly demands appraiser name
+ TIN-or-PTIN + appraisal date + qualifications + donee EIN
(`crates/btctax-core/src/donation.rs:68-79`), and `form_8283` sets `needs_review`
(`crates/btctax-core/src/forms.rs:460-462`). But **`needs_review` has no consumer that gates
anything.** `grep -rn needs_review` outside tests reaches only CSV columns, a TUI `" [review]"`
suffix, and two `eprintln!` warnings (`crates/btctax-cli/src/main.rs:833-845` and `:937-946`), fed
by `IrsPdfReport.form_8283_needs_review` (`crates/btctax-cli/src/cmd/admin.rs:997-1001`). No
`RefuseReason` reads `DonationDetails` — `grep` for `is_review_complete` inside
`btctax-core/src/tax/` returns nothing. `fill_one`'s identity block is `if let Some(details) =
details` (`crates/btctax-forms/src/form8283.rs:442`), so with no details stored the whole Part IV/V
identity block simply goes unwritten and the PDF is produced anyway.

**Consequence:** **files an incomplete required attachment** while Schedule A line 12 claims
$600,000. btctax's own doctrine is collect / refuse / leave blank; here it collects (the CLI command
exists), does not refuse, and prints the claim anyway. The warning is on **stderr with exit 0**, so
any scripted or piped use loses it entirely.

### 3. §170(f)(8) contemporaneous written acknowledgment is never asked, and the paraphrase hid it. (BOTH shapes. CRITICAL for the cash shape, which otherwise files silent and clean.)

**Required:** Schedule A (2024) prints it on the very lines btctax fills —
`design/forms/extract/f1040sa--2024.txt:53-57`, verbatim:

> `Gifts to  11 Gifts by cash or check. If you made any gift of $250 or more, see`
> `Charity      instructions . . .   11`
> `Caution: If you  12 Other than by cash or check. If you made any gift of $250 or more,`
> `made a gift and`
> `got a benefit for it,  see instructions. You must attach Form 8283 if over $500 . . .  12`

§170(f)(8)(A): no deduction for any contribution of $250 or more without a CWA; (f)(8)(B)(ii)–(iii):
the CWA must state whether goods or services were provided and their value.

**btctax does:** nothing — `grep` for `170(f)(8)` and `contemporaneous` across the crates hits only
the *lot-selection* sense of "contemporaneous", never the substantiation sense. And the
transcription dropped the text that names it: `crates/btctax-core/src/tax/printed.rs:1317-1319`
reads `/// L11 — gifts by cash or check.` and `/// L12 — gifts other than by cash or check (includes
crypto donations; Form 8283 over $500).` — the "$250 or more, see instructions" clause and the
entire **"Caution: If you made a gift and got a benefit for it"** side-note are gone. That is the
paraphrase rule's own failure mode: the dropped term became invisible once the line was compressed.

**Consequence:** **overstates the deduction** (and understates tax) for any filer without a CWA, in
full — a $1,000,000 cash gift to a church with no CWA is a $1,000,000 disallowance. And it is the
**cash** shape, the one that today produces a clean packet with **no advisory at all**, that carries
this. A quid-pro-quo benefit (a $1M donor at a church routinely gets something) reduces the
deduction and is never asked either.

### 4. Section B Part I column (i) prints an entry the instructions tell this filer not to make. (Bitcoin shape. IMPORTANT.)

**Required:** i8283 for **both** bound revisions is identical and unambiguous —
`design/forms/extract/i8283--2024.txt:1185-1191` and `i8283--2025.txt:1194-1200`: *"**Column (i).**
Complete column (i), amount claimed as a deduction, **if you are a pass-through entity or a member
of a pass-through entity.** If you are a pass-through entity, enter your share… If you are a member,
enter your share… allocated to you by the pass-through entity."* An individual donating their own
bitcoin is neither.

**btctax does:** writes it unconditionally for every Section-B carrier row —
`crates/btctax-forms/src/form8283.rs:434-436`:

```rust
if let Some(ded) = row.claimed_deduction {
    push_money(&mut w, &mut p, &m.deduction, ded, 5, Some((5, ord)));
}
```

and the value written is `Removal.claimed_deduction`, i.e. the **pre-ceiling** §170(e) figure
(`crates/btctax-core/src/forms.rs:448`), so the emitted packet prints **$1,000,000 in column (i)
beside a Schedule A line 12 of $600,000**. `printed.rs:128-133` argues that L12 ≠ the 8283's
per-donation amounts is legitimate — true for the FMV column (c), and it silently absorbed column
(i), which is literally captioned *"Amount claimed as a deduction"*.

**Consequence:** **files fabricated testimony on the highest-scrutiny line of the highest-scrutiny
form** — an unrequested $1,000,000 entry that contradicts the return's own claimed deduction. This
is the exact class the repo already fixed twice (the Schedule B FBAR pair `3bcf3a0`, the Section A
5a/5b/5c scoping `r3 M-1` at `form8283.rs:508-511`).

★ **Structural note:** the §G-13 field-provenance census cannot see this, and cannot see any defect
of this shape. The census enumerates only the fields the map does **not** fill
(`crates/btctax-forms/forms/2024/f8283.map.toml`, `[census]` section) and asserts the recorded-gap
count (`crates/btctax-forms/tests/field_census.rs:187`, **`const GAPS: usize = 0;`** — measured, and
note this **corrects the brief's shared fact of "6 gap fields incl. Form 8283 line 5a"**, which went
stale when §G-21 closed and the `[line5a]/[line5b]/[line5c]` blocks were added to the map). A
**wrongly filled** cell is outside the census's domain entirely.

### 5. The church is a 50%-org by assumption, never by a collected fact. (Both shapes. IMPORTANT — and the brief's direct question.)

**When does `RefuseReason::NonPublicCharityContribution` fire?** Exactly one place —
`crates/btctax-core/src/tax/return_refuse.rs:841-863` — and only on the **class label the filer
typed**: `Cash30 | OrdinaryProp30 | CapGainProp20`, over `schedule_a.charitable` and
`charitable_carryover_in`. It never inspects a donee.

- **Non-crypto gifts:** the input surface collects **two fields only** — `CharClass` (a free choice
  among all six §170(b) buckets) and `CharAmount`
  (`crates/btctax-input-form/src/spec/sections.rs:872-935`). No donee name, no EIN, no
  organization-type question. A church gift is `Cash60` because the filer said so; a private-
  foundation gift mislabeled `Cash60` sails straight through the guard.
- **Crypto gifts:** the guard is **unreachable by construction**. `crypto_charitable_gifts`
  (`return_1040.rs:642-670`) emits only `CapGainProp30`/`OrdinaryProp50`, hard-coded from
  `leg.term`; its own doc comment states *"Both are 50%-org classes, so `apply_170b`'s '50%-org
  only' precondition holds by construction."* Nothing asks who received the bitcoin.

btctax **does** say so out loud: `Advisory::CharitableDoneeAssumedPublicCharity`
(`crates/btctax-core/src/tax/advisories.rs:146-148`, message at `:362-368`, fired unconditionally
on any donation at `:957-964`) — *"If the donee is a PRIVATE FOUNDATION, the correct treatment is
the 20% ceiling at BASIS (which v1 refuses). Verify who you gave to."* That is an honest gap, not a
silent one.

**Consequence for this vector:** none — a church **is** a §170(b)(1)(A)(i) organization, so the
assumption is right. **Consequence in general: UNDERSTATES tax.** A private-foundation bitcoin gift
gets FMV at 30% instead of basis at 20% (§170(e)(1)(B)(ii), §170(b)(1)(D)), and the refusal designed
to catch it cannot fire on the ledger path. The provenance of "this donee is a 50%-org" is
*nothing* — the third row of the doctrine table.

### 6. The $400,000 carryover exists in memory and reaches no human. (Bitcoin shape. IMPORTANT.)

**Required:** §170(d)(1) — the excess carries to the 5 succeeding years. `is_expired`
(`charitable.rs:43-45`) implements this correctly, and the class/vintage tagging is right.

**btctax does:** computes `CharitableResult.carryover_out` and then reads it in exactly one place:
`apply_carryover_writeback` (`return_1040.rs:2361`), reached from one command,
`crates/btctax-cli/src/cmd/tax.rs:679`. That command **errors out** unless a `ReturnInputs` row
already exists for year+1 (`cmd/tax.rs:670-678`) — and year+1 is 2025, for which btctax has no
full-return tables at all, so the filer must hand-create an input row for a year it cannot compute.
`grep` for `charitable_carryover_out` outside `btctax-core` returns nothing: **no report, no PDF, no
manifest, no advisory ever prints the number.**

**Consequence:** **overstates tax in years 2025–2029** by silently losing $400,000 of deduction
(~$148,000 at 37%) for any filer who does not know to run `--write-carryover` and does not first
fabricate a 2025 input row. Nothing on a 1040 or Schedule A reports a carryover-out, so there is no
form-line defect — which is precisely why only btctax can tell the filer, and it doesn't.

### 7. The §170(b)(1)(C)(iii) basis election is not offered. (Bitcoin shape. MINOR — overstates tax.)

**Required:** §170(b)(1)(C)(iii) lets the filer elect to reduce the contribution by the appreciation
and take the **50%** ceiling instead of 30%.

**btctax does:** `grep` for `170(b)(1)(C)(iii)` returns nothing; `apply_170b` has no election
parameter.

**Consequence: OVERSTATES tax** (taxpayer-favorable omission, so it advises rather than gates —
same posture as `OtherCreditsOmitted`). On this vector, if the donated bitcoin's basis were high the
election would allow up to $1,000,000 in 2024 instead of $600,000. With low basis the default is
better, so it is genuinely a choice and cannot be decided for the filer.

### 8. Small transcription drift in the Section-B metadata. (MINOR / NIT.)

- `crates/btctax-core/src/donation.rs:10,23,27,31,34,37,40` label the appraiser fields **"Part
  III"** and the donee fields **"Part IV"**. On the Rev. 12-2023 form btctax actually files (and on
  12-2025) the appraiser declaration is **Part IV** and the donee acknowledgment is **Part V** —
  the map header says so explicitly (`crates/btctax-forms/forms/2024/f8283.map.toml:2`), and
  `form8283.rs:437-438` already writes the dual form ("Part IV/III", "Part V/IV"). Doc comments that
  disagree with the form are the failure mode the transcribe rule exists to prevent.
- `is_review_complete(B)` requires `donee_ein` but **not** `donee_address`
  (`donation.rs:68-79`), while the form's Part V has an address block and the map maps it
  (`f8283.map.toml` `[section_b] donee_address`). Either require it or record why it is excluded.
  Consequence: a row marked "complete" can still go out with a blank donee address the charity must
  fill by hand — small, but it is the difference between "not filing-ready" being flagged and not.
- The CCA-202302012 appraisal advisory (`crates/btctax-cli/src/render.rs:1054-1072`) is built only
  in the crypto-slice report path (`cmd/tax.rs:522-526`); `advisories_for` in
  `btctax-core/src/tax/advisories.rs` has no appraisal variant, so the **full-return
  `export-irs-pdf`** path never emits it. It does emit the "Section B is NOT filing-ready without a
  signed Part IV/V" warning (`main.rs:838-845`), which covers the substance, so this is Minor.

### What is CORRECT and should not be re-litigated

- The ceiling ladder, the two-term 30% cap, oldest-vintage-first consumption, 5-year expiry, and the
  run-even-in-a-standard-deduction-year rule (Reg. §1.170A-10(a)(2)) — `charitable.rs:106-188`,
  eight KATs at `:209-335`.
- **AMT: no interaction, correctly.** The §57(a)(6) appreciated-property preference was repealed for
  post-1992 contributions; `form6251.rs:107-115` reasons the §170(e) half of line 3 to zero on the
  right ground (bitcoin's AMT basis is its regular basis). No add-back is missing.
- **NIIT: no interaction, correctly.** A charitable deduction is not a §1411(c)(1)(B) properly
  allocable deduction; `grep` finds no charitable term anywhere near Form 8960.
- The Part V donee acknowledgment (receipt date, "unrelated use?", signature/title) is deliberately
  left blank as the **donee's own testimony** — recorded `rule = "artifact"` in the census with the
  right reasoning (`f8283.map.toml`, Part V block). Do not "close" this.
- The `NonCryptoNoncashGift` refusal keys on the **aggregate** noncash (`return_1040.rs:833-860`),
  which is the right predicate; the §G-21 restriction gate keys on Schedule A line 12 and the
  Section split, both of which the packet also keys on. These are well built.

---

## THE SMALLEST THING THAT CLOSES IT

Sequenced. Items 1–3 are the ones that decide whether this vector can be filed.

1. **§170(f)(11)(D) attach-the-appraisal.** Add
   `pub const APPRAISAL_ATTACHMENT_THRESHOLD: Usd = dec!(500000);` beside
   `QUALIFIED_APPRAISAL_THRESHOLD` (`tables.rs:190`), with the instruction sentence as its doc
   comment. Add `Advisory::QualifiedAppraisalMustBeAttached { claimed: Usd }` to the enum in
   `advisories.rs:~148` and fire it from `advisories_for` when the year's **claimed** noncash
   exceeds it — key on `ar.schedule_a.charitable_noncash_12`
   (`return_1040.rs:449`), the same quantity the §G-21 screen and `packet.rs:661` key on, never
   `year_donation_deduction`. Add one line to the packet manifest
   (`cmd/admin.rs:960-975`) naming the appraisal as a filer-supplied attachment so it appears in
   the stapling order. Pair with a **B1 planted-defect test** (assert red when the constant is
   removed).

2. **Promote Section-B incompleteness from warning to refusal.** Add
   `RefuseReason::Form8283SectionBIncomplete` and screen it in `screen_compute_dependent`
   (`return_1040.rs:827`, beside the existing `NonCryptoNoncashGift` block) on
   `claimed_noncash > FORM_8283_THRESHOLD && section == B && rows.any(|r| r.needs_review)`. The
   collector already exists (`btctax reconcile set-donation-details`), so this is refuse-with-an-exit,
   not a dead end. This is the change that makes `is_review_complete` — currently a predicate no
   guarantee depends on — actually hold something. Mutation-verify: delete the screen, the test reds.

3. **Collect §170(f)(8).** Add `charitable_acknowledgments_on_file: Option<bool>` to
   `ReturnInputs` (beside `donations_had_restrictions`, which it exactly mirrors: one return-level
   universal, `PerYear` durability), a `QuestionId` entry, and a `RefuseReason`
   `CharitableAcknowledgmentUnanswered`. **Live when any single gift — cash or noncash, user-entered
   or ledger — is ≥ $250.** `None` ⇒ refuse; `Some(false)` ⇒ refuse (the deduction is denied, so the
   figure btctax computed is too large — same structure as the restriction gate at
   `return_1040.rs:2137-2150`); `Some(true)` ⇒ proceed. Default-to-NO so an omission fails closed.
   While in the file, **restore the verbatim line text** at `printed.rs:1317-1319`, including the
   "$250 or more" clause and the "Caution: If you made a gift and got a benefit for it" side note.

4. **Stop writing Section B column (i).** In `form8283.rs:434-436`, gate the `push_money` on a
   pass-through fact rather than on `claimed_deduction.is_some()`. Two options, in preference order:
   (a) leave it **blank** for every btctax filer, since btctax models no pass-through entity — this
   is already the census's own reasoning for the header entity-name/TIN cells and the family-PTE box
   (`f8283.map.toml`, header block), so it is consistent, needs no new input, and the cells move from
   *mapped* to `rule = "unmodeled"` in the census; or (b) collect a pass-through-member flag and
   write the allocated share. **(a) is the smallest and the one the census already justifies.** Pair
   with a planted-defect test asserting the cell is absent from the emitted PDF for an individual
   filer.

5. **Make the 50%-org status collected, not assumed.** Add
   `donee_is_public_charity_170b1A: Option<bool>` to `DonationDetails`
   (`donation.rs:17-48`) — it is already the per-donation side-table and already holds the donee's
   name/EIN/address, so this is one field and one CLI flag on `set-donation-details`. Route
   `Some(false)` into the **existing** `RefuseReason::NonPublicCharityContribution`, which today
   cannot fire on the ledger path at all. Keep `CharitableDoneeAssumedPublicCharity` as the `None`
   message. Do **not** add a §170(b)(1)(A) organization-type taxonomy — one boolean with the
   statutory cite in its doc comment is what the form's own Part V affirmation asks
   (*"acknowledges that it is a qualified organization under section 170(c)"*).

6. **Print the carryover-out.** `CharitableResult.carryover_out` is already computed on every run;
   surface it in the `report --tax-year` render (the struct is
   `crates/btctax-cli/src/cmd/tax.rs:345-380`) as a per-class, per-vintage line, and repeat it in
   the `export-irs-pdf` stderr block beside the other §170 warnings. No new computation — this is
   a display of a value that already exists and currently reaches nobody. Separately, relax
   `write_back_carryover`'s hard requirement that a year+1 `ReturnInputs` row pre-exist
   (`cmd/tax.rs:670-678`) **only** if the shadowing concern documented there can be met; otherwise
   printing it is sufficient and the write-back stays as is.

7. **Doc-comment fixes** (one commit, no behavior): `donation.rs` Part III/IV → **IV/V**; add
   `donee_address` to `is_review_complete(B)` or record why not; refresh the stale `[census]` prose
   in `f8283.map.toml` that still describes 5a/5b/5c as a gap after §G-21 closed them.

8. **File as a follow-up, owning phase = whenever §170 is next opened:** the §170(b)(1)(C)(iii)
   basis election as a new `Advisory` (overstates-tax direction, so it advises and never gates).

---

## WHAT I AM NOT SURE OF

- **Which quantity §170(f)(11)(D)'s $500,000 keys on.** The instruction says *"claiming a deduction
  of more than $500,000 for an item (or group of similar items)"*. On this vector both readings
  cross the line ($1,000,000 contributed, $600,000 claimed after the ceiling), so it does not matter
  here — but a $700,000 gift at a $1,000,000 AGI would claim $300,000 in year 1 and carry $400,000,
  and I could not settle from the instructions whether the appraisal must be attached to the year-1
  return. `i8283--2025.txt:1389` (*"property for which you claimed a deduction of more than
  $500,000"*) leans toward the claimed figure; §170(f)(11)(D)'s own text leans toward the
  contribution. **Adjudicate against the statute before implementing item 1's predicate.**
- **The §170(b)(1)(G)(iii) vs Pub. 526 Worksheet-2 ordering.** btctax computes the 50%-tier ceiling
  as `50%·AGI − allowed_cash` (`charitable.rs:153`). §170(b)(1)(G)(iii)(II) reads as *"reducing the
  contribution base by the aggregate amount of contributions of cash"*, i.e. `50%·(AGI − cash
  contributed)` — which differs whenever cash is capped at 60% or the cash carryover is partly
  disallowed. `legal/text/irs-publications/Pub526_Charitable_Contributions.txt` is archived and I
  did not work its worksheet through. **Latent on this vector** (one class only). Worth its own KAT
  in a mixed-gift year; I would not open it as part of this work.
- **Whether refusing on an unanswered §170(f)(8) CWA (item 3) is too heavy.** It would gate every
  itemizing filer with a $250 gift. The alternative is `Some(false)` ⇒ refuse, `None` ⇒ advisory. I
  proposed refuse-on-`None` because it matches `donations_had_restrictions`, `HsaActivityUnanswered`
  and the "default to NO so omissions fail closed" rule, but the scoping is an owner's call.
- **Whether an unrequested-but-true column (i) entry (item 4) is a filing defect or cosmetic.** I
  read it as a defect under this repo's own "an entry is testimony" rule, and the fact that the
  printed $1,000,000 contradicts the return's own $600,000 makes it worse than a neutral extra mark
  — but it is the same species of practitioner judgment §G-18 flagged for the 1040 line-7 checkbox.
- **Whether `Removal.appraisal_required` (`state.rs:207`) has any live consumer beyond the
  `BlockerKind::QualifiedAppraisalNote` advisory** (`project/fold.rs:1384-1390`, severity Advisory,
  pinned never-Hard at `state.rs:345-347`). It is a user-set bool on the event and the fold computes
  its own threshold independently; I did not trace whether the two can disagree and whether anything
  would notice.
- **Whether the TY2024 bundled PDF's column (i) caption matches the 12-2025 extract.** I verified the
  *instructions* for both TY2024 and TY2025 say the same thing (`i8283--2024.txt:1185-1191`,
  `i8283--2025.txt:1194-1200`), and the map header states the bundled asset is Rev. 12-2023 with
  Section B field names identical to 12-2025 (`f8283.map.toml:1-8`). There is no
  `design/forms/extract/f8283--2024.txt`, so I could not read the 12-2023 form's own text layer.
  **Extract it before acting on item 4.**
