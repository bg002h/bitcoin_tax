# Field provenance — every box on every form must be accounted for

**Written 2026-07-30**, from a working session with the owner. Captured on disk **before** measuring
or consulting, because the reasoning below is the input to both and existed only in a conversation.

**Status: DESIGN NOTE, nothing built.** The measurement in §5 is done; the design in §3–§4 is
proposed and unreviewed. `design/forms/LABEL_READER.md` covers the *label* census (built, increment
1 of 2); this covers the *field* census, which supersedes it as the accounting unit.

---

## 1. How we got here — the label census is keyed on the wrong thing

The label census enumerates a form's printed line labels and asks whether each is accounted for. It
works, and increment 1 built it (45 → 50 labels on Schedule 1-A, three box-assignment models tried,
two refuted by opening the PDF).

**Then the owner pointed at line 22.** Lines 22a/22b are not single entries: the form prints three
columns, and the AcroForm carries six fields for the two rows. The VIN in column (i) is one field
with `maxlen=17`, drawn as a comb of per-character cells. Columns (ii) and (iii) are single boxes.

The `label-proof` render made the defect visible: **columns (ii) and (iii) of row 22a both printed
`22a`** — the census could not tell them apart. And the form addresses cells by column *in its own
words*: line 23 reads *"Add lines 22a and 22b, **column (iii)**"*.

★ **This is not a line-22 quirk.** `f8949` has 244 fields and `f1040` 141 (TY2024). At line
granularity the largest forms in the set would get the weakest check.

## 2. ★★★ The unit of forgetting is a BOX, not a line

A field census is strictly stronger than a label census, on every axis that matters:

| | label census | field census |
|---|---|---|
| enumeration | parsed off the page — right-alignment traps, comb quirks, three failed join models | **the AcroForm list is exhaustive and authoritative**; no parsing at all |
| line 22 | `22a(ii)` and `22a(iii)` indistinguishable | **distinct FQNs — the problem vanishes by construction** |
| checks | a list we derived | **the `.map.toml` the emitter actually consumes** |

★ **The label reader is not wasted — it changes job.** Fields become the accounting unit; labels
become the human-readable *address* that makes a 244-row census reviewable and the proof sheet
legible. `22a(iii)` means something to a person; `Line22b[0].f2_06[0]` does not.

### The half that already exists, and the half that does not

`btctax-forms/src/verify.rs::no_unmapped_filled` asserts **every field carrying a value is in the
authorised set** — it stops us writing where we should not.

**Nothing asserts the other direction:** every field is either mapped, or deliberately not ours.
One direction stops stray writes; the missing one stops silent omissions — and omission is the
direction that costs a filer money.

## 3. ★★★ The provenance taxonomy (owner, 2026-07-30)

Every AcroForm field must resolve to exactly one of these. Only the last is a defect.

| # | provenance | meaning |
|---|---|---|
| 1 | **Filled** | we compute or collect a value and write it |
| 2 | **Declined** | the filer **was asked and said no**, so the line is correctly blank — carries a pointer to the question and its answer |
| 3 | **Not applicable** | structurally cannot apply (an MFJ-only line for a single filer) |
| 4 | **Not ours** | another party or the filer fills it by hand — signatures, preparer block, the donee/appraiser declarations on Form 8283 |
| 5 | **Refused** | we cannot answer it and must refuse rather than guess |
| 6 | **★ NOTHING EVER DECIDED** | never collected, never asked, never modelled — **the only bug**, and today indistinguishable from 2–4 |

### ★★ A recorded "no" is provenance for US, not testimony on the return

This is the distinction that keeps the design honest, and it is where it could most easily go wrong.

Answering "no" to an interview question **does not put testimony on the form.** The line stays
**blank**, and we must never print `0` to show that the filer answered. The answer lives in our
records; it makes the omission *informed and auditable*; the form says nothing, because nothing is
what the filer is entitled to say.

★ That is `CLAUDE.md`'s rule exactly — *does the silence ASSERT, or FORGO?* — and it is why
**§G-11 blocks the honest version of this**: `fmt_money(Usd) -> String` cannot express blank, so
even a correctly-recorded "no" renders as `0`.

## 4. ★★★ Invert it: the FORMS derive the INTERVIEW

Today the question list is hand-written and hopes it covers the forms. Inverted, every field must
resolve to one of §3's six, so **an unaccounted field is either a missing question or a missing map
entry** — and *"what should the TUI ask?"* stops being a judgement call.

★ The owner's leverage observation: **one y/n question can account for many lines.** "Did you pay
qualified passenger vehicle loan interest?" → no → Schedule 1-A **lines 22–30, nine lines**, all
lawfully blank with recorded provenance. **The question set is far smaller than the field set**, and
§5's job is to find out by how much.

### What already exists — P9's question registry (`btctax-core/src/tax/questions.rs`)

Substantially more than expected, and it must be built on rather than duplicated:

- `FormQuestion` owns the prompt (phrased as the FORM phrases it), the refusal, the refusal detail,
  and **the liveness predicate — explicitly the only copy in the codebase**.
- The answer is **`Option<bool>`**, so *unanswered* is already distinct from *no*.
- Both doctrine classes are implemented: **class (A) declarations** (14: foreign accounts, foreign
  trust, HSA, dual-status, …) where unanswered + live ⇒ **REFUSE**; and **class (B) skippables**
  (blindness ×2, sales-tax election, DOB ×2, date of death ×2) where skipping **forgoes** a benefit
  and silence is lawful.

**Gaps for this design:** (a) no question exists for whole form *sections* — the car-loan example is
TY2025 OBBBA work not yet built; (b) **nothing links field → question**, so no blank can point at
the answer that explains it; (c) §G-11.

## 5. ★★ Question state must be visible AT SCALE — and today it is not

Owner's point: with a large question set, knowing **which are answered and which are not** is
necessary. Verified findings:

- `cmd/answer.rs::live_questions(ri) -> Vec<Ask>` **does** enumerate every live question across both
  classes. The primitive exists.
- **But `return_refuse.rs::screen_inputs` returns `Option<Refusal>` — the FIRST unanswered live
  declaration only.** At ~21 questions that is tolerable; at section-level questions across 16 forms
  it becomes *answer one, re-run, hit the next* — **N unanswered ⇒ N round-trips**, and the filer
  never sees how much is left.

**Four states must be distinguished, or a long list is noise:**

| state | cost |
|---|---|
| not live | silent, correct |
| live + answered | done |
| live + unanswered, **class (A)** | **blocks the return** |
| live + unanswered, **class (B)** | **forgoes money**, lawfully |

★ A progress bar would flatten that last row and must not: skipping the car-loan question is not an
error, it is a decision to leave a deduction on the table — and the filer deserves to see that
**before** filing, not never.

★★ **It is one surface, readable from either end.** Each unanswered question names the fields it
would account for; each unaccounted field names the question that would explain it.

## 6. THE MEASUREMENT — TY2024, where the maps are complete

TY2025 has only 5 map files to TY2024's 15 (TY2025 `FullReturnParams` land last, by design), so
TY2024 is the honest year to measure.

```
form            fields  mapped  UNACCOUNTED
f1040             141      87      54
f1040s1            69      12      57
f1040s2            60       6      54
f1040s3            39       7      32
f1040sa            37      23      14
f1040sb            72      68       4
f1040sc           105      13      92
f8275              95      53      42
f8283             117      59      58
f8949             244     238       6
f8959              26      19       7
f8960              38      16      22
f8995              33      20      13
schedule_d         55      27      28
schedule_se        27      14      13
─────────────────────────────────────
TOTAL           1158     662     496
```

★★ **496 is NOT a defect count.** It is the number of boxes with **no recorded decision** — nobody
wrote down whether we fill them, the filer does, or they do not apply. Under §3 that is category 6
mixed with categories 2–4, and the two are indistinguishable today. **The open work is classifying
them, which is recording decisions, not writing code.**

★ Two measurement errors were made and caught before reporting, and they are worth keeping as a
caution: the first pass counted only the **TY2025** maps (deliberately incomplete, giving a wildly
inflated 197 unaccounted for f1040), and the second missed `f8283`'s `Form8283[0]` FQN prefix and
reported **0 mapped** for a form with 59. Both were found by re-running, not by anything structural.

## 6a. ★★ THE CLASSIFICATION — what the 496 actually are (f1040 TY2024, 54 of them)

Joined field→label with `xtask label-boxes` (the address map: FQN → the line it sits on). Three
regions fall out, and they are **not** three equal thirds:

| region | n | what it is | likely provenance |
|---|---|---|---|
| **header** (above line 1) | 8 | **foreign country / province / postal code**, presidential-campaign checkbox, `f1_01`–`f1_03` | *not applicable* for most filers — but the foreign-address gap is real: **a filer abroad cannot have their address printed**, and today that is invisible |
| **trailer** (below the last numbered line) | 16 | third-party designee (name, phone, PIN, yes/no), signature block (IP PINs, phone, email), preparer block (name, PTIN, firm, EIN, address, self-employed) | almost entirely **not ours** — the filer signs by hand, the preparer block is another party's |
| **body** | 30 | checkboxes and sub-boxes on lines we *partly* map — line 16's 8814/4972/other, line 26, sub-lines 1b–1h, 5b, 6b, and 35a's bank routing/account | **the interesting residue** — a mix of "needs a question", "not applicable", and possibly category 6 |

★ So roughly **45% is header/trailer** and disposed of by a handful of blanket decisions, and
**~55% is per-line body** — the part that needs real classification. If that ratio holds across the
15 forms, the genuine work is ≈270 fields, not 496.

★★ **The practice already exists in prose.** `f1040.map.toml` carries, as a comment: *"the spouse's
IP PIN cell exists (f2_36) but ReturnInputs does not capture one, so it is left BLANK, never
guessed."* That is precisely a §3 provenance record — **written by hand, in a comment, unenforced,
and invisible to any check.** §G-13 is about making that structural and total, not inventing it.

### ★ A defect this surfaced in the label reader (increment 1)

**Boxes below the last numbered line are all attributed to it** — the 16 trailer fields came back
labelled `37`. The "last label at or above the box's centre" rule has **no upper bound**, so an
entire region with no line numbers collapses onto the final label. The join needs a **region**
concept — *header* (before the first label), *body* (between labels), *trailer* (after the last) —
and the trailer must be its own category rather than a mis-attribution. Filed here rather than fixed,
because it changes what a "label" is and that is the ⑥ consult's subject.

## 6b. ★★★ THE DECLINE RECORD MUST BE CRYPTOGRAPHICALLY DELETABLE (owner, 2026-07-30)

The ⑥ consult was asked whether recording "the filer was asked and said no" is a **liability**
(a discoverable record of what was asked and declined) or a **protection** (evidence of diligence).
**The owner supplied a third answer, and it is better than either:** make the record
**cryptographically deletable**, so its lifetime is the *filer's* decision — kept while it is useful,
destroyed after filing by discarding the key.

### What already exists — privacy is DONE

- The whole vault is **one encrypted SQLite image**: `vault.rs::save()` serialises the DB, wraps it
  in a versioned blob, and encrypts under a passphrase-protected OpenPGP cert (`crypto.rs`,
  sequoia). The `.tmp`/`.bak` that `atomic_write` touches are **ciphertext**.
- `SecretBuf` **mlocks and scrubs** every plaintext copy on the save path (`zeroize`).
- So interview answers held in the vault are **already private at rest**. The honest bound is
  documented in-place: while the vault is OPEN, the live SQLite connection keeps plaintext in its own
  heap for the session (accepted, stated, not hidden).

### ★★ What does NOT exist — and why "destroy the passphrase" is not enough

**There is exactly ONE cert per vault.** No envelope encryption, no per-item data keys, no shredding
path. So "destroy the passphrase after filing" destroys **the entire vault**, and that is not a
lawful-outcome-preserving move:

★★★ **BASIS CARRIES FORWARD ACROSS TAX YEARS.** A filer who shreds the vault after filing loses the
cost basis of every unsold lot. The next year's return then cannot be computed — or worse, is
computed from a reconstructed basis nobody can substantiate. **Whole-vault shredding trades a privacy
win for a permanent tax disaster.**

### So the requirement is PER-ITEM crypto-shredding

The interview answers need their own data key, wrapped by the vault key; destroying the wrapped key
renders the answers unrecoverable **while the lot ledger survives intact**. That is envelope
encryption, and none of it exists today.

★ Design questions this raises, which the ⑥ consult should now absorb (they replace scope-question
(a), which the owner has answered):

1. **Granularity of shredding.** Per answer? Per tax year's interview? Per question? Too fine is key
   sprawl; too coarse and the filer cannot keep the answers that still matter (a §1031 or carryover
   answer may be relevant for years).
2. **What survives a shred, and does the FIELD CENSUS still pass afterwards?** If the "declined"
   provenance pointed at a destroyed answer, the field's provenance becomes *unknown* — which under
   §3 is category 6, the defect. **A shred must not turn a lawfully-blank field into an unaccounted
   one.** Possibly the census must record the DECISION's existence separately from its CONTENT: "this
   was declined on 2026-04-15, detail shredded" is still a determinate provenance.
3. **Does an emitted return remain reproducible after a shred?** If not, say so plainly — a filer who
   cannot regenerate their own filed PDF has lost something real.
4. **Retention default.** Never auto-shred: destroying evidence of diligence must be an explicit,
   informed act by the filer, never a background job or a default.

## 6c. ✅ ⑥ CONSULT ANSWERED — `hybrid`, and it restructures §3

Verbatim: [`reviews/field-provenance-fable-r1.md`](../../reviews/field-provenance-fable-r1.md); four
load-bearing claims re-verified before folding, all held.

★★★ **The flat six-way in §3 is WRONG IN SHAPE — it conflates two layers.** `filled` / `declined` /
`refused` are **per-return outcomes**; `not-ours` / `artifact` / `never-decided` are **per-field
facts**. Written flat, *"declined" goes into a static file where it is false for the next filer who
answers yes.* Correct structure: a **static per-(form,year) census** recording the RULE for every
FQN, plus a **per-return resolution derived from it.**

**The static census — every FQN resolves to exactly one:**

| | rule | note |
|---|---|---|
| 1 | `computed(rule)` | we derive and write it |
| 2 | `collected(input)` | filer-supplied, written verbatim — **split from computed on purpose**: testimony vs arithmetic, which §G-11's "no stated zero from unstated inputs" needs |
| 3 | `asked(QuestionId)` | gated by an interview answer. ★ **"Declined" is the no-branch AT EMIT TIME, not a category anyone writes down** |
| 4a | `not-ours: filer-by-hand` | signature/date — carries a **"must be completed before filing" duty to surface**; an unsigned return is not a return (§6061) |
| 4b | `not-ours: third-party` | preparer block, 8283 donee/appraiser. ★ The **designee is neither** — it is an askable class-(B) election and belongs under `asked()` |
| 5 | `unmodeled(advisory)` | ★ **MISSING from §3 entirely** — a benefit outside btctax's scope, forgone by using it, and it must name its advisory (EIC 27–30, direct-deposit 35b–d) |
| 6 | `artifact` | 1-pt spacer / preprinted-constant cells — **`verify.rs:269` already knows these**; no decision is encodable |

*Not-applicable* is per-return **liveness** of a rule, not a static entry. *Refused* is whole-return
(`screen_inputs`), so it folds into `asked()`. ★★ **"Nothing ever decided" is an FQN ABSENT from the
census — the completeness test's red, never an entry.**

**Record-the-decline: protection on net, and mostly a non-event** — the durable record **already
exists** (`ReturnInputs` persists `foreign_accounts: Some(false)`, and must). The census adds
structure, not new testimonial content; *refusing* to hold it would be the active choice, and would
protect only a filer who answered falsely. Preparer practice retains exactly this (§6107 workpapers,
Form 8867, Circular 230) and an honest record supports §6664(c) reasonable cause. Consequences:
**keep the census DERIVED** (never a second store that can drift), and **the pointer references the
QUESTION ONLY, dereferenced at emit** — copying the answer is a stale-answer hazard.

**Two stores, one link, cross-checked.** Registry = year-stable tax semantics in core; census =
per-(form,year) PDF fact, belonging **inside the `.map.toml`** so `no_unmapped_filled` and the new
completeness check read the same universe. Two tests close it: `(map ∪ census) == the AcroForm FQN
set` exactly, and every referenced `QuestionId` resolves against `QuestionId::ALL`.

★★ **Granularity: the question's granularity is the FORM's own gating granularity.** Transcribe its
skip-instruction ("didn't pay vehicle-loan interest → skip 22–30"); never invent a coarser compound.
One question retires many FIELDS but only one PREDICATE *the form itself states*. Too-coarse's failure
mode: **a compound "no" spanning distinct legal predicates fabricates precision the filer never swore
to.** Per-field questions are never needed — provenance is per-field, questions are per-gate, the link
is many-to-one. Generalise it to **one DECISION → N fields** (filing status: five checkboxes, one
computed selection, four blank-because-sibling-filled).

★ **The §6a trailer mis-attribution is fixed for free**: region (header/body/trailer) becomes
derivable from the census's `not-ours` entries, rather than needing a geometry patch.

### ✅ SEQUENCING — census FIRST, and §6's claim was overdrawn

**This note said "§G-11 blocks the honest version." That was wrong by one level, and the correction is
accepted.** The 496 unaccounted are **unmapped**, so the emitter already leaves them genuinely blank —
classifying them changes no rendering. What §G-11 gates is honest rendering of the **mapped 662**, and
**the census audit of those 662 IS §G-11's worklist.** So: census lands now → §G-11's spec consumes
its mapped-field audit → any remapping follows.

### ★★★ OPEN — the shred requirement lands on the consult's own breaking assumption

§6b arrived after dispatch. A **derived** census resolves `asked()` by dereferencing the answer; **shred
the answer and the provenance becomes unresolvable — a lawfully-blank field silently becomes an
unaccounted one**, the exact defect the census exists to catch. The consult's `WHAT WOULD MAKE THIS
WRONG` names this: the "no second store" rule must bend far enough to persist *the decision's
existence, separate from its content*. **Unresolved. First question for any round 2.**

## 6d. ★★★ YEAR-SCOPING — the ⑥ consult's "year-stable registry" is not true, and it is already known

**Owner question, 2026-07-30, after the consult: "some questions will have different answers for
future tax years — does this design account for that?"** Three parts, and only the first is handled.

### 1. Per-year ANSWERS — ✅ handled

Storage is keyed by tax year: `return_inputs::get(conn, tax_year)` / `set(conn, year, &ri)`. TY2024
and TY2025 answers are separate records and **nothing carries forward silently**, which is correct —
a prior year's "no" is not testimony for this year.

### 2. Per-year QUESTION SETS — ❌ NOT handled, and documented as a structural limit

The ⑥ consult calls the registry *"year-stable tax semantics"*. **That is true only by accident of
what is in it today.** `questions.rs:403`, verbatim:

> `live` receives only `&ReturnInputs`, which **carries no tax year**, so it **CANNOT** be scoped to
> "years that compute modified AGI".

★★ **It has already bitten once.** `HasIncomeExclusion` is a TY2025 MAGI question. Unable to scope by
year, it was made **always live** — TY2024 filers are asked a TY2025 question — with the trade
recorded in place: a year-agnostic proxy (Schedule A over $10,000 SALT) *"would refuse TY2024 returns
that compute correctly today"*, a never-live question *"violates the Declarations invariant"*, and
`Some(false)` ⇒ MAGI = AGI, which TY2024's `FlatCap` assumes and never reads, so no TY2024 figure can
move.

★★★ **The workaround does not generalise, and Schedule 1-A is where it breaks.** Asking a TY2024
filer *"did you pay qualified passenger vehicle loan interest?"* asks about a **deduction that did not
exist in 2024**. A "no" is not neutral there — it is an answer to a question with no TY2024 meaning,
and by the consult's own granularity rule that **fabricates precision the filer never swore to**.

**So the registry needs a year dimension** — either a tax year on `ReturnInputs`, or `live` taking
`(year, &ReturnInputs)`. This is a change to P9's central invariant (*the liveness predicate is the
only copy*) and must not be smuggled in as a patch.

### 3. DURABILITY — a missing axis, and the one that costs the filer time

Because storage is per-year and nothing carries forward, **every question is re-asked every year**.
That is right for facts that change (foreign accounts, car-loan interest, blindness) and wasteful for
facts that do not (**date of birth**). At the question counts §4 implies, re-asking everything
annually is the §5 round-trip problem in a new costume.

★★ **But a prior-year answer must NEVER silently satisfy this year's provenance** — that is the
answered-ness invariant crossing a year boundary, and it is exactly the *"software answered for the
filer"* defect. The lawful shape is a **confirmation**, not a carry: *"Last year you said no. Still
true for 2025?"* — which is a new answer, given this year, with this year's date.

So each question needs to declare its **durability**: `PerYear` (re-ask blank) vs `Durable`
(offer last year's answer for confirmation, never assume it). Neither exists today.

★ **And it interacts with §6b's shred:** if last year's answers are shredded, a `Durable` question has
nothing to confirm against and must fall back to asking blank — which is correct behaviour, but it
means shredding has a *usability* cost the filer should be told about before they do it, not after.

## 7. Open questions for the ⑥ consult

1. Is the §3 taxonomy right, and is "Declined + question pointer" the correct shape for a lawful
   blank — or does linking a blank to an interview answer create a record we would not want to hold?
2. Should the field census and the question registry be **one system or two**?
3. What is the right question **granularity** — per section, per line, per field? (§5's measurement
   should inform this; it is the part that most needs a number rather than an opinion.)
4. How does this sequence against **§G-11**? The honest version needs an emitter that can express
   blank; does the census land first, or does §G-11's spec have to come first?
5. Does `screen_inputs`' first-only refusal become a blocker at this scale, and is an aggregate
   status surface a separate piece of work?
