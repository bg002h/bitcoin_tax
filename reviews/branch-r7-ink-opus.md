# r7 — INK LENS (Opus)

**Date:** 2026-08-02 · **Range:** `4fe5ce4..HEAD` (`58515da`) · **Brief:** [`BRIEF-r7.md`](./BRIEF-r7.md)

**Scope:** `crates/btctax-forms/src/form1040_full.rs` (+51, the r6 I-1 geometry guard) and anything
that reads back the cells it writes. Lens B (`line_coverage_check.rs`) deliberately not opened.

**VERDICT: 0 Critical / 1 Important / 2 Minor / 1 Nit**

The guard is **correct, reachable, and pure** — I planted both swaps and watched it refuse. It cannot
put a mark on a return: it writes nothing, and the whole forms suite is byte-identical with and
without it. The Important is that its own **B1 claim is false**: the comment argues the 1040 KAT suite
is its kill-test, and it is not — **deleting the guard outright leaves 233/233 green.**

**Verbatim below.**

---

## THE ONE QUESTION, answered

**Can this put a wrong mark — or remove a required one — on a filed federal return?** No. The added
block is pure: three `need()` lookups, three `cy()` reads off the **blank** PDF, one comparison, one
`return Err`. It never touches `writes`, `placements`, or `push_money`. Verified structurally
(form1040_full.rs:222-248) and empirically: with the guard deleted the entire `btctax-forms` suite —
including `full_return_form_fills_are_byte_deterministic`, `the_whole_packet_is_byte_reproducible` and
all 8 `golden_packet::diff_shard_*` — is **233 passed, 4 skipped**, identical to HEAD. No ink moved.

**Does it CLAIM a guarantee it does not deliver?** Two claims. The geometric one is delivered
(I-1 below has the evidence). The **B1 one is not.**

---

```
SEVERITY: Important
WHERE: crates/btctax-forms/src/form1040_full.rs:214-221
CLAIM: The guard shipped with NO committed kill-test, and the comment that says otherwise is wrong:
       removing the guard entirely reds nothing. B1's own reviewable question — "which test reds when
       this checker is removed?" — has the answer "none".
FAILURE: The comment asserts:

           // ★★ B1, observed. This guard sits on the production path of EVERY `fill_form_1040_full`,
           // so the whole 1040 KAT suite is its kill-test rather than one dedicated case.

       That conflates two different things. The KAT suite reds when the **map** is broken *while the
       guard is present* — which is what the author observed. It does not red when the **guard** is
       broken or deleted, because the committed map is correct. Those are the two halves B1 exists to
       keep apart.

       Mutation-verified, not argued. I removed the entire block (form1040_full.rs:222-248, 1353
       bytes) and ran the suite:

         guard present  → `cargo nextest run -p btctax-forms` → 233 run: 233 passed, 4 skipped
         guard DELETED  → `cargo nextest run -p btctax-forms` → 233 run: 233 passed, 4 skipped

       Nothing observed the difference. So the guard is, today, exactly the artifact `CLAUDE.md` §B1
       names — an instrument whose discrimination was watched once, by hand, in a working tree, and
       then shipped with no standing witness. Its own protection lasts until the next person who finds
       an unexplained `{ ... }` block and tidies it, or until a refactor moves it after the branch.

       The stakes are not hypothetical-in-general: this guard's whole justification (r6 I-1, restated
       at :207-209) is that **the cost is entirely in the NEXT map** — TY2025's full-return map does
       not exist yet. An unwitnessed guard is not there when the next map is authored, which is the
       one moment it exists for.

EVIDENCE: `CLAUDE.md` §B1: "No checker exists until it has been observed RED on a planted defect. Every
       new census, conformance check, citation check, lint, or review harness lands **paired** with a
       negative test that plants the exact defect it exists to catch and asserts red." And under
       "Tests for conformance": "**A guarantee without a test that reds when it is removed does not
       exist.** Mutation-verify."

       ★★ The template is eight lines long and sits in the same file, 500 lines below the code under
       review — `crates/btctax-forms/tests/full_return_forms.rs:1726-1733`:

           #[test]
           fn form_1040_full_same_column_swap_fails_closed() {
               let mut map = btctax_forms::Form1040Map::ty2024();
               std::mem::swap(&mut map.line9, &mut map.line15);
               let err = fill_form_1040_full_with_map(&f1040(), &kitchen_sink_header(),
                                                      FilingStatus::Single, &map)
                   .expect_err("a same-column swap must fail closed");
               assert!(matches!(err, FormsError::Geometry(_)), "{err:?}");
           }

       That is the identical shape (`Form1040Map::ty2024()` + `mem::swap` + `expect_err`) — it is the
       test r6's Important cited as *unable* to see the 34/35a/37 case, because it swaps 9↔15. The fix
       is to write it twice more with `line35a`/`line37` and `line34`/`line37`. Nothing in the current
       range does.

       ★ This is the same field-of-view failure `CLAUDE.md` §B3 records for this very branch: the fix
       already existed in the tree, and nobody carried it across.

FIX: two `#[test]`s modelled on `form_1040_full_same_column_swap_fails_closed`, asserting
       `FormsError::Geometry(m)` with `m.contains("refund/owe block")` so they cannot be satisfied by
       the descent leg firing for some other reason. I confirmed both plants do reach the guard and
       produce that string (see I-1's evidence), so both tests are known-red-on-the-defect before they
       are written.
```

---

## Minor / Nit

```
SEVERITY: Minor
WHERE: crates/btctax-forms/src/form1040_full.rs:218-219
CLAIM: The two error messages the B1 comment presents as observed output, inside `Geometry("…")`
       quotes, are not what the code emits. The observation was real; the quote is a paraphrase.
FAILURE: The comment records:

           //   line35a ↔ line37 → Geometry("… sit at y 486.0/414.0/474.0, which is not strictly descending")
           //   line34  ↔ line37 → Geometry("… sit at y 414.0/474.0/486.0, which is not strictly descending")

       I planted both swaps in the committed `forms/2024/f1040.map.toml` and captured the real strings:

           35a↔37 → Geometry("1040 refund/owe block is mis-mapped: lines 34/35a/37 sit at
                              y 486.00153/414.00153/473.9995, which is not strictly descending. …")
           34 ↔37 → Geometry("1040 refund/owe block is mis-mapped: lines 34/35a/37 sit at
                              y 414.00153/473.9995/486.00153, which is not strictly descending. …")

       The values match to rounding, so the run happened — but the guard formats with `{y34}`
       (`Display`), which for f32 emits the shortest round-tripping form and **never** appends `.0`.
       `rustc` check: `format!("{}", 486.0f32)` → `"486"`; `format!("{:?}", …)` → `"486.0"`. The `.0`
       style in the comment is `verify.rs:407`'s `{:.1}`, borrowed from the r6 report's quotation of a
       *different* error. So the "verbatim" record is a reconstruction of one instrument's output in
       another instrument's format.
EVIDENCE: This project's own doctrine is "**Transcribe … never paraphrase**", and it ships
       `cite_check.rs::a_paraphrase_is_rejected_and_the_real_sentence_is_accepted` precisely because a
       paraphrase inside quote marks is the failure mode it does not tolerate elsewhere. r6 filed the
       structurally identical thing against itself ("the reason I wrote into the test doc AND into
       FOLLOWUPS.md §G-24 is not true, and it is the same class as the tenforty incident"). A B1
       record whose quoted evidence cannot be reproduced by running the code is the one artifact that
       must be exact — it is the only durable trace that the kill happened at all, and once I-1's
       standing tests exist the comment should just point at them instead.
       (Cosmetic corollary: a filer-facing refusal reading "y 486.00153/414.00153/473.9995" is noise;
       `{y34:.1}` matches `verify.rs` and would also make the comment true.)
```

```
SEVERITY: Minor
WHERE: crates/btctax-forms/src/form1040_full.rs:236-238, 240
CLAIM: The guard has TWO refusal powers, not one, and the second is new and unconditional: it makes
       `line34`, `line35a` and `line37` mandatory in every full-return map, on every return —
       including returns that would write none of them. And its ordering test is page-blind.
FAILURE: (a) **Mandatory-map widening.** `need(&map.line34, …)?` / `line35a` / `line37` now run before
       the branch. Previously a map lacking `line37` filed refund returns fine and only refused owing
       ones; a map lacking `line34`/`line35a` refused only refund returns. Now any one missing refuses
       *everything*, including the exactly-even return where the whole block is blank by design.

       That is the fail-closed direction and I am not asking for it to be reverted — but it is a real
       coupling, and the coupling lands on an ELECTION. r6's own last Nit is that line 35a ("Amount of
       line 34 you want refunded to you") is a choice the software makes without asking, and belongs
       in §G-11. If anyone acts on that by unmapping `line35a` — the obvious way to stop making an
       unasked election, and exactly what the map already does for 35b/35c/35d and 36
       (`f1040.map.toml:87-90`, "UNMAPPED ON PURPOSE") — the guard hard-refuses **every 1040 in the
       product**, owing and refund alike, with a message about geometry. Nothing records that
       dependency; the guard's comment says only that it "never looks at what we wrote".

       (b) **Page-blind.** `cy_of` compares raw PDF y across whatever pages the map names. The premise
       at :239 — "The form prints them in this order down the page, so their centres strictly descend"
       — silently assumes one page, and nothing checks it. A `map.line37` aimed at a **Page1** amount
       cell (page 1's money column runs to low y) satisfies `y34 > y35a > y37` and passes. Nor does
       `verify_flat` catch it: for money cells the expected page comes from `page_of(fqn)`
       (`cells.rs:23-29`), i.e. from the map's own string, so the page leg cannot see a cross-page
       mis-map, and the descent group is assigned by the *writer* (`GRP_P2_AMOUNT`), not by the actual
       page. This is not a regression — the pre-change writer had the same hole — but the guard is the
       piece now carrying the "map-independent, fails closed" claim for this block, and `page_of` is
       already imported at the top of this very file (:28). One `==` closes it.
EVIDENCE: form1040_full.rs:28 (`use crate::cells::{page_of, …}`), :236-238, :239-247;
       cells.rs:23-29; verify.rs:390-413; f1040.map.toml:87-90; r6 report, final Nit.
```

```
SEVERITY: Nit
WHERE: crates/btctax-forms/src/form1040_full.rs:240
CLAIM: The guard compares with a bare `>` where the sibling instrument requires a 1.0-point margin.
FAILURE: `verify.rs:407` is `if w[0].1 <= w[1].1 + EPS` with `EPS: f32 = 1.0` (verify.rs:14) —
       deliberately demanding real separation, because two money cells that differ by float noise are
       the same printed row. The guard's `!(y34 > y35a && y35a > y37)` accepts a 0.0001-point gap.
       Not reachable on the TY2024 map (the rows are 12 points apart, and an exact re-aim at the same
       widget gives equality, which `>` does catch), and the true values carry ~0.0015 of noise
       (486.00153 / 473.9995 / 414.00153) that a 1.0 EPS absorbs comfortably. Purely a consistency
       point: two checks in one crate expressing "strictly above" two different ways.
EVIDENCE: form1040_full.rs:240 vs verify.rs:14, 407.
```

---

## ALSO CHECKED, SOUND

- **Purity — "its only power is to refuse."** True, with the caveat in Minor-2 that it now refuses on
  two grounds. The block (form1040_full.rs:222-248) contains no `push_money`, no `writes.`, no
  `placements.`, and its only mutation is three `let` bindings inside its own scope. It reads
  `blank_fields` (the **blank** PDF, loaded at :96-97) — so it is strictly pre-write and cannot depend
  on anything we emitted, which is what makes the claim at :211-212 accurate.
- **Reachability — confirmed by execution, not inspection.** Plant `line35a ↔ line37` in
  `forms/2024/f1040.map.toml` → 4 tests fail with `Geometry("1040 refund/owe block is mis-mapped:
  lines 34/35a/37 sit at y 486.00153/414.00153/473.9995 …")`. Plant `line34 ↔ line37` → same guard,
  `y 414.00153/473.9995/486.00153`. Both restored from `/tmp` backups afterwards.
- **Does it hold the property r6 said was lost?** Yes, and I checked the harder half — whether the
  *rest* of the lost coverage matters. Pre-change, 34/35a/37 sat in `GRP_P2_AMOUNT` with lines 16–33,
  so each was also checked against line 33 and its predecessors, always. Post-change each is checked
  against 33 **only in the branch that writes it**. That residue is harmless because **detection is
  now exactly co-extensive with harm**: a mis-mapped `line34`/`line35a` produces ink only on a refund
  return, and on a refund return ordinals 13/14 are still compared against line 33 (ord 12); a
  mis-mapped `line37` produces ink only on an owing return, where ord 15 is still compared against
  line 33. The one comparison no branch can ever make — 34/35a **against** 37 — is precisely what the
  guard adds, unconditionally, including on the exactly-even return that writes neither. Worked
  examples I traced: `line34 → f2_22` (line 33's own box) is caught by descent on a refund return
  (498 ≯ 498); `line34 → line 20's box` passes the guard but is caught by descent on the only branch
  that would print it; `line35a → line 36's box` and `line37 → line 38's box` pass **both** — but they
  passed pre-change too, being order-preserving, so no coverage was lost there. The guard is a
  faithful restoration, not a lookalike.
- **Which cells `fields()[0]` reads.** All three map entries are bare strings ⇒ `MoneyCell::Single`
  (`f1040.map.toml:83-85`), so `fields()[0]` is the cell itself. The comment's fallback reasoning for
  `MoneyCell::Pair` ("a pair's dollars field carries the row's y") is correct against
  `map.rs:221-226` and `cells.rs:57-67`, which put the dollars field in the descent placement and the
  cents field in a `free` one.
- **The form's own order.** `design/forms/extract/f1040--2024.txt` lines 114/115/119/120/122: 34
  "amount you overpaid" → 35a "refunded to you" → 36 "applied to your 2025 estimated tax" → 37 "the
  amount you owe" → 38 "Estimated tax penalty". The guard's premise is the form's printed order,
  transcribed, not inferred.
- **Nothing reads these cells back differently because of the guard.** It emits no field, so
  `transcribe::extract_lines`, `assert_only_filled`/`no_unmapped_filled`, the oracle harness and
  `render.rs` see byte-identical output — proven by `full_return_form_fills_are_byte_deterministic`,
  `the_whole_packet_is_byte_reproducible` and the 8 `golden_packet::diff_shard_*` passing unchanged
  with the guard deleted. The range touches no reader (4 files, none of them one).
- **The new refusal path propagates correctly.** `fill_form_1040_full` (lib.rs:256-264) → `packet.rs:74`
  uses `?`, so a `Geometry` error aborts the whole packet rather than emitting a return with a form
  missing. Fail-closed at the only two call sites.
- **`.and_then(|f| f.cy())` on a rect-less field** yields the distinct, named message "…maps to {fqn},
  which has no rectangle in the blank form" rather than a silent pass — the right direction for a
  parent/kids field or a spacer.
- **`MAX_UNLOCATABLE`, rules (2b)/(2c), the checker's kills** — Lens B, untouched.
- **Suite at pristine HEAD:** `cargo nextest run -p btctax-forms` → **233 run: 233 passed, 4 skipped.**

## WHAT WOULD MAKE THIS REVIEW WRONG

1. **If "the KAT suite is its kill-test" is read as a claim about *coverage* rather than about B1.**
   The sentence is literally true in one reading — the guard does sit on every fill, so every KAT
   executes it. My Important rests on B1's stated test being *"which test reds when this checker is
   removed?"*, and on that reading the answer is measured, not arguable: none. If the project's
   standard is instead "was the discrimination ever witnessed by a human", the author witnessed it and
   this drops to a Minor about a comment that overstates.
2. **If the residual coverage I cleared is not actually co-extensive with harm.** My "detection = harm"
   argument depends on the mutual exclusivity holding — i.e. on `overpaid`/`owed` never both firing,
   and on neither branch writing when both are false. That is true of the current code
   (`line33 > line24` / `line24 > line33`) and was r6's own subject. If a future edit makes the two
   arms overlap, or writes 35a outside the `overpaid` arm, my clearance expires with it.
3. **If a rotated page is reachable.** I treated `/Rotate` as out of scope because it would equally
   break `verify.rs`'s pre-existing descent leg, so it is not a regression in this range. On a rotated
   or upside-down blank the guard would refuse a *correct* map — a fail-closed direction, but a
   refusal of a return it should not refuse, which is one of the brief's questions and which I have
   answered only for the un-rotated case.
4. **If the Minor-2 unmapping scenario is judged fanciful.** It depends on someone acting on r6's 35a
   election Nit by removing the map entry rather than by adding a filer question. If §G-11 lands as
   `LineEntry::Blank` instead, the coupling never bites.

---

## Tree state

**Clean of my work.** I mutated exactly two tracked files, both backed up with `cp` to `/tmp` and
restored with `cp` (never `git checkout --`):

- `crates/btctax-forms/forms/2024/f1040.map.toml` — two planted swaps; restored, md5
  `9d40bf9cf60bbd22c38a3e4a8f1f19b8` identical to the pre-mutation backup.
- `crates/btctax-forms/src/form1040_full.rs` — guard block deleted for the B1 mutation; restored,
  `git diff --stat` empty.

`git diff --stat -- crates/btctax-forms/` is **empty**, and the forms suite is green at 233/233.

★ **Not mine, and untouched:** the shared worktree showed other files changing under me mid-review —
first `crates/btctax-core/src/tax/testonly.rs` + `crates/btctax-adapters/tests/shipped_tables_are_the_validated_tables.rs`
(a §G-25 MFS/HoH-brackets edit), later `FOLLOWUPS.md` + `crates/xtask/src/line_coverage_check.rs`.
That is a concurrent agent in the same tree, the same condition r6 reported. I left all of them alone.
