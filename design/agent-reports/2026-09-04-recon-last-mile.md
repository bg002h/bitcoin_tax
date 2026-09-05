# Recon — the last mile: between "correct numbers on a PDF" and "a return the IRS has accepted"

**Scope:** recon only, no code changed. Repo read at `main` (HEAD `945d1ac2`, working tree has
only `CONTINUITY.md` modified per the pre-existing git status). Method: grep/read across
`crates/btctax-{core,forms,cli}`, `LIMITATIONS.md`, `NOTICE`, `design/SPEC_full_return.md`, and
`design/direction/FILING-READINESS-PLAN.md` (the most recent merged cycle, "filing readiness",
`945d1ac2`) — plus a handful of targeted irs.gov searches for facts the repo doesn't need to state
because it is print-and-mail only. Every claim below is grep/read-verified against current source;
line numbers are cited so they can be re-checked after the next merge.

**Headline finding:** the branch that just merged (`feat/filing-readiness`, `945d1ac2`) is
entirely about the *arithmetic* half — getting the right numbers onto the right boxes. It never
touches signature, payment mechanics, identity data for filing, state returns, or record
retention. Those six areas are **not tracked anywhere** — no FOLLOWUPS entry, no design doc, no
SPEC section beyond a one-line exclusion. This recon is genuinely surveying unaddressed territory,
not re-deriving something already scoped.

The product's own framing (`LIMITATIONS.md:3`) already states the outer boundary honestly:
**"Federal only · Print-and-mail (no e-file)."** Nearly everything below follows from taking that
sentence seriously rather than as a footnote.

---

## 1. Signature and jurat — PARTIAL, with an explicit DELIBERATE REFUSAL at the core

**The signature itself: DELIBERATELY REFUSED**, and said plainly in three places:
- `NOTICE`: *"No Paid Preparer is identified and no PTIN is filled... this is a self-prepared
  return. The signature on it is yours alone."*
- `crates/btctax-cli/src/cmd/admin.rs:410-414` (`hand_marks`, function starts line 360): the
  packet's `manifest.txt` unconditionally lists *"Form 1040 page 2 — the signature block: your
  signature, the date, your occupation, and the Identity Protection PIN if the IRS issued you
  one... A return is not filed until it is signed under penalties of perjury (§6065), and no
  software may sign it for you."*
- This is the shipped fix for FILING-READINESS-PLAN item **N4** ("unsignalled hand-marks") — the
  mechanism is real code (`hand_marks`/`hand_marks_block`, `admin.rs`), not aspirational prose, and
  it is *conditioned*: each mark only appears when the box is actually blank in that packet
  (`admin.rs:352-359` for the digital-asset question, `:361-368` for the no-Schedule-D box,
  `:376-388` for a Form 8283 Section B appraiser/donee signature).

**What IS collected and printed (MODELLED):**
- **Occupation** — `ReturnHeader.taxpayer.occupation` / `.spouse.occupation`
  (`crates/btctax-core/src/tax/packet.rs:199-200`), filled at
  `crates/btctax-forms/src/form1040_full.rs:445,447`.
- **Taxpayer's IP PIN** — a dedicated `IpPin` newtype, 6-digit validated, masked `Debug`
  (`packet.rs:159-183`), filled at `form1040_full.rs:449-450`. The doc comment names the real
  consequence: *"A paper return that omits an issued IP PIN is REJECTED or delayed"*
  (`packet.rs:159,367`).
- **Third Party Designee: ABSENT, and named as such in the field census** —
  `crates/btctax-forms/forms/2024/f1040.map.toml:260-264`: all five cells ("Do you want to allow
  another person to discuss this return with the IRS?" Yes/No, name, phone, PIN) are
  `rule = "unmodeled"`, reason *"btctax grants no third-party authorisation and collects no
  designee."*
- **Spouse's IP PIN — ABSENT, and flagged as a genuine capture gap, not a refusal.**
  `f1040.map.toml:267-276`: *"ReturnInputs captures the taxpayer's (f2_34, mapped) but not the
  spouse's... ★ Asymmetric with the taxpayer's; a return omitting an issued spouse IP PIN is
  rejected."* `packet.rs:369` repeats it: *"The spouse's IP PIN is not captured by `ReturnInputs`
  at all — a capture gap, recorded in LIMITATIONS rather than fabricated."* This is a real,
  named hole in the input surface, not a policy choice — the comment says "closing it is one
  optional field on ReturnInputs" (`f1040.map.toml:274`).
- **Phone / email — ABSENT, one deliberate, one not.** `f1040.map.toml:277`: *"Filer's phone
  number — optional contact detail; ReturnInputs collects no phone number."* `:278`: *"email...
  collecting one would add PII the return does not need"* — the email omission reads as a
  considered privacy choice; the phone omission reads as a plain gap (nothing marks it
  deliberate the way the email line does).

**Net:** for a print-and-mail filing, the load-bearing pieces (occupation, taxpayer IP PIN, and
the loud "you must sign this" instruction) are handled about as well as software safely can — the
wet signature is correctly left to the human, not faked. The two real gaps are the spouse's IP PIN
(silent rejection risk on MFJ if the IRS issued one) and third-party designee (a feature gap, not
a defect — it's an authorization the filer may not even want).

---

## 2. Attachments and statements — MODELLED for the cases the repo has already found; several classes never come up because the product is federal/individual/no-trust scope

**MODELLED, with real code (not just prose):**
- **>4 dependents continuation statement** — `crates/btctax-core/src/tax/dependents_statement.rs`.
  A genuine transcription of the IRS-mandated statement (i1040gi: *"include a statement showing the
  information required in columns (1) through (4)"*), one predicate (`more_than_four_dependents`)
  drives both the checkbox and the statement so they can't diverge (`dependents_statement.rs:44-52`).
- **Form 8283 (noncash donations)** — `crates/btctax-forms/src/form8283.rs`, with Section B
  (>$5,000 crypto donation) triggering the appraiser/donee-signature hand-mark above. The qualified-
  appraisal-attached threshold (§170(f)(11)(D), >$500,000 claimed) is explicitly **advised, not
  refused**, because btctax "cannot produce an appraisal, only a qualified appraiser can"
  (`LIMITATIONS.md:349`); the manifest names it as an item the filer must supply.
- **Form 8275 disclosure** (Defensive Filing / Approach-B) — a real filled PDF, Attachment
  Sequence No. 92 (`crates/btctax-forms/tests/census.rs:2`); its Part II narrative had a
  now-fixed truncation defect disclosed in `NOTICE`.
- **§170(f)(8) charitable acknowledgment** — btctax refuses/advises rather than fabricates; the
  form's own instruction is quoted back to the filer: *"Don't attach [the CWA] to your return.
  Instead, keep it for your records"* (`return_inputs.rs:858`, `questions.rs:1314`,
  `return_1040.rs:2528`) — a correct instance of transcribing the form rather than guessing.
- **Charitable carryover to next year** — computed and surfaced via `--write-carryover`
  (FILING-READINESS-PLAN item P6), so the filer is at least told the carryforward exists.
- **>500K claimed appraisal** — advisory only (above), correctly, since btctax cannot generate a
  third party's appraisal.

**ABSENT / never reached, because the classes don't exist in scope:**
- **Broker-statement substitute for 8949 detail (the "Exception 2" attach-a-statement-in-lieu-of-
  listing-every-row" path)** — not applicable: btctax always fills full per-lot 8949 rows itself
  from the reconciled ledger; there is no code path that produces a substitute statement instead.
  Not a gap so much as a different design (btctax generates the detail rather than deferring to a
  broker's own listing).
- **Election statements outside the ones already modelled** (e.g. a written §475 mark-to-market
  election, a formal specific-identification election letter under §1.1012-1) — no grep hits
  anywhere in `crates/`. Not currently a live concern given the product's scope (no trader
  mark-to-market election is modelled), but worth naming as a class the product has never had to
  answer, because "specific identification" here means the *lot-selection* mechanism
  (`whatif.rs:39`), not a filed election statement.

---

## 3. Payment and refund — PARTIAL: the arithmetic is done; every payment MECHANISM is absent, one by explicit IRS-sanctioned default

**MODELLED:**
- 1040 lines 33 (total payments), 24 (total tax), 34/35a (overpayment → refund), 37 (amount owed)
  are computed end-to-end (`return_1040.rs:1445-1448,2036-2037`) and filled
  (`form1040_full.rs:157-170,291-321`).
- The whole-dollar rounding election is applied consistently so the on-screen "amount you owe"
  matches the filed PDF to the dollar (`LIMITATIONS.md:74-80`).

**ABSENT, each with a specific IRS-facing consequence:**
- **Direct deposit (lines 35b–35d: routing number, account type, account number) and Form 8888
  (split refund)** — `f1040.map.toml:245-249`, all `unmodeled`: *"btctax collects no bank details;
  the refund amount is printed and the filer supplies deposit instructions themselves."* Documented
  user-facing consequence in `LIMITATIONS.md:222`: *"you will receive a paper check."*
- **Line 36 (overpayment applied to next year's estimated tax)** — `f1040.map.toml:250`,
  `unmodeled`: *"btctax offers no such election; the whole overpayment is shown as refundable on
  line 35a."* Confirmed independently in `return_1040.rs` comments ("L36 apply-to-next pinned
  0/blank in v1").
- **Line 38 / Form 2210 (estimated tax penalty)** — `f1040.map.toml:253`, `unmodeled`:
  *"btctax emits no Form 2210 and computes no §6654 underpayment penalty."* ★ This one has a real
  mitigating fact, correctly cited in the map's own reason: the i1040 instructions make this line
  **optional** — *"the IRS figures and bills the penalty if the filer leaves it blank"* — so a
  blank here is the instructions' own default, not a silent product gap. Confirmed against
  irs.gov: Form 2210 is voluntary in the ordinary case (the IRS will "figure any penalty you owe
  and send you a bill" — [IRS Topic 306](https://www.irs.gov/taxtopics/tc306), [2025 Instructions
  for Form 2210](https://www.irs.gov/pub/irs-pdf/i2210.pdf)) — but filing it yourself is still
  sometimes *required* (e.g., requesting a penalty waiver, or the annualized-income method), a
  case btctax has no way to detect or serve since it does no quarterly-income modelling.
- **Form 1040-V (payment voucher)** — no filler exists at all, and unlike the lines above it is not
  even named as an exclusion in `LIMITATIONS.md` or `SPEC_full_return.md` — it simply never comes
  up. A filer paying by check needs this to remit payment correctly; btctax's packet gives them the
  "amount you owe" figure but not the accompanying voucher.
- **Form 1040-ES / estimated payments for the *next* year** — no code at all. Not mentioned as an
  exclusion; simply outside anything the product currently reasons about. This matters most for
  exactly the population the product serves (large one-off crypto gains), since a filer who owes a
  large amount this year is a strong candidate for owing an estimated-tax penalty *next* year if
  they don't start paying quarterly — a forward-looking consequence the product never surfaces.

---

## 4. Prior-year and identity data — MODELLED where the product's own filing channel needs it, ABSENT where only e-file would need it

Because btctax is explicitly print-and-mail only (`LIMITATIONS.md:3`, `SPEC_full_return.md:79`
"e-file" is listed among out-of-scope items, `SPEC_full_return.md:578` again), the e-file-specific
identity data (prior-year AGI, self-select PIN, ERO authorization) is **correctly N/A rather than
a gap** — the IRS requires prior-year AGI / self-select PIN specifically to authenticate an
*electronic* signature ([IRS: Self-Select PIN Method](https://www.irs.gov/e-file-providers/self-select-pin-method-for-forms-1040-and-4868-modernized-e-file-mef)); a wet-ink paper signature has no such
requirement. This is a case where the product's stated scope decision (no e-file) genuinely
retires an entire category of last-mile work rather than deferring it.

**SSN/ITIN — MODELLED, fail-closed.** `ReturnHeader::build` (`packet.rs:365-`) canonicalizes every
SSN (taxpayer, spouse, every dependent) via `Ssn::canonical` and **refuses the whole return** if
any fails (`HeaderError::Ssn`). No fabrication path exists — this is the "an entry is testimony"
doctrine applied correctly to identity fields.

**IP PIN — see §1 above** (taxpayer: modelled; spouse: a named, real capture gap).

**Prior-year figures actually needed for *this* return (not e-file, but substantively — carryover
verification)** — capital-loss carryforward, charitable carryforward, QBI carryforward — are
**collected as filer-declared inputs** (`ReturnInputs`), not fetched or cross-checked against an
actual prior-year return. That is consistent with the product never having access to a prior
year's filed PDF, but it means a filer who mis-transcribes last year's carryforward number gets no
independent check — a correctness risk that's adjacent to, but distinct from, the "last mile"
question asked here.

---

## 5. State returns — ABSENT by explicit, repeated statement; usability is real but partial

**Explicitly, repeatedly, and correctly disclosed as out of scope — not silently missing:**
- `LIMITATIONS.md:302`: *"**State and local returns** — federal only."*
- `LIMITATIONS.md:3`: *"Federal only"* in the version banner itself.
- `SPEC_full_return.md:79`: *"state returns"* listed among "Out of scope (v1)".
- `SPEC_full_return.md:578`: *"state returns, e-file, non-crypto Schedule C/E/F"* under
  "unrepresentable / documented-out (no input; would refuse if captured)".

No grep hit anywhere in `crates/` treats "state" as anything other than (a) the mailing address
state field, or (b) SALT — state/local tax paid as a Schedule A *input* to the federal return. There
is no state-tax computation, no state form, and no state e-file of any kind.

**Is a federal-only product usable in practice?** Partially, and it depends heavily on the filer:
- **No state income tax** (AK, FL, NV, NH¹, SD, TN, TX, WA, WY) — federal-only is fully sufficient;
  these filers have no state income-tax return to file at all.
- **Every other state** — the filer must separately prepare and file a state return, almost all of
  which *start from federal AGI or federal taxable income* and require the federal figures as
  inputs. Since btctax computes and prints those figures correctly (subject to the arithmetic
  caveats already tracked in FILING-READINESS-PLAN), a filer in a state-tax state can use btctax's
  federal output as the starting point for a state return prepared elsewhere (by hand, a state
  free-file tool, or a preparer) — but btctax gives them zero help with state-specific treatment of
  crypto (states vary on conformity to federal capital-gains treatment, some states tax crypto
  differently, and community-property states raise basis/allocation questions federal-only
  software cannot see). ¹NH taxes interest/dividends only, not wages/capital gains, as of recent
  years — worth a specific check for the vector's actual year.
- **Net:** the disclosure is honest and unambiguous, but the practical burden this places on a
  crypto filer specifically (most of whose home states DO tax capital gains) is larger than
  "federal only" reads at first glance, because crypto-heavy income concentrates state tax
  liability that the filer must reconstruct entirely outside the tool.

---

## 6. Record retention and the audit trail — ABSENT as explicit filer guidance; PARTIAL as a byproduct of the vault's own design

**No explicit record-retention guidance anywhere for the filer.** Zero hits in `README.md`,
`LIMITATIONS.md`, or `NOTICE` for retention duration, "Publication 552," "keep your records for,"
or similar. The only "keep for your records" language that exists is the IRS's own phrase quoted
back verbatim where a specific *worksheet* says not to attach itself (§170(f)(8) CWA:
`return_inputs.rs:858`, `questions.rs:1314`, `return_1040.rs:2528`; also the capital-loss
carryover worksheet, `capital_loss_carryover_check.rs:198`, `registries.rs:423`) — those are
per-form transcriptions, not a general audit-preparedness statement. Confirmed against irs.gov: the
general rule is to keep records **3 years from the date filed** (the standard assessment
limitations period), longer for basis-relevant records (property, securities) and for the whole
holding period plus that 3 years for lot basis specifically — see [IRS Publication
552](https://www.irs.gov/pub/irs-pdf/p552.pdf) and [IRS recordkeeping
guidance](https://www.irs.gov/businesses/small-businesses-self-employed/recordkeeping). Because
crypto basis can trace back years before disposal, this is a case where the retention window that
actually matters (holding period + 3 years, potentially a decade or more for a long-held lot) is
longer and less obvious than the generic "3 years" rule, and nothing in the product says so.

**What functions as an audit trail, without being labeled one:**
- `export-snapshot` writes the decrypted SQLite database plus CSVs (the deliberate plaintext
  exception, `cli.rs:122`) — this is the actual lot-by-lot, event-by-event history that would
  support a disposal's basis and holding period under audit.
- The `manifest.txt` per filed packet, `report`'s provenance printing (which *source* produced each
  figure, "so a reviewer can audit" per `resolve.rs:22`/`render.rs:1451`), and the whole
  fail-closed/`NotComputable` design mean the derivation of every figure is traceable in principle.
- **But none of this is packaged, named, or explained to the filer as "this is what to keep and for
  how long if the IRS asks."** A filer who deletes their vault after filing, or who never runs
  `export-snapshot`, has no product-provided fallback — the audit trail exists only as long as the
  filer independently decides to preserve it, with no prompt telling them to.

---

## Summary table

| Area | Verdict |
|---|---|
| Signature (wet ink) | **DELIBERATELY REFUSED** — correct; software cannot sign (§6065) |
| Occupation, taxpayer IP PIN | **MODELLED** |
| Spouse IP PIN | **ABSENT** — named capture gap, real MFJ rejection risk |
| Third-party designee | **ABSENT** — feature gap, not a defect |
| Phone / email | **ABSENT** — email deliberate (PII minimization); phone just missing |
| Dependents statement, Form 8283, Form 8275, §170(f)(8) CWA handling | **MODELLED** |
| Broker-statement-substitute 8949, formal election statements (§475, etc.) | **ABSENT** — not currently a live product concern given scope |
| Refund/owed arithmetic | **MODELLED** |
| Direct deposit, Form 8888 | **ABSENT** — paper check only |
| Line 36 (apply to next year) | **ABSENT** |
| Form 2210 / line 38 | **ABSENT** — mitigated: blank is the IRS's own instructed default |
| Form 1040-V | **ABSENT** — not even named as an exclusion |
| Form 1040-ES / next-year estimates | **ABSENT** — highest-value gap given the product's typical filer (large one-off gains) |
| SSN/ITIN | **MODELLED**, fail-closed |
| Prior-year AGI / self-select PIN | **N/A** — correctly retired by the no-e-file scope decision |
| Carryforward figures (loss/charitable/QBI) | **MODELLED** as filer-declared input, not cross-checked against an actual prior return |
| State returns | **ABSENT** — explicitly and repeatedly disclosed; usable as-is only in no-income-tax states |
| Record retention guidance | **ABSENT** as filer-facing advice |
| Audit-trail data | **PARTIAL** — exists via `export-snapshot`/provenance printing, but unpackaged and unprompted |

## What is genuinely new here versus what the repo already knows

FOLLOWUPS.md and the merged filing-readiness plan contain zero hits for: 1040-V, 1040-ES, Form
2210, direct deposit, third-party designee, prior-year AGI, self-select PIN, state returns (as a
computation), or record retention. The only two items above that already have first-class code
*and* a named plan entry are the signature hand-marks (N4, shipped) and the spouse IP PIN /
third-party designee census entries (documented in the map file but not tracked as a FOLLOWUP or
SPEC item to close). Everything else in this report — 1040-V, 1040-ES, direct deposit, state
returns' practical burden, and record-retention guidance — has no tracking artifact anywhere in
the repo today.
