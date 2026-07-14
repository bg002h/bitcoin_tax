# ARCH — Fable architect guidance: finishing Phase 6 (packet assembly)

Requested at HEAD `bc56837` (every P6 form fills; only packet assembly remains). Persisted verbatim
before folding, per STANDARD_WORKFLOW §2.

**Headline finding I had missed:** the §63(f) **aged/blind checkboxes**. Core computes the age-65/blind
additions into the printed L12, but `f1040.map.toml` has no age/blind checkboxes — so a filed 1040
would claim a nonstandard standard deduction with ZERO boxes checked, which fails the IRS's own
arithmetic cross-check. Same class of defect as P5-C1: a form internally inconsistent with itself.

**Escalated to the user:** Q6 (the DRAFT/attest gate) is a genuine contradiction between SPEC §9
("DRAFT watermark forced on every full return") and the phase exit condition ("the packet is
filable"). Fable declined to pick it, correctly — it is a legal-positioning call.

---

I have the full picture. One more verification pass on a detail that matters for Q1 — I confirmed the 2024 f1040.map.toml has no aged/blind checkboxes, no dependents rows, and no spouse/occupation fields, and that core computes `aged_blind_boxes` from `ri.header` (return_1040.rs:53). That gap is bigger than "name + SSN" and it changes my answer to Q1. Here is my guidance.

---

# Architect's response — finishing Phase 6

**Ranked by cost of getting it wrong:** (1) Q3 report figures, (2) Q2 packet location + all-or-nothing emission, (3) Q1 the header is bigger than name/SSN, (4) Q6 gate semantics (a spec conflict you must surface to the user), (5) Q4 dispatch rule, (6) Q5 spec amendment. Details per question, decisions first.

## Q3 (★) — Make the report print the PRINTED figures. Option (ii), unambiguously.

**Decision: the absolute/full-return block of the report renders whole-dollar printed-chain figures identical to the PDF, cell for cell. The crypto-delta block stays exact-cents. Do not print both in the absolute block.**

The clinching case is line 37. Today `render_dual_report` (`crates/btctax-cli/src/render.rs:1129`) prints `fmt_money(ar.total_tax)` — exact cents. Post-P6 the filed PDF's L37 can be a few dollars different. "Amount you owe" is not an analytical figure; it is an instruction to write a check. A tool that tells the filer $12,345.67 in the terminal and $12,347 on the filed form has produced two authoritative answers to "what do I pay," and no LIMITATIONS paragraph repairs that — option (i) documents the defect instead of removing it. For a fail-closed tool, the rule is: **any figure that appears on the filed return is reported as the filed return states it.** Cents are an implementation detail of getting the rounding right, not a user-facing truth.

What §3.1 actually commits you to: the round-all-amounts election (2024 i1040 p. 23) is an election about the *filed return* — round one amount, round them all, on the return **and its schedules**. It says nothing about your terminal output per se. But your own SPEC §3.4 does: "never … a plausible wrong number." A cents figure presented as "TOTAL TAX (L24)" — labeled with a 1040 line number — *is* a plausible wrong number once the filed L24 differs. The moment you label a report line with a form-line citation, you have promised the form's figure.

This choice also collapses three open items into one piece of work:

- `p5-m1` (report lacks interior schedule lines): the interior lines the report should print are precisely the `printed::*Lines` structs — they are *transcription instructions*, and transcription instructions in cents would be wrong twice over.
- `p5-report-vs-pdf-may-differ-by-rounding`: resolved by construction — there is no user-visible divergence left to document, only a one-line footnote ("whole-dollar figures per the rounding election; internal computation carries cents") plus the LIMITATIONS entry.
- Q6's "should the attestation surface a $2 disagreement": moot. Nothing disagrees.

Two boundaries to hold: (a) the **crypto-delta** block stays cents — it is not a filed figure, SPEC §6 already declares it a different, never-reconciled question, and the frozen engine is cents; (b) **carryover write-back** (`apply_carryover_writeback`) currently persists exact cents. Strictly, next year's worksheets start from *filed* (whole-dollar) figures. The difference is sub-dollar and rounded at first form-line use next year, so it is not a P6 gate item — but file a Minor follow-up ("write-back persists cents; filed-figure derivation would start from the printed lines") so it is a recorded decision, not an accident.

## Q2 — Packet assembly is a core function. The CLI keeps only I/O.

**Decision: add `printed::assemble_printed_return(ri, state, ar, year) -> PrintedReturn` to `crates/btctax-core/src/tax/printed.rs` (or a sibling `packet.rs`). `btctax-forms` gains `fill_full_return(&PrintedReturn, year) -> Vec<NamedForm>`. `cmd/admin.rs::export_irs_pdf` does vault/session, screens, attestation, and file writes — nothing else.**

The dependency order in your CONTINUITY sketch — `f8959` before `sch_2` before `f1040` — *is* the SPEC §3.1 composition invariant. "Schedule 2 L11 = the printed 8959 L18" is tax semantics, exactly the class of knowledge you've ruled out of `btctax-forms` ("zero tax arithmetic" — and which-line-feeds-which is arithmetic), and the CLI is the least testable place for it: your composition KATs live in core, and a CLI-resident composition would be the one copy of the wiring your KATs don't exercise. There is already evidence CLI orchestration duplicates — `assemble_absolute` is called at both tax.rs:245 and tax.rs:404.

```rust
pub struct PrintedReturn {
    pub header: ReturnHeader,          // Q1 — derived in core, one function
    pub filing_status: FilingStatus,
    pub f1040: Form1040Lines,
    pub sch_1: Option<Schedule1Lines>,
    pub sch_2: Option<Schedule2Lines>,
    pub sch_3: Option<Schedule3Lines>,
    pub sch_a: Option<ScheduleALines>,
    pub sch_b: Option<ScheduleBLines>,
    pub sch_c: Option<ScheduleCLines>,
    pub sch_d: ScheduleDLines,
    pub f8959: Form8959Lines,          // always built (Sch 2/1040 need it); emit rule separate
    pub f8960: Option<Form8960Lines>,
    pub f8995: Option<Form8995Lines>,
}
```

What keeps it from drifting — three mechanisms, all ones you already use:

1. **One composition site.** Re-point the existing composition KATs (`schedule_2_line11_takes_the_printed_8959_line_18_not_the_rounded_total`, KAT-9) to go *through* `assemble_printed_return` on a fixture, so the tested wiring is the shipped wiring, and add a tie-out KAT on `PrintedReturn` itself: `f1040.line23 == sch_2.line21`, `f1040.line8 == sch_1.line10`, `f1040.line13 == f8995.line15`, `f1040.line25c >= f8959.line24`, etc.
2. **Exhaustive destructure.** `fill_full_return` destructures `PrintedReturn` with no `..` (your `p1-r3-m1` precedent) — adding a form to the packet without a filler becomes a compile error, and so does the reverse.
3. **A cross-PDF byte oracle.** One KAT: build the packet from a kitchen-sink fixture, fill it, read the text values back (you have `tv()` in `tests/full_return_forms.rs:32`), assert the 1040 L23 *cell text* equals the Schedule 2 L21 *cell text*. That verifies the composition survived transcription — the one leg no current test covers end to end.

Two subsidiary decisions: (a) `fill_full_return` must be **all-or-nothing** — if any member filler fails (Schedule B overflow, a negative in a parenthesized cell), the whole packet refuses and zero bytes hit disk. A packet whose 1040 L2b cites a Schedule B that isn't attached is a wrong return; partial emission would be a fail-open. (b) There is a small existing asymmetry: most file-or-don't-file decisions are core `Option` returns, but Form 8959's ("L18 and L24 both zero") lives in the filler (`lib.rs:141`). Hoist it into core as `Form8959Lines::must_file()` so *every* filing decision is a core fact the packet KATs can see. Minor, do it while you're in there.

## Q1 — The header is not two fields; and the sharpest missing piece is the aged/blind checkboxes.

**Decision: (b), but core-derived. One `ReturnHeader` struct built by one core function; a shared `[identity]` map fragment + one shared writer in `btctax-forms`; SSN canonicalization in core with a screen refuse. And expand scope: the 1040's "identity header" must include the §63(f) age/blind checkboxes, and should include the dependents rows.**

First, the catch. `crates/btctax-forms/forms/2024/f1040.map.toml` has no age/blind checkboxes. But core computes the §63(f) additions (`aged_blind_boxes`, `return_1040.rs:53`) and the printed L12 *includes* them. A filed 1040 claiming a $32,300 MFJ standard deduction with **zero** age/blind boxes checked fails the IRS's own arithmetic cross-check — the checkbox count is how the Service validates a nonstandard standard deduction. This is the same class of defect as the P5-C1 understatement: a form internally inconsistent with its own attachments, except here the "attachment" is a checkbox on the same page. Treat those four checkboxes as **gating** for the packet, same tier as name/SSN. The dependents rows (name/SSN/relationship, credit boxes deliberately unchecked — consistent with L19 = 0 and the `CtcOdcOmitted` advisory) are one tier down but the data is already captured in `Dependent`; fill them. Occupation (`Person.occupation`, `return_inputs.rs:134`) into the signature block is a nicety; take it or leave it.

Now the shape. Why not (a) alone: ten fillers each formatting a name line means ten places to get the *semantics* wrong, and the semantics are non-trivial — "Name(s) shown on return" is **joint names on MFJ** for the schedules, but Schedule C wants the **proprietor only** ("Name of proprietor") with that person's SSN, not the joint line. A naive shared abstraction writes joint names on Schedule C; a naive per-form copy-paste does the same. So:

- **Core:** `ReturnHeader { name_line, taxpayer_name, taxpayer_ssn, spouse: Option<(name, ssn)>, address, aged_blind: AgedBlindBoxes, dependents: Vec<DependentRow>, occupations }`, built by one function from `ri.header` — MFJ name-joining decided once, KAT'd once. v1's single Schedule C is attributed to the taxpayer; document that as an explicit assumption (the SE computation already implicitly assumes it).
- **Map schema:** a small `IdentityCells { name: String, ssn: String }` struct in `map.rs`, a **required** `identity` field on the nine 2024-only schedule maps (a map missing it fails at deserialization — fail-closed at load, and every map loads in tests). On the two maps shared with the crypto slice (`ScheduleDMap`, `Form1040Map`), make it `Option` — the 2017/2025 maps have no verified identity FQNs and the slice has no `ReturnInputs` to source identity from — and have the *full* fillers fail closed on `None`. The 1040's full block (f1_04…f1_12, checkboxes, dependents rows) goes on `Form1040Map` directly; it's one form, don't abstract it.
- **Forms:** one `push_identity(&mut writes, &mut placements, &IdentityCells, &ReturnHeader)` helper in `cells.rs` using `FlatPlacement::free` (the `form8283.rs` precedent, `verify.rs:313`) — page-checked + in the no-unmapped set. Note `free`/no-unmapped catches *stray* writes, not *missing* ones, so each form's read-back KAT must assert the name/SSN cells read back non-empty.
- **SSN normalization (`p1-ssn-normalization-P6`) lands here, in core:** `Ssn::canonical(&str)` — strip to digits, require exactly nine, else error. Wire it into `screen_inputs` so a non-canonicalizable SSN **refuses at compute time**, before any PDF is attempted (§3.4: an unprintable SSN is an uncomputable line). The forms side then formats: hyphenated `NNN-NN-NNNN` for an 11-char cell, digits-only for a 9-char one — decide per cell by reading the field's actual `/MaxLen` from the PDF (the primary source, consistent with your never-guess rule), and add a general fill-time assert `value.len() <= MaxLen` for *all* text writes while you're there; it's cheap and catches the comb-misalignment class wholesale.

## Q4 — Keep two fillers. The risk you're carrying is the dispatch, not the duplication.

**Decision: your call is right. Do not unify.** Three reasons the duplication is load-bearing: (1) the input types are genuinely different (`ScheduleDTotals` from the projection vs `ScheduleDLines` printed chain) — a unified filler is an enum-and-branch with the same code volume and more coupling; (2) the slice is a *shipped, published* surface (v0.2.0) serving 2017/2025 and any no-`ReturnInputs` year, which the full path (TY2024-only, `AbsoluteReturn`-dependent) can't serve; (3) — the one I'd add to your rationale — **they are under different rounding regimes.** The slice prints exact-cents `fmt_money` (`lib.rs:69`, deliberately CSV-identical); the full chain prints whole dollars. A unified filler invites a future "harmonization" that must never happen: a crypto-only filer may legitimately file in cents.

The unseen risk: **after you delete the P5-C1 refusal, the only thing separating a full-return year from the understated crypto-slice Schedule D is an `if` in `export_irs_pdf`.** Today the refusal is a hard guarantee that the slice can't run on a full-return year (`admin.rs:216`). Its deletion downgrades a type-level impossibility to a branch. Mitigations, all cheap: put the dispatch in **one** function (`has ReturnInputs → full packet, else → slice`), pin it with KATs in *both* directions (full-return year's export contains a Schedule D with line 13 / lines 6+14 filled; a no-inputs year still gets the slice unchanged), and give the two packets **non-overlapping filenames** (the slice already writes `form_1040_capgains.pdf`; keep the full packet as `f1040.pdf`, `f1040s1.pdf`, … with a manifest) so artifacts from two runs can never be collated into a chimera return. Secondary, minor: both fillers read the same `schedule_d.map.toml`/`f1040.map.toml`; that's fine (the fault-injection KATs on each side cover a map edit), just keep the maps' full-return entries `Option` so 2017/2025 stay loadable.

## Q5 — Amend the SPEC. The refusal is not just acceptable — the 8949 pattern is *wrong* for Schedule B.

**Decision: keep the fail-closed refusal for v1; amend SPEC §7.4; file the real fix as a post-v1 follow-up whose design is a continuation *statement*, not form copies.**

The 8949 continuation pattern is sound because the IRS defines it ("complete as many Forms 8949 as needed") *and* because Schedule D exists above it as the aggregator of the copies' per-page totals. Schedule B has neither property: no instruction defines multiple Schedule B copies, and Schedule B **is** the aggregator — its line 4 flows directly to 1040 2b. Two Schedule Bs each cross-footing its own subset leave the question "which line 4 is 2b?" with no form-defined answer; a copy whose line 4 held the grand total while its rows sum to a subset would violate your own cross-foot invariant on its face. Professional practice for >14 payers is one Schedule B whose line 1 says "see attached statement" plus a plain continuation sheet — which is a *synthetic page generator*, new machinery outside your geometric oracle, for a case ("common W-2 household" with 15+ interest payers) at the far tail of your declared scope. So `overflow.rs::merge_copies` buys you approximately nothing here: it produces precisely the artifact (same-form copies with per-copy totals) that is wrong for this schedule.

Process note: this is a spec change, so it takes the §2 review loop — but you already have the "spec errata" lane, the change is a scope *reduction* to a fail-closed posture (the cheap direction to review), and you were already planning to declare the deviation to the P6 gate reviewer; land the amendment before that review so the reviewer certifies the spec as written rather than a declared exception. One code nit while you're there: the refusal is raised as `FormsError::Geometry` (`schedule_b.rs:55, 63`), which mislabels it — it's a capacity refusal, not a placement failure. Give it its own variant so the CLI can render "file Schedule B by hand" actionably and so the all-or-nothing packet logic can name it.

## Q6 — The gate: attestation unlocks the clean packet; no attestation yields a DRAFT copy. But this is the user's call to ratify, and you must surface the conflict.

There is a genuine contradiction in your artifacts: SPEC §9 says "DRAFT watermark + attestation **forced on every full return**," and your phase exit condition says the packet is **filable**. A packet whose every page says "DRAFT — ESTIMATE, NOT FOR FILING" (`watermark.rs:14-18`) is not filable. You cannot satisfy both readings literally; per your own workflow, an architect shouldn't quietly pick one — this is a legal-positioning decision (UPL posture, "we distribute") of exactly the class your memory notes reserve to the user.

**The design I recommend proposing:** a two-state gate where every full-return export passes through DRAFT-or-attest:

- `export-irs-pdf` with no `--attest`: emits the packet **DRAFT-watermarked** — a review copy, always available, never refused for lack of attestation.
- `export-irs-pdf --attest "<phrase>"`: emits the **clean, filable** packet. Use a *different, stronger phrase* than the pseudo gate's `"I attest this is true"` (`lib.rs:103`) — something in the shape of "I have reviewed this return and adopt it as my own" — because the thing being attested is different: not data veracity but adoption of a self-prepared return, which is the mechanical/UPL positioning §9 is protecting.
- **Pseudo-active composes and dominates:** a pseudo-active full-return export is watermarked *regardless* of attestation (fictional figures are never filable) — pin that with a KAT, because it is the one path where the two gates could be wired to fight.
- Enforce it structurally: `fill_full_return` takes the gate state and applies `stamp_draft_watermark` *inside the forms crate* before returning bytes, so an un-watermarked, un-attested packet is unconstructible by any caller — fail-closed by construction rather than by CLI discipline. The crypto-slice path keeps its existing pseudo-only semantics untouched (it is shipped behavior; §9's "every full return" doesn't reach it).

Either way the SPEC §9 text needs one sentence of amendment (to say what the gate *is*), and that rides the same review as Q5's amendment. And per Q3: choosing report-option (ii) means the attestation has no rounding divergence to disclose — one less thing the phrase has to carry.

## Q7 — Ordering: yours is right with two amendments. And yes, build three P7 things now.

Your order (identity → packet → gate → delete refusal) is correct in its essentials — in particular, refusal-deletion *last* is non-negotiable, and gate-before-deletion matters because the first filable packet must be born gated. Amendments:

1. **Start with the core packet struct, not the identity fill.** `assemble_printed_return` + `ReturnHeader` + SSN canonicalization are pure core (no PDF work), and the identity fill *consumes* `ReturnHeader` — building maps-first means inventing the header data shape twice. Revised order: **(P6.1)** core `PrintedReturn` + `ReturnHeader` + `Ssn::canonical` + screen refuse + tie-out KATs → **(P6.2)** identity map fragments + shared writer + 1040 header block *including aged/blind boxes* → **(P6.3)** `fill_full_return` + dispatch in `export_irs_pdf` + report renders `PrintedReturn` (Q3) + `p5-n5` wrapping + LIMITATIONS → **(P6.4)** gate (after the user ratifies Q6) → **(P6.5)** delete `CryptoSliceExportForFullReturnYear` + its KAT, replace with the two dispatch KATs → **(P6.6)** Fable gate review.
2. **Fold the spec amendments (Q5, Q6) in before the gate review**, so the reviewer measures against a spec you intend to keep.

Build for P7 now, while the maps are fresh:

- **A line-keyed extractor:** `testonly::extract_lines(bytes, &Map) -> BTreeMap<&'static str, String>` — the inverse transcriber. It is trivial today (you know every FQN) and it is the engine of P7's ATS Scenario-2 *partial-line diff* and golden-return diffs; written in P7 it means re-spelunking FQNs cold. It also powers the cross-PDF tie-out KAT from Q2 immediately.
- **The kitchen-sink household fixture** — one synthetic household that produces *every* form in the packet (itemized A + B + C + D all four routings can't coexist, so one primary + the three D-routing variants). P6's packet KAT needs it anyway; put it behind core's `#[doc(hidden)] pub mod testonly` so P7's golden matrix starts from it instead of rebuilding it.
- **Packet-level determinism + manifest:** extend the existing per-form byte-determinism test to the assembled packet, and emit the manifest in IRS **Attachment Sequence No.** order (it's printed on each schedule's header). P7's whole-packet golden hash then has a stable, collation-correct artifact to pin — and the filer gets their stapling order for free.

One caution flowing from all of the above: P6's exit condition should be stated as "the packet is filable **and every figure on it is internally and mutually consistent** (cross-foots, ties to attachments, checkbox-consistent with L12)" — the aged/blind gap shows "every money line is right" was quietly narrower than "the return is right."
