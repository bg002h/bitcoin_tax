# Fable consult (r2) — what ships, and what must be true before `income scrub` does

Consulted: 2026-08-03. Inputs: `reviews/scrub-CONSULT-fable.md` (brief), `reviews/scrub-r1-workflow.md`
(settled findings, read in full), and the live repo at `main` = `2449ee4`. Every NEW fact asserted
below was checked against git or source during this consult; settled findings were not re-derived.

## Verdict

**Release 0.16.0 now, from `origin/main` (06923c7) — the CTC fix, `--version`, and the fixtures —
without `income scrub`. Scrub is unpushed, gate-red, and its worst defect is an unmade scope
decision; it ships in its own later release only after a fail-closed re-scope and a
class-preservation invariant reach 0C/0I.**

## The fact that changes the shape of the decision

The brief says the six unreleased commits are "all pushed." **They are not.** Verified now:

```
main...origin/main [ahead 2]
  2449ee4  fix(scrub): two PII leaks and the overclaim that hid them
  31d5c79  feat(cli): `btctax income scrub` — hand a real return over without the PII
origin/main tip: 06923c7 (fixtures; includes eb4f9c5 CTC fix, ae13ac1 --version)
```

The two scrub commits are the *only* unpushed work, sitting on top of everything else, touching
files nothing else in the release depends on. So the earlier framing — "revert the two scrub
commits" — conceded too much: **nothing needs reverting.** Move the two commits to a feature branch,
reset local `main` to `origin/main`, and the release the owner asked for is already sitting on the
remote, gate-green. Scrub has never left this machine; holding it back costs one `git branch` and
loses nothing.

## Answers to the brief's questions

### 1. Does the CTC fix's real user harm justify releasing ahead of a fully-fixed scrub?

Yes — but the question embeds a false coupling. It would only be a dilemma if the CTC fix and scrub
had to travel together. They don't: they are severable at zero cost (verified above), so "release
the CTC fix now" and "don't release scrub yet" are the *same action*. The CTC defect is real
published harm (v0.15.0 tells a fully-phased-out filer their tax is overstated by up to $2,000/child
and sends them to file a Schedule 8812 that pays nothing — verified against `eb4f9c5`, which also
carries the production-reachability test the original lacked). Even under the "no users yet"
assumption, shipping that fix costs nothing and closes a live wrong-advice path in a permanently
installable artifact. What the CTC harm does **not** justify is dragging a red-gated command along
with it — and it doesn't have to.

### 2. Is `income scrub` salvageable in its current scope?

**Salvageable, yes — but only fail-closed.** The right shape is:

- **Inputs-only scope is acceptable as v1, but it must REFUSE when the year's return draws on the
  ledger** (non-empty disposals / income / removals / charitable gifts / digital-asset activity for
  the year — `assemble_absolute` takes `&LedgerState` as a required input, so "does the ledger
  contribute" is decidable). The refusal message names what does not travel. This is the repo's own
  doctrine applied: widening an exemption is never the safe edit; a file that silently reproduces
  half a return is the "blank that answers for the filer" pattern at file granularity.
- **"Emit both halves" — scrubbing the ledger — should be REJECTED as the fix, and the rejection
  recorded.** This is the judgment call the brief is really asking for, so plainly: for a *crypto*
  tax tool, the ledger's figures are not like W-2 figures. A `Disposal` carries exact sat
  quantities, exact dates, and wallet attribution (state.rs:145-170) — these are **public
  blockchain quantities**. Scrub's own invariant is "no figure moves"; you cannot
  figure-preservingly de-identify data whose figures *are* the identifier on a public chain. A
  "scrubbed" ledger stamped "safe to hand to a stranger" would be the most dangerous artifact this
  tool could emit — a false safety promise about data that chain analysis can re-identify. The
  honest end-state for ledger-bearing years may permanently be "this cannot be made safe to share;
  here is what you can share instead." That is a product/spec decision — which is exactly why it
  must not be made this week under publish pressure.
- With the ledger refusal in place, the current `--help` promise ("computes an IDENTICAL return")
  becomes *true for every year the command will serve* once the EIN/validity-class fixes land —
  which is a better outcome than weakening the promise to "identical given the same ledger" and
  shipping a file most of this tool's users can't use.

### 3. Is the assistant under- or over-weighting a risk?

**Under-weighted, three things:**

1. **The unpushed state itself.** The recommendation was framed as "revert," which sounds like
   undoing published work and invites resistance. The true cost of holding scrub back is one branch
   command. When the safe path is also the free path, say so — it dissolves the release-pressure
   argument entirely.
2. **The help text is the product.** For every other command, a defect is a wrong number; for
   `income scrub`, the *safety authorization* ("safe to hand to a stranger", "a guarantee held by a
   test, not a hope" — cli.rs:413-428, mirrored in the shipped man page) is the entire product.
   Publishing it false, then fixing it in 0.17.0, still leaves the false authorization in every
   installed 0.16.0 forever (yank ≠ delete). A wrong safety claim is the one artifact this tool
   cannot afford to have in the wild — worse than no command at all.
3. **The re-import data-loss path was filed MINOR and sits next to a CRITICAL rubric line.** A
   scrubbed file is schema-identical to a real one; `income import` is an unconfirmed whole-blob
   upsert; re-importing destroys the vault's real identity and the IP PIN (unrestorable), and the
   synthetic SSN is well-formed enough to *print on a filed 1040*. "Data loss" is Critical in this
   repo's own rubric. The provenance-marker + import-guard fix belongs on the must-fix list, not in
   the residue.

**Over-weighted, one thing, harmlessly:** the "real users NOW" urgency on the CTC fix. With no known
users, days do not matter — but since releasing without scrub costs nothing, the over-weight changes
no conclusion. The recommendation itself (ship without scrub) was correct; this consult sharpens its
mechanism and its conditions rather than reversing it.

**And one argument I steelmanned and reject:** "no users yet, so ship scrub now to start collecting
scrubbed bug reports early." The two Criticals mean the file either moves a figure in the
understating direction (the one direction this codebase promises never to go, in a file whose help
invites re-import) or omits the ledger that is most of a real crypto return — so the reports it
solicits would be wrong reports. A debugging tool that manufactures the defect it exists to
reproduce has negative value at any user count.

### 4. What ships, exactly

**0.16.0 = `origin/main` @ `06923c7`, unchanged:** `--version` (ae13ac1), the CTC advisory fix
(eb4f9c5), the nine-dependent fixtures (b46f608, 06923c7). No scrub, no new help text, nothing to
reword. Tag, publish dependency-first (not `--workspace`), verify via the sparse index, and close
the standing publish-hygiene items (temp crates.io token revocation) if still open.

**Scrub ships later, alone (0.17.0 or whenever green), and only when all of the following are true:**

1. **Scope is decided in writing first** (a short spec amendment, reviewed): inputs-only; refuses on
   ledger-bearing years; records *why* ledger scrubbing is rejected (on-chain figures are public
   quasi-identifiers — figure-preserving de-identification of chain data is a contradiction); never
   points a filer at unscrubbed `export-snapshot` as the workaround.
2. **One invariant, not eight patches: scrub preserves the EQUIVALENCE CLASS, never just the
   value.** Same canonical-identity partition (key `EinMap` on `canonical_ein`, non-canonicalizable
   stays non-canonicalizable) and same validity class everywhere (absent→absent, malformed→malformed,
   valid→synthetic-valid: SSN, EIN, IP PIN, business_description). Held by two class-level tests run
   over the whole fixture set — `screen_inputs(orig) == screen_inputs(scrubbed)` by variant, and
   `ReturnHeader::build(orig).is_err() == ReturnHeader::build(scrubbed).is_err()` — each observed
   RED first (B1). This closes both Criticals' mechanisms and the four-screen cluster at the root.
   (If the SSN-less-export normalization is deliberately kept for the common no-PII-yet case, it is
   a *recorded* exception with honest help text, per the r1 verifier's own temper.)
3. **The disclosure test is exhaustive and mutation-verified:** the spouse-clone plant reds;
   fixture constants no longer collide with scrub constants (Springfield/IL/62704); a `b_1099`
   fixture exercises the payer loop; dependent-count length asserted before the zip.
4. **Dependent DOB is dropped or year-quantized**, and the false "both are read" comment corrected.
5. **The file is safe as an artifact:** `--out` through `fsperms` owner-only write; a provenance
   header marking the file synthetic; `income import` refuses a marked file without `--force`.
6. **The round trip is held by a test** (emit TOML → parse → assert equality), pinning the
   toml-hoisting behavior the emitter silently depends on.
7. **Draft/parked-year coherence:** scrub routes through the input-form store like every other
   reader of the committed row, or at minimum distinguishes "parked" from "no inputs."
8. **Help + man regenerated to say only what is then true**, and a whole-branch (`main..HEAD`)
   review to 0C/0I, briefed on what r1 already covered so it spends its budget on the seams (B3).

## Process notes

- The three `reviews/scrub-*.md` files are untracked. Persist them (verbatim, per the standing
  rule) on the scrub feature branch — they are that branch's review record, not `main`'s.
- Branch mechanics, concretely: `git branch feat/income-scrub 2449ee4`, then
  `git reset --hard origin/main` on local `main` (after committing the review files to the feature
  branch so nothing untracked is at risk). Nothing is lost; nothing was ever pushed.

## One-line summary

The release and the risk were never coupled: the fix the owner wants is already on the remote, the
command that isn't ready has never left the machine, and the only thing scrub needs that a week of
fixes can't give it is a scope decision — so make that decision in a spec, not in a publish.
