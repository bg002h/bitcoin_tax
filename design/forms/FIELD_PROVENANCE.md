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
