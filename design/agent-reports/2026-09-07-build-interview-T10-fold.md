# Fold — the T10 seam review (0C / 3I / 4M / 1N), at `f61a7ebc`

Single agent, shared main tree `/scratch/code/bitcoin_tax`, branch `main`, dispatch HEAD `f61a7ebc`.
Nothing committed; nothing pushed; no subagents; no `git stash` / `restore` / `checkout --` over my own
work (every plant reverted from a `cp` backup, and where an ORIGINAL was needed for a baseline it was
materialised with `git show HEAD:<file> >` and then restored from the backup).

All three Importants folded, plus M-1, M-2, M-3 and N-1. M-4 filed as **FR-89** (secret handling —
never gating, per the owner ruling of 2026-08-27). 18 files, +917 / −110.

---

## 0. Commands, with real output

```
$ cargo fmt --all                                                    # clean

$ CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings
    Checking btctax-tui v0.18.0 (/scratch/code/bitcoin_tax/crates/btctax-tui)
    Checking xtask v0.18.0 (/scratch/code/bitcoin_tax/crates/xtask)
    Checking btctax-tui-edit v0.18.0 (/scratch/code/bitcoin_tax/crates/btctax-tui-edit)
    Finished `dev` profile [optimized + debuginfo] target(s) in 3.04s

$ cargo run -p xtask -- line-coverage
line-coverage OK: 375 money lines across 18 form(s) [f1040:45 f1040s1:12 f1040s1a:50 f1040s2:9
f1040s3:5 f1040sa:21 f1040sb:5 f1040sc:7 f1040sd:31 f1040sse:22 f6251:41 f8889:27 f8949:12 f8959:17
f8960:15 f8995:16 f8995a:39 i1040gi:1], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0),
17 not line-bound (ratchet 17)

$ cargo run -p xtask -- census-join
census join: 274 unmodeled entries across 13 maps, every one placed by a direction block asserted
against the form's extract, graded by one of DIRECTION_OF_CAPTION's 22 readings (each cited to a
sentence the form prints) and covered by an existing variant

$ cargo run -p xtask -- stop-list
R15 stop list: 8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned;
no forbidden shape

$ cargo run -p xtask -- prompt-check
xtask prompt-check: OK — 88 assertions, all verbatim

$ cargo run -p xtask -- box-census
box-census OK: 268 printed boxes across 19 archived editions of 9 information returns, every one
decided (268 entries)

$ make check
     Summary [  20.702s] 3496 tests run: 3496 passed, 12 skipped
```

**All five instruments are byte-identical to their HEAD values** (375 / 18 / 31 / 0 / 17; 274 across
13; 8 + 4, 91; 88; 268 / 19 / 9). `make check` 3491 → **3496**, exactly the five new tests in §7.

### ★ A measurement hazard found and worked around — read this before trusting a green in this tree

`cargo`/`nextest` in this working tree **silently reused stale artifacts** several times: a source
edit compiled nothing and the suite reported PASS in 0.004 s against the *previous* build. It first
showed up as an apparent pre-existing red at HEAD —
`cmd::tax::tests::the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named` failing
on an untouched tree — which turned out to be `btctax-core`'s rlib still carrying the T10 builder's
own P17 plant (`#[serde(default)]` on `DirectDeposit::routing`). `touch crates/btctax-core/src/tax/return_inputs.rs`
made it green immediately.

That matters here twice over: (a) the `cp`-backup revert pattern this workflow mandates can leave a
**planted defect compiled into `target/`**, so a later "green" may be measuring the plant; and (b) a
kill can appear NOT to red for the same reason, which would be read as a blind checker. Every
measurement in this report was taken after an explicit `touch` of the edited file, and the closing
`make check` and clippy runs were preceded by `find crates -name '*.rs' -exec touch {} +` so nothing
in them is cached. Worth a follow-up if it recurs; not filed, because a `touch` before a gate is a
one-word habit and I could not reproduce it deliberately.

---

## I-1 — the opener carries the phone and the foreign address and says it carries neither

### What changed

**The two phrases the brief names, and three more the derived fixture then found.**
`CARRIED_IDENTITY` (`crates/btctax-cli/src/open_next_year.rs:288-`) gains
`("your phone number", &["header.phone"])` and `("your foreign address, if you have one",
&["header.foreign_"])`. Switching the guard's fixture to a derived one immediately reported four
*more* leaves `seed` carries that no phrase claimed — none of them mine:

| leaf | carried since | now named by |
|---|---|---|
| `header.dependents[…]` | **T7** | a new phrase, *"each dependent, by name, SSN and relationship, with every §152 gate blank"* |
| `form_1098[…]` | **T9** | added to the existing *"each employer and payer"* prefix list |
| `sa_1099[…]`, `sa_5498[…]` | **T16** | the same |

So the same defect was three tasks deep, not one — and every one of them was invisible for the same
reason the phone and the foreign address were.

**A prefix that reached past its own struct.** `"header.spouse"` matched `header.spouse_ip_pin`, so a
future `seed` that carried the spouse's IP PIN would have been silently absorbed by the *"your name
and SSN"* phrase and announced as part of somebody's name. Both prefixes are now `"header.taxpayer."`
and `"header.spouse."`. **This is not cosmetic — it is what made kill K-2 below possible at all**; the
first attempt at that kill passed, and the reason was this collision.

**The fourth surface: the shipped `--help` and man page.** `cli.rs`'s long-help doc comment — the one
source `--help`, `docs/man/btctax-income-open-next-year.1` and the TUI offer all print — carries its
own prose copy of the carried list, ending *"and nothing else"*. It had gone stale **twice**: T7 added
dependents without updating it (recorded as an Important in the T7 review) and T10 added the phone and
the foreign address without updating it. It is corrected, the man page regenerated
(`cargo run -p xtask -- docs`), and it is now **checked**: `CARRIED_IDENTITY` grew a fourth column
naming the substring the shipped help must contain, and
`cli::tests::the_shipped_help_names_everything_the_opener_carries` fails the build when a row is added
without telling the filer. A row cannot be added to the table without deciding what the user is told.

### The half that mattered — the fixture is DERIVED, and it is checked

`every_leaf_the_seed_carries_is_named_in_the_report`
(`crates/btctax-cli/tests/open_next_year_t4b.rs`) no longer writes its own year N.

1. **The fixture is `scrub_axis::maximal_sentinel()`, re-yeared to `FROM`.** That is the repo's
   every-`Option`-`Some`, two-rows-of-every-`Vec`, every-leaf-non-default `ReturnInputs`, written as an
   **exhaustive struct literal with no `..Default::default()` anywhere** — so a leaf added to
   `ReturnInputs` tomorrow is an `E0063` *there* and is populated *here* the day it is added. A leaf
   `seed` starts carrying can no longer be invisible to this walk.
2. **Two fixtures, because one cannot reach everything.** The maximal sentinel's Form 8960 line 9b
   makes its year-N return refuse (`Nii9bExceedsDeductedSalt`), so its carryforward chain is
   `not_carried` and the carryforward phrases would go unexercised. `a_year_with_money_in_it` computes
   and covers them. The guard now loops over both, naming which one it is asserting about.
3. **A floor makes the pair a measurement rather than a hope.** After the loop, `open_next_year::seed`
   is called **directly** on the maximal prior — no vault, no carryforward chain, so it cannot be
   defeated by a year-N return that refuses — and every leaf it writes must be one the loop actually
   reached. Trim either fixture and this reds *now*, instead of the next omission reding in three
   tasks' time. This is the assertion the three green-but-blind guards did not have.

### The kills and their observed reds

**K-1 — the instance.** The `"your foreign address, if you have one"` row deleted from
`CARRIED_IDENTITY`:

```
$ cargo nextest run --locked -p btctax-cli -E 'test(every_leaf_the_seed_carries_is_named_in_the_report)' --no-capture
thread 'every_leaf_the_seed_carries_is_named_in_the_report' panicked at
crates/btctax-cli/tests/open_next_year_t4b.rs:1491:13:
the MAXIMAL sentinel (every leaf non-default, compiler-enforced): `header.foreign_country` is covered
by "foreign address", which the report does not print: ["the filing status (…)", "your name and SSN",
"your mailing address", "your phone number", "each employer and payer, by name and EIN/TIN, with every
box blank", "each dependent, by name, SSN and relationship, with every §152 gate blank", "and which
year this one was opened from"]
```

**K-2 — the CLASS, and the whole point of the fold.** A *new* carry planted in `seed`
(`spouse_ip_pin: prior.header.spouse_ip_pin.clone()`), which no phrase names. With the derived
fixture:

```
thread 'every_leaf_the_seed_carries_is_named_in_the_report' panicked at
crates/btctax-cli/tests/open_next_year_t4b.rs:1487:17:
the MAXIMAL sentinel (every leaf non-default, compiler-enforced): the seed writes
`header.spouse_ip_pin`, and no phrase in the opener's report names it — a filer reading "everything
else is blank" would have no reason to look. Report:
Opened TY2025 from TY2024.
  Carried from TY2024 — CONFIRM each: the filing status (…), your name and SSN, your mailing address,
  your phone number, your foreign address, if you have one, each employer and payer …
```

and then, **with the identical plant still in `seed`**, the guard's fixture reverted to the pre-fold
hand-written one and the floor neutralised:

```
$ cargo nextest run --locked -p btctax-cli -E 'test(every_leaf_the_seed_carries_is_named_in_the_report)'
        PASS [   0.291s] (1/1) btctax-cli::open_next_year_t4b every_leaf_the_seed_carries_is_named_in_the_report
     Summary [   0.292s] 1 test run: 1 passed, 814 skipped
```

Same defect, same checker: **green with the hand-written fixture, red with the derived one.** That is
the FR-88 pattern demonstrated on demand rather than argued.

**K-3 — the floor.** The maximal fixture dropped from the loop, the floor left live:

```
thread 'every_leaf_the_seed_carries_is_named_in_the_report' panicked at
crates/btctax-cli/tests/open_next_year_t4b.rs:1523:5:
`seed` writes these leaves and NO fixture above reached them, so the walk could never have asked
whether the report names them — extend the fixture, do not narrow the assertion:
["header.spouse.first_name", "header.spouse.last_name", "header.spouse.ssn", …
 "header.dependents[0].name", … "header.phone", "header.foreign_country", … "sa_5498[1].box6_account_type"]
```

(The message now prints the count plus the first ten; the raw list was 170 leaves long.)

**K-4 — the shipped help.** The `cli.rs` long-help doc comment reverted to HEAD's wording:

```
thread 'cli::tests::the_shipped_help_names_everything_the_opener_carries' panicked at
crates/btctax-cli/src/cli.rs:1353:13:
`btctax income open-next-year --help` must say "your phone number" — the opener carries "your phone
number", and this text is what tells the filer to go and confirm it. Add it to the doc comment and
re-run `cargo run -p xtask -- docs` so the man page follows:
Open NEXT year from this one: …
WHAT COMES WITH THEM, and nothing else: your filing status, your name and SSN (and your spouse's),
your mailing address, each dependent's name, SSN and relationship, each employer and payer by name and
EIN/TIN with every box blank, and last year's computed carryforwards. …
```

### Decided differently

Nothing was narrowed. Three things were **widened**, and each was forced by the derived fixture rather
than chosen: the three extra `CARRIED_IDENTITY` phrases/prefixes (T7/T9/T16 carries), the
prefix-tightening to `header.taxpayer.` / `header.spouse.`, and the `--help` + man-page surface with
its own guard. Widening stopped there: FR-88's *doctrine* question (amending `HARNESS.md` B1) is
untouched, and FR-88 gains a dated note recording what the derived fixture measured.

---

## I-2 — line 35c printed an account type the filer never chose

### What changed

| where | change |
|---|---|
| `btctax-core/src/tax/return_inputs.rs` | `DirectDeposit::kind: Option<DepositAccountKind>` — `None` is *"not chosen yet"* |
| `btctax-input-form/src/spec/sections.rs` | `create` leaves it `None`; `get` is `and_then` so an unchosen type reads as **unanswered**; `set` writes `Some(_)`; the seam comment's false first half deleted |
| `btctax-core/src/tax/return_refuse.rs` | `DirectDepositCell::Kind` (line 35c) + its `Display`; a Kind leg in `screen_direct_deposit`, screened in the form's own order 35b → 35c → 35d, with its own message (nothing is *malformed*; the box is simply not chosen) |
| `btctax-core/src/tax/packet.rs` | `HeaderError::DepositTypeUnchosen`; `ReturnHeader::build` spends the `Option` there. `PrintedDirectDeposit::kind` stays **non-`Option`**, so the emitter's exhaustive `match` has no third arm to invent |
| `btctax-input-form/src/attribute.rs` | `C::Kind => FieldId::DdKind` — the compiler demanded it, which is the point |

The deleted comment is the review's I-2 in one line: *"`Checking` … is the starting position of a
two-way control the filer must confirm"* asserted a confirmation step that did not exist.

### The kill and its observed red

`spec::sections::tests::the_account_type_is_unanswered_until_the_filer_chooses_it_and_refuses_until_then`
drives the **real `create` and the real `set`s** — not a hand-built `DirectDeposit` — because the
defect lived in `create` and a fixture that constructs the struct itself cannot see it. It then checks
both boundaries and both directions: unchosen refuses at `screen_param_free` *and* at
`ReturnHeader::build`; answered with **Savings** (a value the old default could never produce) prints
Savings.

Plant — the `Checking` default restored in `create`:

```
thread '…::the_account_type_is_unanswered_until_the_filer_chooses_it_and_refuses_until_then' panicked
at crates/btctax-input-form/src/spec/sections.rs:1775:9:
assertion `left == right` failed: a type nobody chose must read as unanswered, or the editor shows the
filer an answer they did not give
  left: Some(Choice("Checking"))
 right: None
```

Plant — `screen_direct_deposit`'s Kind leg made inert (`if false`):

```
thread '…::the_account_type_is_unanswered_until_the_filer_chooses_it_and_refuses_until_then' panicked
at crates/btctax-input-form/src/spec/sections.rs:1783:46:
an unchosen account type must refuse
```

The same plant also reds the wire test (below), so the leg is held from two crates:

```
thread 'cmd::tax::tests::the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named'
panicked at crates/btctax-cli/src/cmd/tax.rs:1705:14:
an imported block with no account type must refuse
```

### Decided differently — and a §4.3 consequence the fix creates, measured rather than assumed

**The `Option` weakens §4.3 on the wire for this one key, and the source now says so.** I wrote the
obvious note — *"still no `#[serde(default)]`, so an absent key is a mistyped row"* — and then tested
it. It is false: **serde's derive reads a missing `Option` field as `None` whether or not
`#[serde(default)]` is written**, so `[header.direct_deposit]` with `routing` and `account` but no
`kind` now **parses**, where `routing`/`account` still refuse. Measured:

```
direct_deposit: Some(DirectDeposit { routing: "123456780", kind: None, account: "A1" })
```

That is safe, but not for the reason a comment would have claimed: it fails closed at the **screen**,
not at serde — the block arrives unchosen and `screen_direct_deposit` refuses it naming line 35c. Both
halves (the parse AND the refusal) are now asserted together in
`the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named`, and the field's doc says
plainly which key is the exception and what closes it. Leaving the first draft in would have been I-3
committed inside the fix for I-2.

**`DirectDepositCell::Kind` rather than a new `RefuseReason`.** The reviewer's prescription, kept: it
is the same refusal about the same block with the same two remedies, and the input form dispatches on
the cell. The cost is that the variant is named `DirectDepositNumberMalformed` and a type is not a
number; the *message* carries the truth, and the enum's doc says so in as many words rather than
leaving the reader to notice.

---

## I-3 — the ABA rule's written justification was false in all three claims

### What changed

`RoutingNumber::canonical`'s ★★ paragraph (`packet.rs`) is replaced. The new text leads with **"THIS
RULE BLOCKS A RETURN"**, states what actually happens (the number *is* stored; nothing files, because
the screen is a value rule outside the answered-ness tier and `resolve.rs` fails closed, with
`ReturnHeader::build` refusing again; `RefundByPaperCheck` never fires, because its guard is
`direct_deposit.is_none()`), and then makes the case for keeping the rule **on those terms**: the
refusal is loud, cell-anchored and self-clearing, and against that a false refusal of a genuine
routing number is very close to impossible.

It closes with the inference the false version licensed, denied explicitly: *"a btctax-only validity
rule is cheap"* is false as stated — every one of them blocks a return until the filer acts, and adding
one to another cell has to be argued on its own near-zero false-refusal rate, its own remedy and its
own unrecoverable harm.

The build report's §3 D-3 carries the same correction, with the false sentence **retained struck
through** rather than deleted, because the argument it made is the thing a future maintainer must not
reason from. Its §1 table row for `DirectDeposit` is annotated with the I-2 change. The T10 commit
message (`f8768e93`) repeats the false claim and cannot be rewritten; the review ledger and this
correction are its record.

### The kill and its observed red

A prose fix needs a behaviour witness or it is just different prose.
`advisories::tests::a_malformed_routing_number_blocks_the_return_and_the_paper_check_notice_stays_silent`
asserts the corrected paragraph's claims **in its own order**: stored → refused at the screen → refused
at the print boundary → the advisory stays SILENT while the block is present → deleting the block makes
the return file and the advisory speak.

Plant — the advisory's guard made unconditional (i.e. the old doc's *"`RefundByPaperCheck` then tells
the filer"* made true):

```
thread '…::a_malformed_routing_number_blocks_the_return_and_the_paper_check_notice_stays_silent'
panicked at crates/btctax-core/src/tax/advisories.rs:3133:9:
RefundByPaperCheck is guarded on the block being ABSENT — a malformed block silences it, so the filer
is NOT told a check is coming
```

Plant — the ABA check-digit branch made inert (i.e. the old doc's *"Nothing is blocked"* made true):

```
thread '…' panicked at crates/btctax-core/src/tax/advisories.rs:3116:9:
assertion `left == right` failed: a bad check digit must BLOCK the return — this rule is not a warning
  left: None
 right: Some(DirectDepositNumberMalformed { cell: Routing, why: BadCheckDigit })
```

---

## M-1 — a year whose map has no `[direct_deposit]` dropped the filer's numbers in silence

### What changed

`form1040_full.rs`'s `if let Some(cells) = map.direct_deposit.as_ref()` becomes an exhaustive `match`
on the pair. `(None, Some(_))` — a map with no block and a return that carries one — is now a
`FormsError::Geometry` naming the missing map section, what would have been dropped, and why nothing
else would have told the filer. `(None, None)` is the ordinary state of a year that has not mapped
35b–35d and still fills.

**A refusal, not a hand-mark or an advisory,** because the same function already refuses this way for a
missing money cell eleven lines down (`need(&map.line1a, …)?`) — and because the compensating advisory
is exactly the thing that does *not* work here: it reads `ReturnInputs`, not the map.

This is the condition the reviewer put on adjudication 1 (TY2025 correctly getting no T10 cells). It is
now closed, so the verdict stands unconditionally, and FR-84 no longer walks into a silent drop.

### The kill and its observed red

`full_return_forms::a_map_with_no_direct_deposit_block_refuses_a_return_that_has_one`, both directions.
Plant — the refusal arm made unreachable, i.e. the silent skip restored:

```
thread 'a_map_with_no_direct_deposit_block_refuses_a_return_that_has_one' panicked at
crates/btctax-forms/tests/full_return_forms.rs:4987:9:
dropping the filer's bank details must not be silent — the fill returned Ok on a map with no
[direct_deposit] and a return that carries one
```

★ The first version of that assertion used `expect_err`, whose `Ok` payload is the whole filled PDF —
the failure message was **1.2 MB of byte literals**. Destructured with `let Err(err) = … else` instead:
a kill whose red nobody can read is barely a kill.

### An incidental the line-coverage gate caught

The refusal's first draft named `Advisory::RefundByPaperCheck` in a **string literal** inside
`btctax-forms/src`. `xtask line-coverage`'s (4b) rule derives *"is this type printed?"* from whether the
emitter crate names the identifier in non-comment code, so that one word pulled the whole `Advisory`
enum into scope and the gate failed:

```
xtask line-coverage: line-coverage FAILED (1 problem(s)):
  - crates/btctax-core/src/tax/advisories.rs: type `Advisory` declares a Usd field but has no
    `cover_advisory()` — a money-bearing printed type that nothing in the table mentions is exactly
    the gap nested money was
```

Reworded to *"the paper-check advisory (`RefundByPaperCheck`)"* — same meaning, no bare identifier —
and the instrument returned to its HEAD value. Recorded because it is the gate working: an incidental
edit changed a derived scope, and the derivation noticed.

---

## M-2 — `Advisory::RefundByPaperCheck`'s doc comment was stale

Rewritten to say what the variant does now (fires when the return is due a refund and the filer gave no
instruction; the block is collected and printed when they do), plus the guard's consequence that I-3
turns on.

**And it is checked, because a doc comment is the one copy no assertion could reach** — which is
precisely why it survived T10 while the `message()`, the fire condition and `LIMITATIONS.md` were all
updated three lines away. `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` now
scans this module's own source (`include_str!`) for the retracted sentence, with the needle assembled
from two literals so the file holds no literal instance of the phrase it forbids.

**★ The instrument caught its own author first.** The check went red on its very first run — on *my*
new doc comment, which quoted the retracted sentence verbatim while explaining that it was retracted —
and on the pre-existing `!m.contains("v1 never fills")` assertion. Both are now routed through the one
assembled needle. That is B1's *"seen red once"* arriving unplanned, and it is the reason the check is
worth having: the wording had to be paraphrased rather than quoted.

Plant — the retracted sentence restored to the variant's doc comment:

```
thread '…::the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given' panicked at
crates/btctax-core/src/tax/advisories.rs:3070:9:
`advisories.rs` still claims "v1 never fills" somewhere — T10 fills the direct-deposit block, and the
sentence is retracted in the message, the fire condition and LIMITATIONS.md. A doc comment saying
otherwise is the copy that outlives them.
```

---

## M-3 — clearing the foreign country left the province and postal code at rest

`AddrForeignCountry`'s `set` clears `foreign_province` and `foreign_postal_code` when the new value is
empty — **the block cleared as a unit**, mirroring the Spouse section's `delete` for `spouse_ip_pin`. A
country *change* (Elbonia → Ruritania) keeps them: that is one edit of one address, and clearing there
would throw away what the filer typed.

Kill: `clearing_the_foreign_country_clears_the_row_so_a_later_country_cannot_resurrect_it`, asserting
the change-keeps direction as well so it cannot be satisfied by clearing always. Plant — the clear
removed:

```
thread '…::clearing_the_foreign_country_clears_the_row_so_a_later_country_cannot_resurrect_it'
panicked at crates/btctax-input-form/src/spec/sections.rs:1765:9:
assertion `left == right` failed: clearing the country must clear the row, not leave it dead but at rest
  left: ("Mud Province", "XY1 2AB")
 right: ("", "")
```

---

## N-1 — *"nothing on the return reads it"*, said of a field that prints on the return

`AddrPhone`'s help now reads *"…a blank is lawful, and no figure on the return is computed from it; it
prints in the signature block."* The same sentence in `scrub.rs`'s phone note is corrected the same
way. No test: it is a wording fix, and the phone's printing is already held by
`the_phone_number_prints_in_the_signature_block_and_nowhere_else`.

---

## M-4 — secret handling: filed, never gating

**FR-89** in `FOLLOWUPS.md`, owning phase *ownerless residue*, with its reproduction (`btctax tui-edit`
→ `T` → Direct deposit → `c`; the two cells echo and redisplay unmasked, unlike the two IP PINs two
sections away), the reason the call is deliberate (a filer must read a routing number back against
their cheque, and a `Secret` field's `get` returns presence only), and each existing bound named with
the test that holds it (`scrub_routing`/`scrub_account` in §3.3's derived matrix; `income show`'s
`mask_pii` with a plant; masked `Debug`; never carried across a year). What is unbounded is only the
editor's on-screen echo and whatever the terminal scrollback keeps.

Per the owner ruling of 2026-08-27 this is neither Critical nor Important and held nothing.

---

## Deviations

1. **I-1 was folded WIDER than the brief's two phrases, and every widening was forced by the derived
   fixture rather than chosen.** Switching the guard to `maximal_sentinel` immediately reported
   `header.dependents` (carried since T7), `form_1098` (T9), `sa_1099` and `sa_5498` (T16) as carried
   and unnamed — the same defect, three tasks older. Naming them was not optional once the guard could
   see them: leaving them would have meant deleting or narrowing a true failure.

2. **The `header.spouse` → `header.spouse.` prefix tightening was not in the brief and is load-bearing.**
   Kill K-2 (a new carry planted in `seed`) initially PASSED because `header.spouse_ip_pin` starts with
   `header.spouse` and was absorbed by the *"your name and SSN"* phrase. A prefix that reaches past its
   own struct is the same blindness as a fixture that never populates a leaf, one level down.

3. **The `--help` / man-page surface was folded, and given its own guard.** The brief scoped I-1 to
   `CARRIED_IDENTITY` and the test's `NAMED`. But the review's own framing is *"four surfaces"*, the
   shipped help is one of them, and it had gone stale twice for the same reason (T7's review filed the
   identical finding). `CARRIED_IDENTITY` grew a fourth column — the substring the help must contain —
   so the second copy is now checked rather than remembered.

4. **The §4.3 wire rule for `kind` is genuinely weakened by the `Option`, and the source says so.**
   Serde reads a missing `Option` field as `None` regardless of `#[serde(default)]`, so an imported
   `[header.direct_deposit]` with no `kind` key parses (unchosen) rather than refusing, where the two
   strings still refuse. The screen closes it; serde does not. Recorded in the field's doc and asserted
   in both halves rather than papered over. **This is the only behavioural loosening in the fold.**

5. **`DirectDepositCell::Kind` rather than a new `RefuseReason` variant** — the reviewer's prescription,
   kept deliberately. The variant name (`DirectDepositNumberMalformed`) is imprecise for a checkbox;
   the message carries the truth and the enum doc records the imprecision rather than hiding it.

6. **M-2 got a source-scan instrument that the brief did not ask for.** The review's own diagnosis was
   that the doc comment is *the copy nothing asserted on*; fixing the words without fixing that would
   have left the next stale comment to the next reviewer. It cost six lines, and it went red on its
   first run.

7. **Nothing was done about the `foreign_address_is_live()` reader itself.** I-1's printed-page harm
   runs through it, but the reviewer's verdict (seam 5) is that a single `Option` at the print boundary
   is the right shape and that the fix is the announcement, not the liveness rule. M-3 closes the
   at-rest half. No change proposed.

8. **FR-88 was annotated, not actioned.** The `HARNESS.md` B1 amendment remains the owner's call. What
   the fold added is the dated measurement that argument now rests on — four further omissions surfaced
   the moment the fixture became derived.

---

## The five new tests

| test | crate | holds |
|---|---|---|
| `cli::tests::the_shipped_help_names_everything_the_opener_carries` | btctax-cli | I-1's fourth surface: every `CARRIED_IDENTITY` row's token appears in the shipped `--help` (and thus the man page) |
| `spec::sections::tests::the_account_type_is_unanswered_until_the_filer_chooses_it_and_refuses_until_then` | btctax-input-form | I-2, through the real `create` + `set`s, both boundaries, both directions |
| `spec::sections::tests::clearing_the_foreign_country_clears_the_row_so_a_later_country_cannot_resurrect_it` | btctax-input-form | M-3, plus the change-keeps direction |
| `tax::advisories::tests::a_malformed_routing_number_blocks_the_return_and_the_paper_check_notice_stays_silent` | btctax-core | I-3: the corrected paragraph's claims, in its own order |
| `full_return_forms::a_map_with_no_direct_deposit_block_refuses_a_return_that_has_one` | btctax-forms | M-1, both directions |

Two existing tests were extended rather than duplicated:
`every_leaf_the_seed_carries_is_named_in_the_report` (the derived fixture, the two-fixture loop and the
floor), `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` (M-2's source scan) and
`the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named` (the `kind`-key wire
behaviour and its refusal).

---

## Closing gate

```
$ make check
     Summary [  20.702s] 3496 tests run: 3496 passed, 12 skipped
```

`cargo fmt --all` clean; `CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets
--all-features -- -D warnings` clean (forced re-check, not cached). All five instruments unchanged from
HEAD. 18 files modified, no untracked files, no plant residue (`git diff` carries no `if false`, no
`unreachable_code`, no probe). Nothing committed, nothing pushed.
