# P7 Independent Review — ROUND 3, on the r2 fold (Fable, r3)

*Persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2. Author = Opus; reviewer = Fable.
Nothing below has been edited, softened, or reordered.*

---

**Baseline:** r2 fold diff (636 lines) + current source. Tree clean, `make check` green at 1708, frozen files byte-identical to `059ec2a`. ✓

## Verification of the r2 fold

I attacked each fix rather than reading it.

- **r2-I1 (wrong TIN) — GENUINELY FIXED.** Reverting `&proprietor.ssn` → `&header.taxpayer.ssn` fails `form_8995_row_1i_carries_the_proprietors_tin_not_the_taxpayers`. The KAT builds a real MFJ/spouse-owned return and asserts `222-22-2222`. Correct fix (`header.proprietor`, fail-closed), and the fail-closed branch is provably unreachable — `line2 > 0` ⇒ `schedule_c.is_some()` ⇒ `proprietor.is_some()`.
- **r2-I2 (empty description) — the *filler* half is genuinely fixed.** The row is now keyed off `line2 > 0`, not the name, and fails closed on a blank name. Mutating the gate (`if true` + drop the blank-name check) kills **two** KATs. Good. **But the core half — the new refusal — is untested. See I1 below.**
- **r2-I3 (mis-cited instruction) — HONESTLY FIXED.** The new `why` quotes the instruction correctly, states that btctax **departs** from it, names OTS as the one following it, and — importantly — volunteers that the error *can fall either way, including understating tax by $1*, which is the direction btctax's posture refuses. That is a harder thing to write than the version I flagged, and it is the right one. Filing the election itself as `spec-3.1-crossfoot-vs-round-the-total` for a user decision, rather than changing it unilaterally, is the correct call. **I re-read the instruction (p. 23) and the new text is accurate on every point.** On the open question the fold asks: I agree the instruction's scope is genuinely ambiguous (source-document amounts vs. amounts already entered on other lines), and the entry now takes the reading *least* favourable to btctax, which is the right posture for a gate.
- **`assert_row1_is_one_row` — sound, and NOT circular.** I dumped the real rects: (a) spans y **[551.97, 575.97]** (the full 24pt row), (b) and (c) sit at cy 558 inside it. Fault-injected a cross-row typo (`row1_business` → row 1ii's `f1_6`): **caught**. If (a) itself is the mis-mapped one, the band moves to row 1ii and (b)/(c) at 558 fall outside — also caught. The only configuration that passes is all three moved consistently to another row (I injected it: passes) — and that is *correct*, because line 2 reads "Combine lines 1i through **1v**", so a business listed on row 1ii is a validly-filled form. The check catches exactly the harmful case. (The fold's framing — "can't be satisfied by a wholly-consistent wrong row" — is false, but harmlessly so; see N1.)
- **Goldens regenerate bit-identically** after the QD change (12 households, both oracles re-run from scratch).
- **The QD term is now genuinely load-bearing.** Dropping `qualified_dividends` from `net_capital_gain` (return_1040.rs) now goes red on `golden_returns`. Previously it was free. Good close.
- **The refusal is in the right screen.** `screen_inputs` is input-only (no compute dependency) and is the fail-closed gate on the real path (`resolve.rs:96`, `admin.rs:453`). `trim()` is the right emptiness test. No exhaustive `match` on `RefuseReason` needed updating. No shipped fixture or example profile carries a Schedule C without a description, so nothing silently changed.

The fold is good. It closed all three Importants, and unlike the last two folds it did **not** introduce a wrong number. But it repeated the one habit that keeps recurring: it shipped a new load-bearing guard with no test behind it.

---

## Findings

### [IMPORTANT] The new `ScheduleCNoBusinessDescription` refusal has zero tests — deleting it passes all 1708
**Where:** `crates/btctax-core/src/tax/return_refuse.rs:594-606` (the screen), `:117` (the variant). No test anywhere constructs a Schedule C with an empty `business_description`.
**What:** The refusal that is the **first line of defence** for r2-I2 is entirely uncovered. I disabled it (`if false && c.business_description.trim().is_empty()`) and ran the full gate:
```
Summary [7.376s] 1708 tests run: 1708 passed, 1 skipped
```
This is the third consecutive fold to ship an untested load-bearing guard — it is exactly the r1-I2 finding (`business_qbi` leg of the QBI refuse, which my M3 mutation survived) repeating.
**Why it matters:** It is not merely belt-and-braces behind the filler's fail-closed. It is the **only** guard on **Schedule C line A**. A Schedule C whose net profit is at or below the §6017 $400 SE floor (or zero) produces `business_qbi = 0` ⇒ `line2 = 0` ⇒ no Form 8995 at all ⇒ the filler's fail-closed **never fires** — and `schedule_c.rs` writes line A only `if !value.is_empty()`. So with this screen regressed, that filer files a **Schedule C with a blank "Principal business or profession"** and nothing in the suite notices. Facially incomplete, the same class as P6's unnamed 8949 and r1-I1's unnamed 8995. Even on the 8995 path, regressing this screen degrades a clean `Refusal` into a `FormsError::Geometry` thrown from deep inside the filler.
**Evidence:** the mutation run above; `return_refuse.rs:780` shows a `screen_inputs(...).map(|r| r.reason)` test helper already exists, so the test is a three-line addition.
**Fix:** Add a KAT asserting `screen_inputs` returns `RefuseReason::ScheduleCNoBusinessDescription` for a Schedule C with `business_description: ""` (and one with `"   "`, to pin `trim()`), plus a negative leg with a real name. Consider also asserting Schedule C line A is non-blank on the golden packets — the `f1040sc` fills are already transcribed there.

### [MINOR] Row 1i(b) hardcodes the hyphenated SSN, bypassing the crate's own `/MaxLen` fail-closed guard
**Where:** `crates/btctax-forms/src/form8995.rs:~180` — `push_literal(..., &proprietor.ssn.hyphenated(), ...)`; contrast `cells.rs:104-148` (`render_ssn` / `push_identity`).
**What:** The crate has a dedicated guard for exactly this: `render_ssn(ssn, max_len)` reads the cell's own `/MaxLen` and **fails closed** — *"an SSN cell with /MaxLen {n} cannot hold an SSN (9 digits) — the map points at the wrong widget."* Every other SSN cell in the crate goes through it. Row 1i(b) does not; it hardcodes `.hyphenated()` and asserts the `/MaxLen` in a comment instead of verifying it.
**Why it matters:** I confirmed row 1i(b)'s `/MaxLen` **is** 11, so the output is correct for TY2024 — no wrong return today. But it is a latent trap: if the TY2025 revision makes that cell a 9-char comb (as the 1040's own SSN cells are), this silently writes 11 characters into a 9-character field, while every other SSN cell in the packet would have failed closed. It also breaks the crate's single established pattern for a cell whose rendering depends on the PDF.
**Fix:** Route it through `render_ssn`/`ssn_for_cell` with the blank PDF's `/MaxLen`, as `push_identity` does.

### [MINOR] The filler validates the trimmed name but writes the untrimmed one
**Where:** `form8995.rs:~146` — guard is `lines.business_name.trim().is_empty()`, write is `&lines.business_name`.
**What:** Core refuses on `trim().is_empty()`, so `"  "` cannot reach the filler. But a description like `"  Bitcoin mining  "` passes both and is written to row 1i(a) with its surrounding whitespace intact. Schedule C line A (`printed.rs:1001`, `h.business_description.clone()`) has the same behaviour, so the two at least agree.
**Fix:** Trim once at capture (or in `ScheduleCHeader`), so line A and row 1i(a) carry the same canonical string.

### [MINOR] The "btctax must agree with one oracle" guard is vacuous on TOTAL TAX
**Where:** `golden_returns.rs` — `let matches_2 = o2.is_none_or(|v| ours == v);` combined with the `matches_1 || matches_2 || d.agrees_with.starts_with("neither")` assertion.
**What:** TOTAL TAX is the one line where taxcalc reports no comparable figure (`o2 = None`), so `is_none_or` makes `matches_2` **unconditionally true** there. The anti-"btctax against the world" guard therefore cannot fire on TOTAL TAX for any household, and the new `agrees_with: "neither"` escape is not actually doing any work — the assertion would have passed regardless.
**Why it matters:** Not a live defect (the divergence is correctly declared and the outlier is pinned), but the assertion's message promises a guarantee it structurally cannot provide on that line, which is how a guard rots into decoration. It also means a *future* undeclared TOTAL-TAX disagreement is caught only by the `diffs` list, not by the stronger guard.
**Fix:** Make the guard explicit about the one-oracle case: when `o2.is_none()`, require `matches_1 || agrees_with.starts_with("neither")` rather than letting `is_none_or` short-circuit it.

### [NIT] Two doc claims in `form8995.rs` are contradicted by the PDF's own rects
`assert_row1_is_one_row`'s docstring says (b) and (c) sit "in [the (a) cell's] **upper half**". From the real rects, (a) is y[551.97, 575.97] (center 564) and (b)/(c) are y[551.97, 563.97] (center 558) — they sit in the **lower** half. Separately, the fold asks me to confirm the check "can't be satisfied by a wholly-consistent wrong row": it **can** (I injected it and it passes), and that is fine — a business on row 1ii is a valid form, since line 2 combines rows 1i–1v. Both are harmless, but they are comments that were reasoned about rather than measured, on a function whose whole purpose is measurement.

### [NIT] The cross-footing exception is still a hardcoded string match in a second file
Unchanged from r2. `golden_packet.rs` re-pins `h.name == "single_miner_qbi_limited_by_net_capital_gain" && cell == "line24"` with the figures updated by hand (16,832/16,833). It is self-invalidating, so it will not rot silently — but it is now the second time these constants had to be hand-edited in two places when the household changed, which is the cost the duplication was always going to charge.

---

## Verdict

The r2 fold is the first of the three that did not introduce a wrong number, and its handling of I3 is genuinely good work — it took the reading of the IRS instruction *least* favourable to itself, disclosed that the election can understate tax by $1, and escalated the election to the user rather than quietly keeping it. I3, I1 (proprietor TIN) and the filler half of I2 are all fixed and all fault-injected.

But the core half of I2 shipped with no test at all, and it is the sole guard preventing a filed Schedule C with a blank line A. That is the same defect class as r1-I2, in the third fold running: the fix is written, the guard is correct, and nothing holds it in place. It gates.

**VERDICT: 0 Critical / 1 Important / 3 Minor / 2 Nit**
