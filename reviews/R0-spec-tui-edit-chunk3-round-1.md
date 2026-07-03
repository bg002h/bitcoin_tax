# R0 — Architect review: SPEC_tui_edit_chunk3 (round 1)

**Artifact:** `design/SPEC_tui_edit_chunk3.md` (DRAFT @ `6f88876`, baseline `7ba67a1`)
**Reviewer:** R0 (independent; architect). Source verified file-by-file against the working
tree at HEAD of `feat/tui-edit-chunk3` — every load-bearing citation re-checked.
**Verdict: NOT GREEN — 1 Critical / 7 Important / 10 Minor / 2 Nit.**

---

## Attest-atomicity ruling (the mandatory question first)

**The persist fn itself is a faithful mirror — the claim is CORRECT at the fn level.**
`persist_safe_harbor_attest` as specified (spec D4) reproduces `cmd::reconcile::safe_harbor_attest`
(reconcile.rs:540-563) exactly: Void first, then the re-attested copy via struct-update
(`timely_allocation_attested: true, ..prior` — preserving `lots`, `as_of_date`, `method`,
`pre2025_method`, matching reconcile.rs:551-554), the SAME injected `now` for both, both appends
on the one in-memory Connection, ONE `session.save()`. On save-`Err` the on-disk vault is
unchanged (atomic `Vault::save`, NFR2/NFR3) and both appends persist in memory — exactly the
CLI's failure surface. KAT-P2g's `decision_seq` assertions pin the ordering. Pre-flight arms
mirror reconcile.rs:476-538 exactly, including arm order (count → already-attested →
unconservable → not-timebarred → proceed) and the already-effective refusal (the I-2(b)/N-2
"no doomed Void appended" property).

**What is NOT sound is the post-Err lifecycle story around that fn** — see C1. The "nothing
persisted on Err" claim is true only until the next successful save of the same session, and
the specified remedy is physically impossible while the editor runs. That is where the
Critical lives, not in the append/save mechanics.

---

## Critical

### C1 — Attest save-Err story: the CLI remedy is unreachable (VaultLock) and the confirmed irrevocable batch silently piggy-backs on the next unrelated save

Spec (Hard Constraints; D3 Enter-arm): on `Err(e)` the flow closes, status =
`"Save error: {e} — … retry via CLI: btctax reconcile safe-harbor-attest"`, and D4 asserts
"the two appends are in-memory but the vault is pre-action on-disk."

Two defects, both verified against source:

1. **The CLI cannot run.** The editor's `Session` holds the store's exclusive `VaultLock` for
   the editor's entire lifetime (editor.rs:8-14, 59-69; session.rs:53-58 → `Vault::open`).
   A user following the status message gets `StoreError::Locked` until they QUIT the editor.
   The message never says to quit. (Quitting is also what makes the remedy SAFE: it discards
   the in-memory residue, so the CLI attest then operates on the clean pre-action vault.)

2. **"Nothing persisted" is unstable.** After the failed save, the confirmed Void+Attest batch
   sits in the in-memory Connection. Every `persist_*` fn ends in `session.save()`, which
   serializes the WHOLE in-memory DB — so ANY later successful confirmed mutation (a set-fmv, a
   donation-details upsert, …) flushes the irrevocable two-decision batch as a side effect, while
   the user's mental model (told "Save error … retry via CLI") is that the attest never happened.
   The resulting vault state is internally valid (the batch was typed-word-confirmed and is
   engine-correct), but an IRREVOCABLE §7.4 action landing un-announced, attached to an unrelated
   confirmation, is precisely what the "explicit payload-showing confirmation" guarantee and the
   §7.4 UX bar exist to prevent. The spec neither documents nor mitigates this. (Chunk 2b has the
   same latent residue-after-Esc hazard for single benign appends; §7.4 elevates it here.)

**Required fixes (all cheap, spec-level):**

- **Remedy string:** `"Save error: {e} — quit the editor now (the unsaved attestation is
  discarded on quit), then run: btctax reconcile safe-harbor-attest"`. Every other
  CLI-pointing status in the spec should likewise be audited for the lock (see M-list); for
  attest it is load-bearing.
- **Close the piggy-back hole.** Recommended: a session-dirty latch — after an attest save-`Err`,
  the editor refuses to open further mutating flows/modals (openers set status
  `"A failed attest save left unsaved decisions — quit the editor, then retry via CLI"`).
  Minimum acceptable alternative: document the flush behavior explicitly in Hard Constraints
  AND pin it with a KAT (attest-Err → unrelated confirmed mutation → save → assert post log
  contains Void+Attest+the new decision, and the derived status/model). The latch is strictly
  better: it restores "persisted only when a confirmation's own save succeeds."
- **Pin the reopen guard** (interacts with I5): after attest-Err, pressing `a` again must hit
  the session-sourced pre-flight and refuse ("already attested" — in-memory), appending NOTHING.
  Add this KAT. It is the guard that stands between the user and the double-batch state, whose
  true consequence the spec understates (see M2): both copies conserve → `effective.len()==2` →
  Hard `DecisionConflict("multiple effective SafeHarborAllocations")` + Path A
  (resolve.rs:958-967), and per resolve.rs:924-934 voiding EITHER copy then fires the §7.4
  conflict — an unrecoverable Hard-gated vault.

The close-on-Err / no-TUI-retry choice itself is **sound and correctly reasoned** (a
re-confirm retry would duplicate the batch); rated: right call, wrong surrounding story.

---

## Important

### I1 — select-lots wallet sourcing is broken for Gift/Donate removals (2 of 4 advertised target kinds), with zero KAT coverage of that path

D1: "`wallet` from `DisposalLeg.wallet`". True only for Disposals (state.rs:131). **`RemovalLeg`
has NO wallet field** (state.rs:148-163) — for the Gift/Donate rows that Claim F explicitly
includes, the spec provides no wallet source. Consequences as written: the LotsForm filter
`l.wallet == disposal_item.wallet` compares `WalletId` to `Option<WalletId>` (doesn't typecheck),
and any improvised `None` default makes every gift/donation dead-end at "No lots available for
wallet …". No KAT drives the FORM path for a removal (KAT-P2f seeds a Donation but calls the
persist fn directly; KAT-E2E-SL uses a sell), so the suite stays green while the flow ships broken
— or worse, a wrong improvised wallet shows the wrong wallet's lots and the user appends
guaranteed-`LotSelectionInvalid` (Hard) selections.

**Fix:** source `wallet` for ALL list items from the raw event —
`events_by_id(snap)[&item.event].wallet.clone()` (`LedgerEvent.wallet: Option<WalletId>`,
event.rs:297-304; the `events_by_id` helper already exists, main.rs:1765-1769). Define the
`None` case (match `l.wallet` against `Some(w)`; `None` → the existing "no lots" error). Add a
KAT that drives `s` → a **Donate** removal → LotsForm shows the correct wallet's lots → persist →
clean re-projection.

### I2 — Duplicate-LotSelection semantics mischaracterized as "FIRST-WINS / the first governs"

resolve.rs:787-800: the dup fires `DecisionConflict` on the SECOND decision's id **and then
`selections.remove(id)` drops the first as well** — "a conflicted disposal applies NEITHER
selection" (resolve.rs:762, 799). While both are live, the disposal falls back to METHOD ORDER;
the first re-applies only after the duplicate is voided. The spec asserts the opposite in four
places: Hard Constraints ("the first (failed-save) governs"), Claim F ("the first stays in
force" — self-contradicting its own "NEITHER actually applies" in the same sentence), the D4
doc comment ("The FIRST … stays in force"), and KAT-S3a step 4's parenthetical. The specified
assertions and remedy happen to be right; the model is wrong and will leak into code comments,
status wording, or a future "assert first applies" test.

**Fix:** correct all four sites to: dup → conflict on the dup's id; NEITHER applies (method-order
fallback) until one is voided; voiding the duplicate reinstates the FIRST. Add one sentence: if
the user EDITED picks before the retry, voiding the duplicate reinstates the ORIGINAL picks —
to keep the edited picks, void the first instead (the conflict clears either way).

### I3 — D2's donation-details reads violate KAT-G1 and contradict KAT-E2E-DD

D2 has main.rs calling `donation_details::get(session.conn(), …)` at list-open (O(n) per-item)
and again in `derive_donation_details_status`. **`conn(` is a persist-only token — forbidden
outside `edit/persist.rs` non-test code** (persist.rs:685). Simultaneously, D2 mandates "No
re-projection on save" — but `Snapshot` ALREADY carries `donation_details: BTreeMap<EventId,
DonationDetails>` (btctax-tui/src/app.rs:104-111, populated by `build_snapshot`, unlock.rs:177,185),
and if the flow reads the snapshot (the only KAT-G1-clean source), skipping the rebuild leaves it
stale → **KAT-E2E-DD step 4 (list shows "present"; form pre-populated) fails as designed.** The
spec mandates two incompatible behaviors.

**Fix:** (a) list + pre-population read from `snap.donation_details` — delete the per-item `get`
design and its O(n) justification; (b) on `Ok` REBUILD the snapshot exactly like every other flow
(the set-fmv Enter arm, main.rs:1318-1339) — uniform discipline, and the Forms tab freshness
paragraph ("stale until next vault open") becomes unnecessary — delete it; (c) derive the status
from the in-hand validated `details` (last-write-wins guarantees it IS the stored value; the disk
round-trip stays pinned by KAT-DD-PERSIST).

### I4 — KAT-G1 guard direction for the new side-table writer is backwards

D4/KAT-G1 note: "verify `donation_details::` is NOT in the forbidden token list; add to the
allowlist…". `persist_only_tokens` (persist.rs:685) is the list of tokens FORBIDDEN outside
`edit/persist.rs`; `tax_profile::set` is guarded BY BEING IN it. Parity requires **adding
`"donation_details::set"` to `persist_only_tokens`**. As written, the implementer can no-op,
leaving the new writer callable from anywhere while Task 4 claims "`donation_details::set`
likewise only in `edit/persist.rs`. KAT-G1 green" — unverified. Fix the note to a direct
instruction; keep Task 4's claim, now actually mechanized.

### I5 — Attest pre-flight sources are mixed; pin one fresh load

D3 step 1 loads events "from the in-memory session" but step 5 reads `snap.state.blockers` —
session events + cached-snapshot projection. They agree only when no unsaved residue exists.
**Fix:** run the whole pre-flight off ONE `session.load_events_and_project()` (the CLI's exact
shape, reconcile.rs:473-474; the method name carries no forbidden token, so it is KAT-G1-legal
in main.rs). This is not pedantry: the session-sourced steps 2–4 (count / already-attested) are
the only guard preventing a second Void+Attest batch after a failed save (see C1's unrecoverable
double-batch consequence). Add the KAT: attest-Err → press `a` → pre-flight refuses → log
unchanged.

### I6 — Missing KAT + understated warning: the post-attest void-REJECTED interaction

The chunk-3 flow creates the first TUI-reachable EFFECTIVE allocation, and the shipped 2b void
flow will offer it: `is_revocable_payload` includes `SafeHarborAllocation` (form.rs:822-836), the
list filter is raw-decision + non-voided only (main.rs:2353-2374), with the SHA warning
(draw_edit.rs:1415-1420) and `derive_void_status` arm 1 (main.rs:2410-2417). A confirmed void of
the attested allocation appends a doomed Void → `DecisionConflict` on the void's id
(resolve.rs:924-934) — **Hard, and permanent**: the void event is append-only, cannot itself be
voided (resolve.rs:312-321), and re-fires every projection → `TaxYearNotComputable` forever
(state.rs:46-49, 68). The spec asserts the §7.4 rejection correctly but (a) never tests it
through the TUI, and (b) the attest Info warning says only "any void attempt fires
DecisionConflict" — omitting that the attempt itself permanently Hard-gates tax computation,
which is exactly what the user must weigh BEFORE typing ATTEST.

**Fix:** add **KAT-E2E-ATTEST-VOID**: after KAT-E2E-ATTEST, drive `v` → the new attested
allocation is listed with the SHA warning → confirm → status is the arm-1 "Void saved, but
DecisionConflict fired — the target decision remains in force" (NOT "Voided…"); allocation still
effective; the doomed void's conflict present. And strengthen the Info-screen warning: "…any void
attempt fires a PERMANENT Hard DecisionConflict that gates tax computation (§7.4) — do not attest
unless the lot list and method match your filed return." (Optional FOLLOWUP: pre-filter effective
allocations out of the 2b void list — effectiveness is derivable from blockers — so the
permanently-damaging no-op is unreachable; not required for chunk 3.)

### I7 — The two typed-word KATs imply contradictory buffer semantics; one cannot pass

KAT-E2E-ATTEST steps 3–4 require the buffer PRESERVED on failed Enter (`"ATTES"` → error → type
`"T"` → submit). KAT-E2E-ATTEST-WRONGWORD requires it effectively CLEARED (`"attest"` → error →
"Type `ATTEST`" → submit — with a preserved buffer that yields `"attestATTEST"` ≠ `ATTEST`).
**Fix:** pin ONE semantics in D3's key table (recommend: buffer preserved + error shown — it
matches the substrate's FieldBuffer behavior), and rewrite WRONGWORD's script accordingly
(Backspace×6, then type `ATTEST`). Either choice is safe (the equality gate is unaffected);
the pair as written is unimplementable.

---

## Minor

- **M1 — Citation drift** (the spec claims write-time verification; these fail it):
  (a) "reconcile.rs:926-933" for the §7.4 void-conflict → **resolve.rs:924-933** (reconcile.rs
  has 900 lines); (b) "persist.rs:577" for `persist_only_tokens` → **persist.rs:685**;
  (c) "main.rs:326" for `appraiser_name` → main.rs:323-324; (d) "resolve.rs:330 BTreeSet insert
  is idempotent" cited for the attest re-void — allocation-targeted voids take the
  resolve.rs:322-328 arm into `allocation_voids` (a `Vec`; duplicates accumulate), never line
  330; the second void is inert because the prior is not in `effective` (step 5). Conclusion
  unchanged; fix the mechanism.
- **M2 — Double-batch failure mode mis-described:** "potentially firing `SafeHarborUnconservable`
  or Path-A fallback" — wrong: each copy conserves independently; the outcome is
  `effective.len()==2` → Hard `DecisionConflict("multiple effective SafeHarborAllocations")` +
  Path A (resolve.rs:958-967), with both copies then §7.4-unvoidable (step 5) — unrecoverable.
  State it precisely; it strengthens the close-on-err rationale.
- **M3 — KAT name collision:** KAT-P2f already exists at HEAD
  (`kat_p2f_void_lot_selection_clears_optimize_attest…`, persist.rs:1186). Re-letter chunk 3's
  strict-prefix KATs (e.g. P2g = select-lots, P2h = attest) and update the roster.
- **M4 — Layer arithmetic:** Hard Constraints declares 12 layers incl. `safe_harbor_attest_modal`
  (layer 9); D3 deletes that modal; the true final count is **11** (8 modals + flow + form +
  screen). Task 4's cross-check still says "12-layer". Make all three sites agree.
- **M5 — KAT-C2f under-specified:** the modal opens only when Σ picks == principal; the script
  types `"100000"` against an unspecified seed. Pin the seed principal = 100000 sat (or type the
  seeded principal).
- **M6 — KAT-E2E-SL is non-discriminating:** with a single lot, method-order fallback consumes the
  same lot — the test passes even if the LotSelection is silently dropped. Seed TWO lots and pick
  the non-FIFO one; assert the legs consume the chosen `LotId`.
- **M7 — Claim F rationale text:** fee-mini-disposition exclusion is via the
  `!fee_mini_disposition` flag (plus the acknowledged SelfTransfer under-inclusion) — NOT "the
  honoring filter": a TP8-(b) fee record shares the SelfTransfer's event id, and
  `honoring_principal(Op::SelfTransfer)` IS `Some(principal)` (resolve.rs:1008-1016). Outcome
  right; fix the mechanism sentence.
- **M8 — select-lots modal height:** "all picks listed individually" is unbounded; define the
  overflow rule (e.g. first N picks + "… and K more, {sat} sat total").
- **M9 — FIELD_CAP=64 vs CLI parity:** addresses and appraiser-qualifications free text truncate
  at 64 chars (form.rs:17, 35-38); the CLI accepts arbitrary length. Raise the cap for the
  free-text donation fields or record the parity limit in FOLLOWUPS.
- **M10 — Stale Advisory timebar on the voided prior (engine fact worth one sentence):**
  allocation-targeted voids never enter `voided` (resolve.rs:322-328 vs 847), so post-attest the
  voided prior is re-evaluated every projection and keeps firing `SafeHarborTimebar` on ITS id.
  KAT-E2E-ATTEST step 6 and `derive_attest_status` are correctly keyed to the NEW id — add an
  explicit note so an implementer doesn't "fix" the stale advisory or widen an assertion to
  "no timebar anywhere" (which would fail).

## Nit

- **N1** — `FieldBuffer::push_str` (KAT-V-DD-4) doesn't exist; the method is `FieldBuffer::set`
  (form.rs:47).
- **N2** — "state.rs:134-178" → 133-179 (Disposal starts at 133; Removal ends at 179).

---

## Verified sound (checked against source; no finding)

- **Attest persist/pre-flight mirror** — see the ruling above.
- **optimize_attestation is optimize-only:** `reconcile select-lots` (reconcile.rs:330-352) never
  touches it; only `optimize accept --attest` co-persists the row (cmd/optimize.rs; the narrow
  per-disposal guard). Clearing on void is 2b's shipped `persist_void` (persist.rs:197-217),
  pinned by the existing KAT-P2f. The spec's scoping call is correct; KAT-E2E-SL-VOID re-pinning
  it for the select-lots case is good.
- **Typed-word ATTEST gate:** case-sensitive equality with trim, error path keeps the step open,
  Esc steps back one level, `q` swallowed — meets the irrevocable-flow bar (modulo I7's script
  fix); no separate modal layer is a justified deviation (the TypedWord step IS the gate).
- **Claim G:** Donation-only filter mirrors reconcile.rs:600-631 (projected `state.removals`,
  Gift → usage error); 10 fields / 2 required matches donation.rs:17-48 + clap (main.rs:308-330);
  `is_review_complete` arms match donation.rs:68-79; no already-set exclusion is correct for a
  last-write-wins upsert; the modal's "last-write-wins; not a decision event" footer is the right
  chunk-1 discipline.
- **Claim H:** timebar-only cure, Advisory severity (state.rs:78), arm order, and the
  already-effective refusal all match reconcile.rs:495-538 and resolve.rs:855-898.
- **Claim F pre-filters:** voided/already-selected sets mirror the shipped 2a/2b prefilter style;
  dup prevention via the list is correct (resolve fires conflict on the second id); Σ==principal
  at submit mirrors resolve.rs:811-823; deliberately NOT pre-validating per-row against displayed
  `remaining_sat` is the right call (post-consumption display would false-reject; CLI parity per
  reconcile.rs:325-329) and the display caveat + FOLLOWUP are honest.
- **Keybindings:** `s`/`d`/`a` free at HEAD (main.rs:193-222); dispatch-order extension follows
  the shipped 9-layer pattern; Err-arm keep-form-open for D1/D2 matches the shipped set-fmv arm
  (main.rs:1318-1344) including `now`-at-Enter injection.
- **KAT-DD-PERSIST** degenerate strict form (log length unchanged) matches KAT-P1's discipline.
- **Scope:** chunks 4/5, import-selections, safe-harbor-allocate, viewer freeze — correctly out;
  no btctax-core/btctax-cli changes needed (all types/accessors exist, incl.
  `Session::donation_details()` and `Snapshot.donation_details`).

---

## Gate decision

**BLOCKED (1C / 7I).** All findings are spec-text fixes plus KAT roster additions; nothing
requires a design restart. Re-review required after fold (§2 loop).
