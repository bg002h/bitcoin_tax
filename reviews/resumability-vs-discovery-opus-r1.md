# Resumable interview vs. discoverable documents — Opus architect consult (2026-07-30)

**Question:** the minimal persisted state that makes an interrupted tax interview resumable, given
that every persisted artifact is a potential exhibit.
**Hypothesis put up for destruction:** *persist answers only; derive everything else.*
**Reviewer:** Opus, independent, read-only, 20 tool calls.
**Status:** persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2.

---

## Verbatim output

```
VERDICT: Right in direction, incomplete in one load-bearing way — "persist answers only" is correct, but today's answer type cannot represent a complete answer, and persisting progress is the *symptom* of that, not the cure.

MINIMAL STATE:
  ON DISK, per (tax_year, QuestionId): the answer, plus WHEN it was given, plus WHICH WORDS were asked
    (prompt hash — r1's named bend). Nothing else about the interview.
  The answer needs a state it does not have: `Given(v) | Declined | Shredded{..} | absent`. Today a
    class-(B) skip and a never-reached question are BOTH `None` (`answer.rs:169,196` break without
    setting) — the CLAUDE.md blank-vs-blank defect, at the question layer.
  There is also NO timestamp anywhere: `return_inputs` is `(year, inputs_json, schema_version)`.
  MUST NOT go on disk: progress, position, an ordered "what remains", a superseded answer value,
    a half-typed token, or any full-blob shadow of `ReturnInputs`.
  UX cost of deriving the rest: **zero**, once `Declined` exists. `live_questions(ri)` is already a pure
    function over `&'static` registries in fixed order, so resume = recompute and filter. Without
    `Declined` the cost is real and unfixable: skipped questions re-appear as "remaining" forever.

DRAFTS: A draft is an EDIT TRANSACTION, not a save file — so it is legitimate only when there is a prior committed return worth protecting from a half-finished edit, and must not exist at all for fresh authoring (there, it is pure residue: a second full copy of SSNs, DOBs and income holding values the filer never adopted).
Scoped that way it is durable and defensible. Unscoped — today's shape — it is the one artifact in the design whose entire reason for existing is to preserve values the filer passed through and discarded.

BAK: **Keep, unscoped.** It is one generation, it AGES AUTOMATICALLY on every save, and it is the corrupt-ciphertext net for a single whole-image vault (`vault.rs:215 restore_from_bak`) where a torn write loses everything. Scoping it out of interview writes would leave the *hardest-to-reconstruct* data as the only unprotected write path.
Counter-intuitive but decisive: committing per answer makes `.bak` hold "one answer ago" instead of "the entire pre-interview state," so the fix in MINIMAL STATE strictly *improves* `.bak`'s exposure. The property to pin is that the generation count is exactly 1 — a later "safer" `.bak.1` rotation would silently falsify G-14's shred with nothing going red.

THE TEST: Three questions, per artifact. Keep it only if it passes 1 and 2 and fails 3.
  1. Is it a decision the filer ADOPTED — or a state they passed through? (passed through ⇒ don't persist)
  2. Does it answer a question a reviewer would ask anyway ("did you inquire?") — or does it CREATE one ("why did this change?")? Protective artifacts shrink the question space; harmful ones expand it.
  3. Is it reconstructible from what you must keep regardless? (yes ⇒ derive it, persisting adds risk at zero information gain)
Committed answer + date + prompt-hash: ✓✓✗ keep. Shred tombstone: ✓✓✗ keep. "What remains": fails 3. WIP draft blob: fails 1 and 2.

- **The draft is not deletable today, and that is live, not future.** There is ZERO `VACUUM`/`secure_delete` in the tree, and `save()` serializes the whole image (`vault.rs:242 db_to_bytes`). So `commit`'s `delete_draft` frees a row pointer while the JSON — SSNs, DOBs, every superseded figure — rides in SQLite free pages into every subsequent encrypted image, indefinitely. The draft's real retention is **unbounded and invisible**, strictly worse than `.bak`'s bounded one generation, and it is the opposite of what the code reads as.
- **The CLI's save-at-end is a defect, and the argument is correctness, not convenience.** Re-answering is RE-TESTIFYING: a filer who loses 20 answers and redoes them from memory at speed answers worse the second time, and understatement risk goes up. Its "nothing persisted on Ctrl-C" virtue is also partly illusory — `.bak` still holds the prior full answer set. **Yes, CLI and TUI should share semantics**, and the shared semantics is *commit per atomic decision* — which is neither of today's two.
- **The draft table's root cause is precedence, not durability.** `answer.rs:96-98` documents it: a near-empty `ReturnInputs` row outranks the user's `tax_profile` at `resolve`, so a partial commit silently converts a computable year into a refusing one — and `input_form_store::commit` therefore screens at STORE time, though `resolve.rs:96` and `packet.rs:124` already screen fail-closed at USE time (which is why the CLI can and does write unscreened rows). Fix precedence and the store-time screen, and the fresh-authoring draft has no job left. Honest trade: per-field commit while editing a *good* existing return can degrade it mid-session — that is precisely the case the scoped edit-transaction draft covers, and the only one it should.
- **A persisted "what remains" is forbidden on correctness grounds, which is stronger than the discovery argument** — it is a second copy of the liveness predicate, the exact invariant r2 protected by keeping `live`'s one-argument signature. It drifts the moment a registry entry or an answer changes. Always recompute. (G-13a's real fix is aggregating `live_questions`, not storing it.)
- **Sequencing consequence, not a re-litigation:** `answered_on` cannot be back-filled. If the answer date and prompt-hash do not land with **G-15** (the schema bump already scheduled first, free while there are no users), then every pre-existing answer's G-14 tombstone must carry an absent or fabricated date — a diligence record that lies is worse than none. Move those two fields into G-15's schema change. Also worth stating plainly in the shred UX: `export_snapshot` writes plaintext `snapshot.sqlite` outside the vault, converting every artifact above into an exhibit in one command, beyond shred's reach. **Practitioner judgement, genuinely:** how long to retain answers post-filing, and whether an "offered the deduction, declined" record helps or hurts in a given examination posture, are a preparer's calls — the architecture should make retention the filer's decision, not pick a window.

WHAT WOULD MAKE THIS WRONG: This assumes an interview answer is atomic and individually meaningful — if answers ever become interdependent enough that a partially-answered set is *misleading* rather than merely incomplete, then all-or-nothing commit becomes the correct semantics and the draft (or an in-memory buffer) is required to hold the intermediate.
```

---

## Verification (orchestrator, before folding)

Claims re-run rather than accepted. **All hold**, and the most serious one is a LIVE defect verified
end-to-end.

| claim | verdict | evidence |
|---|---|---|
| ★★ a class-(B) **skip** and a **never-asked** question are both `None` | ✅ | `answer.rs` date branch `Ok(None) => break` (no set); yes/no branch comments *"A bare Enter KEEPS whatever is on file (may be `None` ⇒ skip)"* then `break`. The record cannot tell *declined* from *never asked* |
| `return_inputs` has **no timestamp** | ✅ | `(year INTEGER PRIMARY KEY, inputs_json TEXT NOT NULL, schema_version INTEGER NOT NULL DEFAULT 0)` — no date column |
| `restore_from_bak` is a real recovery net | ✅ | `vault.rs:215`, called at `:157` |
| ★★★ **the draft is not deletable today** | ✅ **LIVE DEFECT** | `delete_draft` = plain `DELETE FROM return_inputs_draft`; **`VACUUM`/`secure_delete` = 0 occurrences tree-wide**; `db_to_bytes` = `conn.serialize(Main)` → whole image *including free pages*; `save()` encrypts that image |

★★★ **The verified chain:** a `DELETE` frees SQLite pages without overwriting bytes → the whole image
is serialized → encrypted → written. So **superseded draft JSON (SSNs, DOBs, discarded income
figures) rides in free pages into every subsequent vault generation, indefinitely.** `delete_draft`
reads as a deletion and is not one at the byte level. Filed as **§G-16**.

★ Note the same missing `VACUUM` is what consult r2 already required for §G-14's shred — **one fix
serves both**, and §G-16 makes it urgent rather than prospective.

★ The skip-vs-never-asked collapse is *documented as intentional* in `answer.rs` ("Silence is a
legitimate outcome here — unlike a declaration"), and that reasoning is correct for class (B): the
silence is lawful. The architect's point stands anyway — **lawful silence and absent silence are the
same bytes**, so the record cannot supply the provenance §G-13 needs. It is `blank-is-the-normal-case`
one layer up, at the question rather than the form.
