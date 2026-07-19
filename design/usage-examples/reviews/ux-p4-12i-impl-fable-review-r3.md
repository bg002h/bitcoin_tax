# UX-P4-12(i) — Fable re-review r3 (residue fold `11f861d`)

**Scope honored:** this r3 audits only the r2-residue fold (`004e8ee..11f861d`); the r2-certified fix itself was re-checked only for disturbance. Repo `/scratch/code/bitcoin_tax`, HEAD `11f861d`, tree clean before and after my runs.

## What I verified (re-derived from current source)

**1. The new assert (r2-N1) is non-vacuous, correct, and I killed the mutant myself.**

- **Vacuity mechanics confirmed in source.** `TaxInputsFormState::fresh` initializes `dirty: false` (`crates/btctax-tui-edit/src/edit/form.rs:286`), and `form.working = Some(ri)` is a direct field write that never sets it. I traced the whole key path: `s` → `open_tax_inputs_commit_modal` (`main.rs:~1200`, gates only on `working.is_some()`; no flush, no dirty touch) and Enter → the modal dispatch (`main.rs:~999`) → `commit_tax_inputs` directly. The section-change debounce flush fires only on `Left`/`Right`/`Tab`, so **nothing clears `dirty` on this path except the NoTables arm's `if saved { form.dirty = false; }`** (`main.rs:~1320-1324`). Without the `form.dirty = true` precondition the assert would trivially hold.
- **Both directions proven empirically.** (i) Deleted the `if saved { form.dirty = false; }` block → `tax_inputs_commit_non_2024_is_notables_and_writes_nothing` **reds with exactly the new assert's message** ("a saved NoTables draft is not dirty…"). (ii) With the block still deleted, I *also* removed the `form.dirty = true` precondition → the KAT **passes (vacuous)** — demonstrating the precondition is precisely what makes the arm's dirty-clear observable, as the comment claims. Restored from a cp-backup; `git diff` empty; KAT green on restored source.
- **The precondition disturbs nothing else.** Neither `open_tax_inputs_commit_modal` nor `commit_tax_inputs` reads `dirty`; every other assertion in the KAT (modal opens/closes, status content, committed-row absent, draft persists) passed intact in all runs, and two other pre-existing KATs already use the same `form.dirty = true` setup idiom.
- **The assert reflects real behavior, and the comment's claims are accurate.** The dirty-clear is `saved`-gated; `ri` is the synchronous clone of `form.working` handed to `form_save_draft`, so "the draft now matches the working copy" is truthful. The "close hint reads '(autosaved)'" claim maps directly to `draw_edit.rs:2224-2228` (`close_hint` keys on `form.dirty`: dirty → "save & close", clean → "close (autosaved)"), and the mirror claim matches `flush_tax_inputs_draft` (`main.rs:909-914`).

**2. The FOLLOWUPS (i) block is now accurate.** The quoted message matches the shipped format string character-for-character once Rust's `\`-continuation collapses ("… (v1: TY2024) — inputs SAVED as a draft; finalize when tables publish."). The "≤ ~104 chars" claim is exact: 104 chars for a 4-digit year (failure variant 95). The new review trail is factually consistent with the persisted files: r1 verdict "NOT GREEN — 0C/1I" with I-1/M-1/N-1 at those severities; r2 verdict GREEN with M-1(r2)/N-1(r2)/N-2(r2) residue — the trail's items map one-to-one. Cited commits resolve correctly (`bd73968` = the (i) feature, `c2597ad` = the r1 fold), and `reviews/ux-p4-12i-impl-fable-review-r{1,2}.md` both exist. No stale copy of the superseded 162-char message survives outside the (legitimately historical) review files — the two `resolve.rs` "v1 supports TY2024" hits are a different, pre-existing CLI string this work never owned.

**3. The CONTINUITY (i) line is now truthful.** `grep` finds no "DEFERRED", "verdict SOUND", "USER INPUT NEEDED", or align-to-CLI text anywhere in `design/usage-examples/CONTINUITY_post_v070.md`. The replacement records DONE + user-decided 2026-07-19 + r1 0C/1I → r2 GREEN + UNPUSHED, with resolving citations. **UNPUSHED verified:** `origin/feat/post-v070-product-cycle` is at `6047b27`; `bd73968` through `11f861d` are all ahead of it. The `[[full-return-store-before-tables-policy]]` pointer lands on a memory doc that now prescribes KEEP-I-11, and its quote fragment was aligned per N-2(r2) ("finalize when tables publish", no "the year's"— old fragment gone).

**4. No regression; nothing else disturbed.** `git diff c2597ad..11f861d -- crates/btctax-tui-edit/src/main.rs` is **exactly** the test precondition (+2 comment lines) and the new assert (+3 comment lines) — zero product-code lines, so the I-11 guard, NoTables message, and draft-save are byte-identical to `c2597ad`. `make check` run by me: **2059/2059 passed + clippy `-D warnings`, exit 0**; `cargo fmt --check -p btctax-tui-edit` clean (the fmt/CI-only trap does not recur here; the other two touched files are Markdown).

**5. Hunt for record errors in the fold itself:** the "all mutation-proven" claim on the four KAT pins holds (guard-kept + draft-persists re-killed at r1, status/render mutant re-killed at r2, not-dirty killed by me now); "see (i) r1-M1 below" points at the trail block that is indeed directly below; the r2-residue line's "done in this touch" items are all actually done. No dangling cross-reference, no over-claim, no test passing for the wrong reason found.

## Findings

**CRITICAL — none.**

**IMPORTANT — none.**

**MINOR — none.**

**NIT — none.** (The mutation-check comment at `main.rs:10275-10276` quotes the product block verbatim and will drift if that block is ever reshaped — inherent to such comments, not a defect, and today it is exact.)

## VERDICT

**GREEN — 0 Critical / 0 Important.** The residue fold is clean: the new assert is genuinely non-vacuous (mutant killed and vacuity demonstrated empirically, both by me), the FOLLOWUPS quote and review trail now match the shipped string and the persisted reviews, the CONTINUITY resume hazard is gone and its replacement is truthful (including UNPUSHED), and the fold's `main.rs` delta contains no product code — the r2-certified guard/message/draft behavior is byte-identical. Full gate green: `make check` 2059/2059 + clippy, fmt clean on the touched crate.
