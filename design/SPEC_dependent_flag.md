# SPEC — P8a: the claimed-as-dependent flags (a CRITICAL in shipped code)

*Split out of `SPEC_input_surface.md` on the user's call (2026-07-14): **a wrong number on a filed return
must not wait on a UX feature.** The body below is D-8 verbatim from that spec, which was reviewed to
**0 Critical** across rounds r4–r7 (`design/input-surface/reviews/SPEC-P8-fable-review-r{1..3}.md` +
the r4–r7 fold commits). Ceremony scaled down per `STANDARD_WORKFLOW.md` §8; the GATES are not.*

---

## 1. Why this ships alone

btctax **silently understates a dependent filer's tax**, and it does so in released software. This has
nothing to do with templates, TOML, or the input surface — that work merely *found* it. It is
self-contained (a type change, a store migration, a refusal, and a recovery command) and it should not sit
behind a feature that has not been written.

The rest of the input surface (`income template`, `income import` screening, `set-pii`) continues in
`SPEC_input_surface.md` and depends on **nothing** here.

## 2. Scope

**IN:** the store migration · both claimed-flags → `Option<bool>` · the new UNANSWERED refusal ·
consumers testing `== Some(true)` · **`income answer`** (the recovery path — without it, a user with a
stored row and no TOML file is stranded on a refusing year).

**OUT:** everything else in `SPEC_input_surface.md`.

⚠️ **This touches SHIPPED COMPUTE and will take previously-computing years DOWN** — loudly, recoverably,
and in the safe direction only. That is the point (§4).

## 3. The defect

### D-8 — ★★ CRITICAL, IN SHIPPED CODE. The claimed-as-dependent flags must become `Option<bool>`.

**This is not a spec defect. It is a live understatement of tax in released software**, surfaced by the
input-surface work and therefore owned here.

`can_be_claimed_as_dependent_taxpayer` is a bare `bool` with `#[serde(default)]`
(`return_inputs.rs:164`). **It silently guesses `false` — "not claimable" — the taxpayer-favourable
direction.** And it gates **two** understatement guards:

1. **§63(c)(5)** — the dependent standard-deduction FLOOR (`return_1040.rs:78`). Guessed `false` ⇒ the
   filer receives the **full** $14,600 basic standard deduction instead of `max($1,300, earned + $450)`.
2. **§1(g) / Form 8615** — the KIDDIE-TAX refusal (`return_1040.rs:618`) is keyed on the **same flag**.
   Guessed `false` ⇒ **the entire screen is disarmed** and the return files at the child's rate.

★ The irony is exact, and it is the tell. The comment guarding the kiddie block reasons with great care
about staying conservative — *"an under-count would let a real kiddie return slip through at the child's
rate (an understatement)"* — **inside an `if` gated on a flag that is silently guessed.** A meticulously
fail-closed guard, wrapped in a guess.

**Who this hits:** a student or young adult with crypto gains, claimed on a parent's return. That is close
to the *archetypal* btctax user. They get the full standard deduction **and** child-rate tax, and their
1040 prints **"Someone can claim: ☐ You as a dependent" UNCHECKED** — an affirmative false statement on a
signed return.

**Nothing backstops it.** No advisory (zero hits in `advisories.rs`). No refusal. The user is never asked.

**And it contradicts the project's own doctrine and its own idiom.** SPEC §3.4: conservative omissions
*"only ever OVERSTATE tax, never understate."* Every other unguessable question is an `Option<bool>` that
**fails loud**: `foreign_accounts`, `foreign_trust`, `mfs_spouse_itemizes`. The 1040 asks this one of
every filer.

**Fix — three parts. The first one is the one I got wrong.**

**(1) ★ THE MIGRATION. "Back-compat is free" was the OPPOSITE of free — it LAUNDERS the bug.**
The store serializes the whole struct (`cli/return_inputs.rs:44`) and there is **no `skip_serializing_if`
anywhere**, so a bare `bool` is **always written**. **Every stored row already on disk carries
`"can_be_claimed_as_dependent_taxpayer": false`** — even though the user was never asked. Migrating
naively turns that into `Some(false)` = **an answered "No"**, so the new refusal **never fires for a
single pre-existing row.** The fix would repair the *future* and **silently ratify the past — for exactly
the population that has the bug.**

⇒ ★ **The marker goes on the TABLE, not on `ReturnInputs`.** *(r4 put it on the struct — which
deserializes from **two** sources: the stored JSON **and the user's TOML**. A user's TOML never carries a
version ⇒ `serde(default)` ⇒ 0 ⇒ their explicitly-typed `false` would be mapped to `None` ⇒ refuse ⇒ **the
primary journey could never complete.** The distinguishing fact is **when the ROW was written** — a
property of storage, not of the user's document.)*

**The DDL.** ⚠️ There is **no idiom to inherit** — grep finds **zero `ALTER TABLE` in the whole repo**, and
SQLite has **no `ADD COLUMN IF NOT EXISTS`**. And `init_table` runs on **every** `get`/`set`/`exists`/… so
a bare ALTER would error `duplicate column name` on **every command after the first**. ⇒ Put the column in
the `CREATE TABLE IF NOT EXISTS` (fresh vaults), **and for old ones attempt the ALTER and tolerate EXACTLY
the duplicate-column error.**

*(Not `PRAGMA table_info` + a conditional. That is race-free here only because the vault takes a
non-blocking **exclusive file lock** at open (`btctax-store/src/lock.rs`) — an **inherited** invariant, not
an intrinsic one: relax the lock and it silently becomes a TOCTOU where both processes ALTER and one dies.
Tolerating the exact error is race-free **on its own terms**, and drops a PRAGMA from every `get`/`set`.)*

**KAT: open an old-schema vault twice.**

**★ ONE read boundary, and it must cover BOTH deserializers.** `fn row_to_inputs(json, version) ->
ReturnInputs` applies the fixup, and **every** read calls it — `get` **and `all()`**, which today does a
raw `serde_json::from_str` and would return the **un-migrated `Some(false)`**. The migration's whole
correctness rests on **no reader ever seeing the laundered flag**.

**★ ONE write boundary, and it must stamp BOTH branches.** The existing upsert is `ON CONFLICT(year) DO
UPDATE SET inputs_json=excluded.inputs_json` — **it names one column.** Stamp `schema_version = 1` in the
**DO-UPDATE branch too.** Miss it and a user who answers `false` on a pre-existing row keeps version 0 ⇒
the fixup **re-fires on the very next read** ⇒ **their answer is silently re-laundered to `None` and never
sticks**, with **no error ever shown.** The bug would reconstitute itself out of its own fix.

**KATs:** answer `false` on a version-0 row → reload → still `Some(false)`, row is version 1 ·
`all()` migrates identically to `get()` · old-schema vault opens twice.

★ **Map only `false` ⇒ `None`. NEVER `true`.** A stored `false` is indistinguishable from "never asked" —
that *is* the bug. A stored `true` is not: **nothing defaults to `true`**, so it can only have been typed.
Blanket-mapping both would refuse the one user who got it right and discard the only trustworthy value the
field can hold.

This **takes previously-computing years down — LOUDLY, RECOVERABLY, and in the SAFE direction only.**

**(2) SCOPE the spouse flag.** `None` on the **taxpayer** flag refuses unconditionally (the 1040 asks
every filer). `None` on the **spouse** flag refuses **only when the return has a spouse** (MFJ, or
`header.spouse.is_some()`); otherwise it is inert. r3 refused unconditionally — which, since D-2 ships the
flag commented, would have made **the default journey refuse for every Single/HoH filer**, naming a key
about a spouse who does not exist. The project's own precedent scopes exactly this
(`MfsSpouseItemizeUnknown` fires only on MFS); r3 copied the tri-state without copying the scoping.

**(3) FORBID the re-guess at the consumers.** `return_1040.rs:78` and `:618` must test `== Some(true)`,
**never `unwrap_or(false)`** — which is the shipped idiom at `printed.rs:936` and is *the very shape of
this defect*: a `None` silently becoming a taxpayer-favourable "No". `None` is unreachable past
`screen_inputs`; say so, and do not re-guess it.

**(4) ★ GIVE THEM A WAY TO ANSWER.** The migration's recovery story was *"one TOML line recovers"* — which
assumes the user still has the file. **The spec itself tells them to delete it** (D-3's `--force`
plaintext-hygiene path), `income show` emits masked JSON and cannot regenerate it (§8), and `set-pii`
prompts for secrets only (D-6). So a TOML-less user would face a refusing year with **no in-app path to
answer one boolean** — a wall, landing on people who did exactly what the spec told them to.

⇒ **`btctax income answer --year N`** — interactive; the only path a TOML-less user has to the fields they
can otherwise never reach. Stores through the screen-before-store gate (D-7); **refuses on a year with no
row** (only `import` creates).

★ **DECOMPOSED BY THE CRITERION** (D-2), not by "the whole class":
- **`answer` owns legs (a) + (b)** — the fail-loud tri-states, both claimed-flags, `date_of_birth`.
- **`set-pii` owns leg (c) — the SECRETS (SSNs, IP PIN) — EXCLUSIVELY, and no-echo.**

*(r6 said "the whole class in ONE pass". But the ask-the-user class and the secret class **overlap on
`ip_pin`** — which is precisely why leg (c) had to exist. So that rule would have made `income answer`
prompt an **IP PIN**, in a command the spec never required to be no-echo: an echoed crown jewel in terminal
scrollback, the exact leak Cycle 2 exists to close, and the "small, sharp" secret surface D-6 designed
would have become **two commands wide**. And the one-pass rationale never reached it: `ip_pin`'s absence
**never refuses** — no screen reads it, the packet simply prints none — so it is in the class under leg
(c), not the deadlock-driving leg (a). The rule swept in a field its own argument did not cover.)*

⚠️ **The (a)+(b) fields must still be answered in ONE pass**, because D-7 forbids storing a blob
`screen_inputs` refuses: a *partial* answer (`--taxpayer=no` while `foreign_accounts` is still `None` on a
Schedule-B year) **cannot be stored** — the strictly-improving edit is rejected by the gate, and the user
could never answer question 1 until question 2 is answered. **A deadlock.**

⚠️ **Prompt only what is LIVE for this return.** Spouse questions only when a spouse exists; Schedule-B
questions only when Schedule B files — scoped exactly as the refusals are (D-8(2)). Otherwise `answer`
asks a Single filer about a spouse who does not exist: the prompt-level twin of the refusal-level bug
D-8(2) already fixed.

⚠️ **The DOB prompt must be SKIPPABLE — empty input leaves it `None`.** D-2 puts `date_of_birth` in the
class *because* an **old** dummy GRANTS the aged add-on (an understatement). A *mandatory* prompt is a
forcing function to invent a value. `None` is the safe, advised state; the prompt must permit it.

**Only then is §9's promise true — for the users who already have the bug, not merely for new ones.**

## 4. Acceptance

- A filer who **can** be claimed as a dependent **cannot** silently receive the full standard deduction,
  and **cannot** silently escape the Form 8615 kiddie-tax screen.
- **The fix reaches the rows that ALREADY have the bug** — not merely new ones. A stored `false` that was
  never answered maps to `None` and refuses; a stored `true` (which nothing defaults to) is preserved.
- A user whose year now refuses can answer the question **without a TOML file** (`income answer`).
- No path takes a year down **silently**, or in the **understating** direction.
- `make check` green; **0 Critical / 0 Important** from the independent review.

## 5. Build order (TDD; each step red → green)

1. **The store migration.** `schema_version` column in the `CREATE TABLE IF NOT EXISTS`; for old vaults,
   attempt the `ALTER` and tolerate **exactly** the duplicate-column error. One `row_to_inputs(json,
   version)` read boundary called by **`get` AND `all`**. The write stamps `schema_version = 1` in **both
   the INSERT and the DO-UPDATE branch** — miss it and the user's answer is re-laundered on the next read.
   *KATs: open an old-schema vault twice · answer `false` on a version-0 row → reload → still `Some(false)`,
   row is version 1 · `all()` migrates identically to `get()`.*
2. **Both claimed-flags → `Option<bool>`**; version-0 rows map **`false` ⇒ `None`**, `true` preserved.
3. **The UNANSWERED refusal** — taxpayer unconditional; spouse **only when a spouse exists**.
4. **Consumers test `== Some(true)`**, never `unwrap_or(false)` — that idiom *is* the shape of this defect.
5. **`income answer`** — interactive; legs (a)+(b) only (never a secret); prompts only what is **live** for
   this return; the DOB prompt is **skippable** (empty ⇒ `None`); refuses on a year with no row (only
   `import` creates); stores through the screen-before-store gate.
6. `LIMITATIONS.md` — record that the flags are now required.
