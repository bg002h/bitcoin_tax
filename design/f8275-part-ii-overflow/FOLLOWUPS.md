# Form 8275 Part II narrative overflow — FOLLOWUPS

Filed at the whole-branch two-lens re-review of `fix/f8275-part-ii-overflow` (round 2). None of these
block the fix that shipped; each is recorded here so it is not silently dropped. No owning phase exists
yet for this cycle beyond "ship" — these are ownerless residue until a future cycle picks one up.

Legend: **[open]** not started · **[closed]** burned down.

> ## ✅ NOT PRUNED — all seven items below remain LIVE (noted 2026-07-27)
>
> Recorded here because the two sibling Approach-B registries
> (`design/defensive-filing-wizard/`, `design/stale-snapshot-latch/`) *were* pruned on this date and
> marked historical. **This one was not.** Every item below is `btctax-forms` / `btctax-cli` — the
> Form 8275 fill layer and the promote chokepoint — which is kept intact under **all three** branch
> outcomes and both paths of the open architecture decision. Nothing here depends on the UI.
>
> Indexed from the live root registry: **`FOLLOWUPS.md` §"APPROACH-B ARCHITECTURE DECISION +
> POST-WIZARD RECONCILIATION"** → G-1. Burn these down from this file as normal.
>
> (Note: this cycle *displaced* Approach-B sub-project 3 — VARIOUS multi-date 8949 rows — which is
> separately CLOSED as unnecessary with recorded reopen criteria in `DESIGN.md` §4. That closure is
> unaffected by the architecture decision.)

## Round 2 follow-ups (not built in this pass)

- **[open] Per-item Part II numbering (finding 4's "more faithful long-term shape").** The bundled
  Rev. 10-2024 PDF's XFA numbers Part II's 6 lines (`p1-t80[0]`..`p1-t85[0]`) beside Part I's 6 rows
  (`Line1PartII`..`Line6PartII`, confirmed by decompressing the asset's `template` XFA packet). The
  correct long-term shape is ONE explanation per numbered line, matched to its own Part I row — not the
  combined-narrative-to-line-1-then-Part-IV shape this fix ships. Building it needs a `Part1Item`-to-
  narrative correlation this crate does not have today (T13's `disclosure_8275` joins ALL promoted
  tranches' narratives into one string with no per-item boundary preserved) — a taxonomy change on a hot
  type (`whole-surface-sweep-on-taxonomy-change` prices this at ~4 review rounds). Reopen when either (a)
  a filer has enough promoted legs in one year that per-item attribution genuinely matters, or (b) the
  `Part1Item`/narrative correlation gets built for another reason.

- **[open] Record-time Part II length bound in `plan_promote`.** The narrative is immutable once
  recorded (the vault is append-only; `promote_tranche` refuses to re-promote an already-promoted
  tranche), so the ONLY remedy for an overflowing narrative discovered at export time is void-and-redo
  with a shorter `--part-ii-file` (see `crates/btctax-cli/src/cmd/admin.rs`'s `part_ii_overflow_message`).
  A record-time bound (reusing `btctax_forms::part_ii_capacity_check`, called from `cmd::promote::
  promote_tranche` / `chokepoint::mod.rs`'s `plan_promote`) would catch this before the filer walks away
  thinking the promote succeeded. Not built this round: it needs a `year` at record time (today only
  `tax_year` is threaded through at declare/promote time via the tranche's window, not guaranteed to
  match the eventual DISPOSAL year the narrative actually gets filed against — `disclosure_8275` scopes
  by the year the promoted leg is DISPOSED, which can differ from the promote's own recording year), so
  it is a real design question, not a one-line addition.

- **[open] The unbreakable-token overflow path reports an inflated, not exact, row count.**
  `wrap::wrap_part_ii`'s `too_wide` branch (a single run of non-whitespace text wider than any one
  line's budget) reports `rows_needed = lines.len().max(capacity + 1)` — a value GUARANTEED to read as
  "will not fit," not the true (undefined — no finite line count would ever hold an unbreakable token
  wider than every available line) need. Practically unreachable with real English prose (a single
  "word" would need to run ~110+ characters with no whitespace at 8pt in a ~514pt line), but the number
  is fabricated in that branch, matching this repo's own `untested-guard-pattern` caution about numbers
  nobody has verified. Low priority; tighten the message wording (or drop the number in that branch) if
  it ever proves reachable in practice.

- **[open] A defensive Part II emptiness re-check at the fill layer.** `fill_form_8275_inner` trusts
  that `printed.part_ii` is non-empty by the time it is called (BG-D7 refuses an empty narrative at
  record time, and `promote_export_gate`'s completeness check refuses an incomplete disclosure before
  export). `wrap_part_ii("", ...)` already degrades gracefully (writes nothing, no panic), so there is
  no CRASH risk — but there is also no explicit refusal naming "empty Part II" at the fill layer itself
  if some future caller ever reaches it with an empty string bypassing both upstream gates. Low priority
  defense-in-depth, not a live gap today.

- **[open] Document the ~28-line / ~3,700-character Part II+IV ceiling in `cli.rs` and the man page.**
  `btctax reconcile promote-tranche --part-ii-file` has no stated length guidance today; a filer writing
  a genuinely long "lost records" narrative has no way to know the practical ceiling before recording it
  (and — per the record-time-bound follow-up above — recording it too long today means void-and-redo).
  Add a line to the `--part-ii-file` help text / man page section once the wording is settled.

- **[open] `Form8275Map::field_names()` should delegate to `narrative_continuation_fields()`.** Both
  currently hand-roll the same `part_ii_narrative` + `part_ii_continuation` + `part_iv_continuation`
  concatenation (`crates/btctax-forms/src/map.rs`). Cosmetic de-duplication, no behavior change — filed
  rather than done inline to keep this round's diff scoped to the review's actual findings.
