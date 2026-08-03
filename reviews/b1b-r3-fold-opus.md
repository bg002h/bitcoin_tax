# §G-28/B1b review r3 — THE FOLD (Opus, 2026-08-02)

Scope: `git show 62f1498` / `git diff 21fa20e..62f1498` — the fold of r1+r2 ONLY, not the feature. One
question: *did the fold introduce a new defect, or fail to close a finding it claims to close?*

This round exists because of `FOLLOWUPS.md` §G-28's recorded pattern: **the FOLD carries the defect.**
Three consecutive earlier rounds each found a Critical or Important inside the previous round's fix.

**VERDICT: 0 Critical, 0 Important — the §2 gate is MET.** All four r1/r2 Importants confirmed CLOSED
with evidence. 3 Minor + 4 Nit introduced by the fold; all folded in the next commit.

**Persisted VERBATIM before folding.** Everything below the rule is the reviewer's own text.

---

## Verdict

**0 Critical, 0 Important.** The fold's high-risk changes are sound; three Minor and four Nit defects were introduced, all documentation/instrument-strength, none moving a dollar.

### Round-1 Importants — did the fold close them?

| # | Finding | Verdict | Evidence |
|---|---|---|---|
| r1-I1 | `ClearField` launders `Some($0)` into `qbi_w2_wages`/`qbi_ubia` | **CLOSED** | `sections.rs:1079` / `:1095` each carry a dedicated `clear` writing `None`, mirroring their own `set`'s `ok_or(SetError::NoSuchRow)`; `apply.rs:625` asserts `get(...) == None` after `ClearField` for both ids, and reds if either `clear` returns to `None`. |
| r1-I2 | Two authorities for TI-before-QBI | **CLOSED** | One binding at `return_1040.rs:1471`, read by `Qbi199aRegime::of` (:1488, :1539), `uses_8995a` (:1499), `PartIToIiiInputs` (:1537), `compute_8995` (:1564) and `printed_inputs` (:1735). Line 20 = `round_dollar(i.ti_before_qbi)` and line 33 = `round_dollar(pi.ti_before_qbi)` are now provably equal (both idempotent on an already-rounded input). |
| r1-I3 | 25 printed lines outside §G-11 coverage | **CLOSED** | `cargo run -p xtask -- line-coverage` reports `f8995a:39` (was 14) inside 228 total, **and `8 not line-bound (ratchet 8)` is unchanged** — i.e. all 25 new rows passed rule (2b) `label_precedes`, so each quote is printed immediately after its own line number in `f8995a--2024.txt`. That is the Form-6251-line-33 guarantee actually firing on this form. |
| r2-I1 | `QbiAboveThreshold` remediation pointed nowhere | **CLOSED** | Refusal text now names `income import` + `qbi_w2_wages`/`qbi_ubia` under `[schedule_c]` — I verified all three: `IncomeCmd::Import` exists (`cli.rs:390`), `ScheduleCInputs` has no `rename_all` so the serde keys are literally those (`return_inputs.rs:313,318`), and the editor section title is `"§199A limitation (…)"` (`sections.rs:1144`). Anchor is now `Field(QbiW2Wages)`/`Field(QbiUbia)`, removed from the `NotInForm` list, with a test that both ids exist in `form_spec()`. |

### The six named risks — what I actually checked

1. **The reordering.** Clean. `agi` (:1388) and `deduction` (:1432) both precede the new site (:1452/:1471); the diff moves only two `let` bindings *earlier* and nothing else reorders. `assemble_absolute` has a single exit and `PrintedInputs` has no `Default`/`Deserialize`, so `screen_absolute`'s `ar.printed_inputs.ti_before_qbi` (:1937) is bit-identical to the assembler's on every path. **Rounding direction:** I worked both boundaries. At the bottom ($191,950), the pre-fold path already yielded line22=0 ⇒ ratio 0 ⇒ line26=line17 ⇒ line13=line3, i.e. the full uncapped 20% — the same figure the post-fold below-threshold path gives; only the *form* (8995 vs 8995-A) and a now-suppressed spurious `QbiAboveThreshold`/`SstbInPhaseInRange` refusal differ. At the top ($241,950), ratio = 1 ⇒ line26 = line18 = line10 ⇒ line13 = line10 = the hard cap — continuous. **No figure moves in either direction.** `compute_8995`'s income limit now uses the same rounded line 11 the printed Form 8995 uses, which narrows a pre-existing rounded/unrounded split rather than widening it.
2. **`cover_fns_not_registered`** — works on the real file (see Minor 2 for its limits). I replicated the algorithm and ran it: baseline 0 orphans; dropping `cover_form8995apartiii`, `cover_form8995apartii` or `cover_setaxresult` from `all()` each reports exactly 1; removing *both* `cover_schedulebrow` call sites from `cover_scheduleblines` reports it; an orphan appended as the last fn in the file is reported.
3. **Verbatim-test widening** — clean. Replicated: the rejoin produces exactly four captions, `checked == 44` exactly (no extra label admitted), and five planted paraphrases (`Check if service business`, `Check if aggregated`, `Taxpayer ID number`, `Check if a patron`, `Trade or business name`) each red. Cross-cell fabrication requires a quote spanning a `(c)`/`(d)` column marker — not a reachable paraphrase, and the same class already existed in the pre-fold `norm(FORM_TEXT)`.
4. **The two `clear` closures** — clean. Error behaviour is identical to the fields' own `set`; `get` is `and_then`-shaped so `None` is handled; the refusal that reads them fires only inside `if let Some(c) = ri.schedule_c`, so the section is always live.
5. **`line15.max(ZERO)`** — masks nothing. `line13 = max(line11, line12)` with `line11 = min(line3, line10) ≥ 0`, `line12 = line26 ≥ line18 ≥ 0`, and `line14` is always `None` (a patron refuses). It is i8995a's own sentence, and every path that could make it negative refuses upstream.
6. **MAX_EXCEPTIONS 12 → 15** — 15 rows, ratchet 15, no slack. One of the three is genuinely sui generis; two are not (Minor 3).

### Findings

**SEVERITY: Minor**
**FILE:** `crates/btctax-core/src/tax/line_coverage.rs:177-190` (also `crates/xtask/src/line_coverage_check.rs:311-320` and `:859-864`)
**CLAIM:** Three new items were inserted *between* an existing doc-comment block and the function it documented, so three functions lost their docs and three doc blocks now describe the wrong item.
**EVIDENCE:** `line_coverage.rs:177-183` is the block `/// §G-28/B1a — **Form 8995-A Part IV**. … ★ Part IV is Form 8995's lines 5-17 with a DPAD line inserted …`. Pre-fold (`git show 21fa20e:…`) it sat directly above `pub fn cover_form8995apartiv`. The fold inserted the new `Part II` paragraph at :184 and `pub fn cover_form8995apartii` at :191, so that block now heads **Part II**, and `pub fn cover_form8995apartiv` at :394 has **no doc comment at all**. Same shape twice in xtask: `/// (4b)'s decision for ONE source file …` (:311-317) now heads `cover_fns_not_registered` while `fn missing_cover_fns` (:406) is bare; `/// ★★★ (4b) — the completeness scan, now killable …` (:859-863) now heads the new kill test while `the_completeness_scan_demands_a_cover_fn_for_a_printed_money_type` (:915) is bare.
**FAILURE:** No figure moves. The Part II coverage function is now headed by a sentence asserting it is Part IV and that its lines are "Form 8995's lines 5-17 with a DPAD line inserted" — false for lines 2-16 — in the one module whose entire purpose is that a doc comment matches what it describes. The next reader auditing Part II reads Part IV's provenance argument.

**SEVERITY: Minor**
**FILE:** `crates/xtask/src/line_coverage_check.rs:320-404`
**CLAIM:** The reachability walk is a plain substring scan over a chunk that extends to the *next top-level `pub fn`*, so (a) a `cover_x(` occurrence in a **comment or string** inside any reachable chunk marks `x` reached, and (b) because `all()` is the **last** top-level `pub fn` in `line_coverage.rs`, its chunk runs to EOF — anything appended after it (a `#[cfg(test)] mod tests`, a helper, a doc block) grants reachability to every function it names.
**EVIDENCE:** `let end = starts.get(k + 1).copied().unwrap_or(cov_src.len());` … `if n != &cur && body.contains(&format!("{n}("))`. `grep -n '^pub fn ' line_coverage.rs` puts `all()` last, at :2450 of 2486. Running the algorithm: with `cover_form8995apartiii` deleted from `all()` **plus** the single comment line `// see cover_form8995apartiii(x) for details` inside `all()`, the checker returns `[]`. Same with a `#[cfg(test)] mod tests { … super::cover_form8995apartiii(&Default::default()) … }` appended after `all()` — `[]`. The committed kill test's fixtures (a) and (b) both place `pub fn all()` **first**, so the file shape that actually exists is never exercised.
**FAILURE:** Latent, not live — no comment in `line_coverage.rs` names a `cover_*` with a paren, and the file has no test module (verified). But the instrument's own claim ("`all()` cannot reach it") is weaker than stated: the very next author who adds `#[cfg(test)] mod tests` to `line_coverage.rs` silently disables it for every function the tests touch, and the report keeps printing **OK** — which is the exact false-completeness class the checker was written to kill.

**SEVERITY: Minor**
**FILE:** `crates/xtask/src/line_coverage_check.rs:99-116` (rows at `crates/btctax-core/src/tax/line_coverage.rs:275` and `:281`)
**CLAIM:** Two of the three new exception rows fit the **existing** `Production::Carry` by its own written definition, so the ratchet was raised two units more than the grammar required, and §G-29 proposes a production that duplicates one already there.
**EVIDENCE:** `Production::Carry` is documented at `line_coverage.rs:61-62` as `/// *"Enter the amount from line N"* — blank when the source line is blank.` The two new exceptions quote *"Phased-in reduction. **Enter the amount from line 26**, if any"* and *"Patron reduction. **Enter the amount from** Schedule D (Form 8995-A), **line 6**, if any"* — verbatim instances of that sentence, blank exactly when their source line is blank. `Coverage::line` and `Coverage::exception` differ only in `production`/`reason`; both discard the value (`_value: Usd`, :132/:154), so both call sites would pass `unwrap_or(Usd::ZERO)` identically, and `check()` imposes **no additional rule on `Carry`** (rules (3) and (4) key on `Clamped` and `Combine` only). Only `f8995a:24` (a ratio; no `Divide` production exists) is genuinely unclassifiable.
**FAILURE:** No figure moves. `MAX_EXCEPTIONS` is 15 where 13 would do, and the ratchet — whose stated value is that it "only ever goes DOWN" — now carries two units of slack a future author can spend without editing the constant. §G-29 files `Production::ConditionalBlank` as a missing production for a case `Carry` already names.

**SEVERITY: Nit**
**FILE:** `crates/xtask/src/line_coverage_check.rs:113,116`
**CLAIM:** The MAX_EXCEPTIONS rationale miscounts, and contradicts `FOLLOWUPS.md` §G-29 written in the same commit.
**EVIDENCE:** The comment names three rows sharing the sentence (`f8995a:12`, `f8995a:14`, and "the pre-existing **f8995a:38**") but then says *"**Four rows**, one sentence"* and *"adding it would take this number **DOWN by four**"*. §G-29 (`FOLLOWUPS.md`) tabulates the same three rows and says *"One sentence covers **three** rows"* and *"would take `MAX_EXCEPTIONS` **down by three**"*.
**FAILURE:** None on a return. A future author acting on the xtask comment would expect 15 → 11 and see 15 → 12.

**SEVERITY: Nit**
**FILE:** `crates/btctax-input-form/src/apply.rs:68` and `:675`
**CLAIM:** The fold added the first non-registry, plain-`Money` fields with a dedicated `clear` and did not update the two comments that enumerate that set.
**EVIDENCE:** `:68` — `// `clear` (the 13 registry-delegating tri-state/date leaves — whose registry setter writes only a definite yes/no …)`; `:675` — `/// … the 13 registry-delegating tri-state/date fields UN-ANSWER their underlying `Option` leaf to `None` …; plain Date/Money/Text/Bool/Secret clear to their empty value`. `QbiW2Wages`/`QbiUbia` are plain `FieldKind::Money`, are not registry-delegating, and now carry a dedicated `clear` — so both the count and the "plain Money clears to its empty value" clause are false.
**FAILURE:** None. A reader of the dispatcher is told the dedicated-`clear` set is closed over registry tri-state/date fields, which is the exact assumption r1-I1 punished.

**SEVERITY: Nit**
**FILE:** `crates/btctax-forms/tests/f8995a_map.rs:271`
**CLAIM:** The comment describes a filter the code does not implement.
**EVIDENCE:** `// The continuation is the next non-empty line whose first cell is not a line number.` The loop below is `for cont in lines.iter().skip(i + 1).take(3)` with no emptiness or first-cell test and **no `break`** — it appends a zip for *every* one of the three whose cell count happens to match.
**FAILURE:** None today (I ran it: exactly one of the three matches, producing exactly the four captions). It would silently append a second, wrong zip if the extract ever put another equal-arity row within three lines.

**SEVERITY: Nit**
**FILE:** `crates/btctax-forms/tests/f8995a_fill.rs:257`
**CLAIM:** `assert!(!printed.ends_with('0'))` encodes a rule that is false for line 24 in general and is redundant beside the exact assertion above it.
**EVIDENCE:** Immediately preceded by `assert_eq!(printed, "28.062", …)`, which already pins the value exactly. But a ratio of `25000/50000` prints `"50"` and `50000/50000` prints `"100"` — both correct, both `ends_with('0')`.
**FAILURE:** None on the current fixture. Re-pointing the fixture at a round percentage reds the suite for a reason that is not a defect.
