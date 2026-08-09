# r4 review — `design/SPEC_income_scrub.md`

_Independent re-review, Opus, read-only, dispatched against `reviews/BRIEF-scrub-spec-r4.md`.
Branch `feat/income-scrub` @ `d9d7b87`. Scoped to the FOLD (`git diff 970dc98..d9d7b87`) — the status header, §2.2, §3.2, §3.3, §5.1, §7, §8. No file was modified; no build was run. Persisted VERBATIM before any fold._

---

VERDICT: needs-changes

CRITICAL: 0
IMPORTANT: 2

I checked the two paragraphs the header nominates, in the order it nominates them, and I checked them by **running the derivation they specify against the actual fixtures** rather than by reading them against the rest of the document. That is where both findings come from, and it is also the answer to the second question — see that section.

Nothing in the fold emits an original identity value, so there is no Critical. §2.2, §5.1's filter, §7 and §8's re-scheduling are closed and I could not break them.

---

    [I] §3.3 names the WRONG blind spot. The derivation is blind to a replaced field that
        is STRUCTURALLY ABSENT from the sentinel fixture, not merely value-colliding — and
        on every fixture this repo has, that blindness covers ALL FOUR of the fields the
        last three rounds were about.

WHERE: §3.3 (the `to_value` derivation, its ★ blind-spot paragraph, and the per-field `assert_ne!`
precondition); §8 step 4. Fixtures: `crates/btctax-core/src/tax/testonly.rs:240` (`kitchen_sink_household`),
`:269` (the header's `..Default::default()`), `:272-298` (the W-2 literals' `..Default::default()`),
`:468`/`:498` (`amt_owing_household`, the *only* fixture with a `b_1099` element).
Scrubber: `crates/btctax-core/src/tax/scrub.rs:164` (`ip_pin: None`), `:175-177` (`scrub_name_list`'s
trim-empty early return), `:261` (the `ein` filter).

FAILURE: The stated blind spot is *"a field whose fixture value happens to equal its stand-in shows no
diff."* That is a real hazard and a rare one. The hazard that actually bites is the one the paragraph
does not name: **a replaced field produces no differing path when the fixture never instantiates it**,
because the field is `None` on both sides, `""` on both sides, or lives inside an empty `Vec`. The spec
constrains the sentinel fixture only over **values** — *"every replaceable string holds a distinct,
recognisable value"* — which is circular (you must already know the replaced set to populate it) and
says nothing about **structure**.

Run the specified derivation against the natural sentinel base, `kitchen_sink_household`, and the
derived field axis silently omits:

| field | why no path differs | which round this field was |
|---|---|---|
| `w2s[].ein` | every W-2 literal ends `..Default::default()` (`testonly.rs:272-298`) ⇒ `ein: None` on both sides | §3.1's CRITICAL — the $1,546.80 manufactured excess-SS credit |
| `header.ip_pin` | the header ends `..Default::default()` (`testonly.rs:269`); **no fixture in the file sets `ip_pin` at all** ⇒ `None` on both sides | r3's I-4 — the row this fold just added to §5.1 |
| `foreign_country_names` | never set in any fixture ⇒ `""`, and `scrub_name_list` returns `String::new()` for trim-empty (`scrub.rs:175-177`) ⇒ `""` on both sides | r2's I-3, and §3.3's **own worked example**, still quoted six lines below the new paragraph |
| `b_1099[].payer` | `kitchen_sink_household` has no `b_1099` element; only `amt_owing_household` does (`testonly.rs:498`) | r3's I-1, vector A — the `"(unnamed broker)"` message flip |

(Verified by grep against `testonly.rs`, not inferred: the only matches for `foreign_accounts` are
`:352` and `:791`, and there are **no** matches for `ip_pin` or `foreign_country_names`.)

So the instrument written to stop "a field added to the scrubber cannot reach a release without someone
deciding its class behaviour" would, as specified, decide nothing about four fields — including the two
that r3 filed and the one that carries §3.1's Critical. And the stated guard cannot help: the
`assert_ne!` precondition is defined **"one per derived field"**, so it is *by construction* incapable of
firing on a field that never entered the derivation. §5's own rule forbids the document to claim a
guarantee no test holds; as written, this is one.

Note this is not closed by §8 step 4's *"a `b_1099` fixture … and a `foreign_accounts: Some(true)`
fixture."* That is a hand-list of two, one level down, naming neither `ip_pin` nor `ein` — the same shape
the header's own table records for r2 and r3.

FIX: constrain the sentinel fixture over **structure**, and make the constraint a compile error rather
than a discipline. Three clauses:

1. *"The sentinel fixture is **maximal**: every `Option` is `Some`, every `Vec` holds **≥ 2** elements
   (two, so an index-varying stand-in like `Payer{i+1}` and `EinMap`'s distinctness both show), and every
   nested struct is present. A field the fixture does not instantiate produces no path on either side and
   drops out of the axis silently — which is the blind spot that matters, not value collision."*
2. *"It is written as an exhaustive struct literal — no `..`, no `..Default::default()` — for all ten
   structs (§7), so a field added anywhere is a compile error in the fixture, not a smaller axis."*
3. The B1 kill-test for the derivation itself: *"a fixture variant that leaves `ip_pin: None` must make
   the axis check RED"* — otherwise the derivation ships never having been seen discriminating.

Keep the value-collision precondition; it is correct, just secondary.

---

    [I] §3.3's third verdict is authorised by "the TYPE as the reason", and the type does
        not discriminate on the `malformed` axis — `header.taxpayer.ssn` is a plain
        `String` WITH a malformed state, so the rule as written licenses exempting
        `dependents[].ssn` and `header.spouse.ssn`, on which the scrubber violates §3.2
        today.

WHERE: §3.3 (the three-verdict table, `**`no such state`, with the TYPE as the reason**`, and the ★★
paragraph enumerating which fields have which states).
`crates/btctax-core/src/tax/packet.rs:208` (`ssn: Ssn::canonical(&p.ssn)?` — taxpayer *and* spouse, via
`FiledPerson::build`), `:425` (`ssn: Ssn::canonical(&d.ssn)?` — every dependent).
`crates/btctax-core/src/tax/scrub.rs:111` (`ssn: synthetic_ssn(100 + n)`, unconditional).

FAILURE: The paragraph sorts fields into two buckets — `header.taxpayer.ssn` / `w2s[].ein` /
`header.ip_pin` (states distinguished), and *"`occupation`, `first_name`, `address_*`,
`dependents[].name`, the payer/employer fields, `business_description` and `foreign_country_names` …
plain `String`s with **no malformed state at all**"*. Two replaced fields appear in **neither** bucket:
`header.spouse.ssn` and `header.dependents[].ssn`. Both are plain `String`s, and the stated authority for
a `no such state` verdict is the **type** — so an implementer who applies the rule (rather than
back-deriving §3.2's ★ mechanism) marks their `malformed` cells `no such state`, correctly by the
document's own words, and the loop stays green with no fixture required.

The type is simply not what decides malformed-ness. `header.taxpayer.ssn` is a `String` too; what gives
it a malformed state is that `Ssn::canonical` reads a validity class off it (`packet.rs:208`). The same
call reads dependents (`packet.rs:425`) and the spouse (`:208`).

Vector — one dependent, SSN typed `"123-45-678"` (eight digits, an ordinary typo):
- Original: `ReturnHeader::build` → `Err(HeaderError::Ssn(SsnError::WrongLength(8)))`. The filer cannot
  export, so they run `income scrub` **because of that refusal**.
- Scrubbed: `scrub_dependent` writes `synthetic_ssn(101)` unconditionally (`scrub.rs:111`) → nine digits →
  `Ok`. The recipient's copy **exports where the original refused**.

That is §3.2's governing harm on the exact field §3.3's third verdict can exempt, and §3.3 test 2 is the
instrument that would catch it — but only if the matrix requires a fixture for that cell.

FIX: replace the reason clause with the discriminator that actually decides, one sentence each:
- `absent` — *decided by the type*: representable iff the field is an `Option` or lives in a `Vec`. (Here
  the type genuinely is the reason, and it is mechanically checkable.)
- `malformed` — *decided by whether any predicate in the program reads a validity class off the field*,
  which is §3.2's ★ mechanism, not the type. Today that is exactly three canonicalizers:
  `Ssn::canonical` (`packet.rs:208` taxpayer/spouse, `:425` dependents), `IpPin::canonical`
  (`packet.rs:169`), `canonical_ein` (`return_1040.rs:681`). *"A `no such state` verdict on a `malformed`
  cell must name the absence of such a reader, never the type."*

---

    [M] §5.1's key column mixes two vocabularies, so a re-derivation from §3.3 produces a
        path with no matching row.

WHERE: §5.1's table, row `excess_ss_not_creditable[].ein` (`advisories.rs:706`); §8 step 8.

FAILURE: Every other key is a `ReturnInputs` path — which is what §3.3's derivation emits. This one is an
`AbsoluteReturn` path, and `scrub_pii` does not replace it; it replaces `w2s[].ein` (`scrub.rs:262`),
which then propagates. So a maintainer re-deriving the table from §3.3 gets `w2s[].ein`, finds no row for
it, and finds a row with no derivation — in the one table whose stated purpose is to be *"checkable in
both directions."*

FIX: key the row `w2s[].ein`, and put the advisory in the `reaches` column: *"the excess-SS advisory
(`advisories.rs:706`), via `NonCreditableSs.ein`"*.

---

    [M] `header.dependents[].date_of_birth` is a replaced field that is neither in §5.1's
        table nor excused by §5.1's filter.

WHERE: §5.1's filter clause and table; §6 (*"Dependent `date_of_birth` is DROPPED (`None`)"*).

FAILURE: §6 makes the dependent DOB a replaced field, and it will appear in §3.3's derived path set
(`Some(date)` vs `None`). §5.1's filter excuses only *"`first_name`, `last_name`, `ssn`, `occupation`,
`address_*`, `dependents[].name`, `dependents[].ssn`"* — the DOB is not identity the help announces as
replaced, and a recipient would not predict it from "the identity is replaced". I verified the verdict
itself is right — `DependentRow` carries `name`/`ssn`/`relationship` only (`packet.rs:418-428`), and the
dependent advisories read `.len()` and `.name` — so the class is `neither`, harmless. But §5.1's stated
reason for keeping the `neither` rows is that *"'replaced but harmless' is a conclusion, not an
omission"*, and this one is currently an omission.

FIX: one row — `header.dependents[].date_of_birth` | *dropped per §6; no printed cell, no message —
`DependentRow` carries no DOB (`packet.rs:418-428`)* | **neither**.

---

    [M] "It is the TOML type" steers the derivation at a serializer `btctax-core` does not
        depend on, and under that serializer a field replaced with `None` vanishes from
        the diff entirely.

WHERE: §3.3 (*"`ReturnInputs` derives `Serialize` … — it is the TOML type — so the replaced set is
computable as the set of paths at which `to_value(ri)` and `to_value(scrub_pii(&ri))` differ"*).
`crates/btctax-core/Cargo.toml` — the `[dependencies]` and `[dev-dependencies]` blocks contain
`serde_json`; **`toml` appears nowhere**. TOML serialization happens one crate up
(`btctax-cli/src/cmd/tax.rs:177`, `toml::to_string_pretty`).

FAILURE: `serde_json::to_value` is in-crate and renders `None` as `Value::Null` at a present key, so a
presence change is an ordinary value difference. `toml::Value` omits the key, so `header.ip_pin` (and,
after §6, `dependents[].date_of_birth`) differ only by **presence** — invisible to a differ that walks
one side's keys. Given §5.1 now argues at length that *"a cell that goes MISSING is a divergence exactly
as much as one whose contents change,"* the diff deserves the same sentence.

FIX: name the serializer (`serde_json::to_value`, already a dependency) and add the clause: *"a path
present on one side and absent on the other is a difference."*

---

    [N] §8 step 1 still says "*(r3 is this document.)*" and step 9 still briefs the final
        reviewer on "what r1 and r2 covered".

WHERE: §8 steps 1 and 9. The status header says DRAFT r4 and lists four persisted reviews. Step 9's
under-count is the one with teeth: harness B3's whole point is telling the last pass what the earlier
rounds already held, and it now under-states that by two rounds.

---

    CLEAN — checked in the fold, and I could not break these

Recorded so a fifth round, if there is one, does not re-spend budget here.

- **The trim-emptiness rule is safe across the WHOLE reachable surface, not merely the three
  `screen_inputs` readers** (the brief's risk 4 — resolved negative). I enumerated every non-`trim`
  `is_empty()` that can see a replaced field: `form1040_full.rs:400` (the `text` helper's blank-cell
  gate), `schedule_c.rs:86` (lines A/B), `schedule_b.rs:185` (line 7b), `verify.rs:255`, and
  `scrub.rs:261` (the EIN filter). Each is safe, for a *different* reason, so the enumeration was
  necessary: `business_description` and `foreign_country_names` cannot reach a printer with a trim-empty
  value because `screen_inputs` refuses first (`:901`, `:798`); Schedule B payer cells are written
  unconditionally (`schedule_b.rs:84-90`) and rows are filtered on `amount > 0`, never on the payer, so
  `""` and `"   "` neither drop a row nor change a figure; `full_name()` trims (`packet.rs:216-220`), so
  a whitespace-only first name and an empty one produce the identical name line; and `canonical_ein`
  strips whitespace before counting digits (`return_1040.rs:681-687`), so `Some("")` and `Some("   ")`
  are both `None` to every EIN reader including `return_refuse.rs:941`. **`verify.rs:255` is a
  no-*unmapped*-field guard, not a must-be-non-empty guard** (`assert_only_filled` errors only when a
  filled field is *outside* `allowed`), so writing `""` cannot fail read-back verification. Mapping
  trim-empty → `""` opens no class flip anywhere.
- **The six structs the `..`-free guard misses hold no identity field beyond the ones already named.**
  Checked field-by-field against `return_inputs.rs`: `W2` = `owner`, `employer`, `ein`, boxes, `box12`;
  `Form1099Int`/`Div`/`G` = `payer` + boxes; `Form1099B` = `payer`, four figures, the basis gate;
  `ScheduleCInputs` = `owner`, `business_description`, `naics_code`, method, figures. No payer TIN, no
  employer address, no second free-text field. §7's extension of the guard adds coverage, not a new
  disclosure decision.
- **§2.2 is closed.** The `year - 1` disjunct now states what is true — deliberate over-refusal securing
  no live read — and refutes *both* prior mechanisms with resolvable citations rather than minting a
  third. This is the correct end state for r2 M-1 / r3 M-1.
- **§5.1's filter is sound and the IP PIN row is right**, including the `printed (PRESENCE)` class and
  the paragraph explaining why no §3 instrument can ever red on it. I re-asked r3's question of every
  remaining replaced field and found no further printed-cell or message reader — only the two table
  bookkeeping gaps above (M-1, M-2), neither of which changes what the help must say.
- **No Critical.** Every replacement the fold introduces or re-scopes emits a constant or `""`; no branch
  in it can carry an original identity byte into the emitted file.

---

DID THE FOLD CLOSE r3?

- **I-1** (emptiness keyed to `""`, not `trim()`) — **closed.** §3.2 states the class as the predicate the
  readers use, cites all three (`:672`, `:798`, `:901`) and the repo's own pin at `:1484`, and §8 step 4
  carries the identical wording. The no-leak argument is stated rather than assumed.
- **I-2** (scoping excluded `business_description`; `scrub.rs:246-250` orders the opposite) — **closed.**
  Scope is now *"every string `scrub_pii` replaces"*, the refusal-losing vector is written out with the
  `#[serde(default)]` provenance, and §7 + §8 step 6 both carry the third comment correction with its
  rule (*non-empty in, stand-in out; trim-empty in, `""` out*).
- **I-3** (derivation names no computable mechanism; state axis over-generates) — **partially.** Both
  halves were answered in the right direction and both answers are one level short: the derivation names
  an operation but the wrong blind spot (finding I-1), and the third verdict exists but is authorised by
  a property that does not discriminate (finding I-2).
- **I-4** (§5.1 omits the IP PIN; the table applies an unstated filter) — **closed.** The row is present
  and correctly classed, the filter is stated, and the ★★★ paragraph explaining the invisibility is the
  right record. Residue is two `neither`-class bookkeeping gaps (M-1, M-2), not a reopen.
- **M-1** (`year - 1` comment) — **closed**, and the stale `scrub.rs:222-223` → `:220-221` citation was
  corrected in the same edit.
- **M-2** (fixtures scheduled after the test that needs them) — **closed.** §8 step 4 pulls them in and
  states the red-across-a-gate reason. (Note finding I-1 shows the *list* of fixtures it pulls in is
  short by two.)
- **M-3** (§8 step 8's "every member") — **closed**: *"every member the table classes as printed or
  message."*
- **N** (`w2s[].employer` "reaches nothing") — **closed.** The row now names the input-form reader
  (`sections.rs:642`) and says why it is not a divergence.

---

PROSE-SHAPED OR TEST-SHAPED?

**Test-shaped. Fold these two and stop reviewing this document.**

The strongest evidence is *how* both findings were reached. Neither came from reading section against
section. Both came from taking the mechanism §3.3 specifies and executing it against the real fixtures —
grepping `testonly.rs` for `ip_pin` (zero matches), for `foreign_country_names` (zero matches), checking
which fixture holds a `b_1099` element (one, and not the kitchen sink), and reading the `..Default::default()`
that leaves every W-2's `ein` at `None`. That is an execution finding wearing a review's clothes, and it
is precisely the signal this project's rule describes: **the risk has moved out of the prose and into the
fixture.**

Three rounds have now converged in the way that says the instrument should change. r2 found a hand-list;
r3 found a hand-list inside r2's fix; r4 finds that r3's fix specifies a derivation whose completeness is
a property of a *fixture nobody has written yet*. A fifth prose round cannot answer whether that fixture
is maximal, because the fixture does not exist. A command can, in one line, the moment step 4 begins.

So the burndown is:

1. **Fold I-1, I-2 and the three Minors.** Together they are roughly six sentences in §3.3, one row and
   one key rename in §5.1, and one clause in §8 step 4. Gate the fold on the citation check, as before.
2. **Do not run a fifth whole-document round.** Confirm the fold by reading the two touched paragraphs
   against this report — that is a diff read, not a review.
3. **Make the derivation the first thing step 4 builds, and watch it go red (B1).** Land the axis
   derivation and print the derived path set *before* any matrix row exists. The one-command answer that
   settles I-1 permanently: **the printed set must contain `w2s[].ein`, `header.ip_pin`,
   `foreign_country_names` and `b_1099[].payer`.** If it does not, the sentinel fixture is not maximal and
   the derivation is blind — and it is blind on the four fields three review rounds were spent on. Pair it
   with the kill-test: a fixture variant with `ip_pin: None` must make the axis check RED. That test, once
   it exists, holds I-1 forever; no reader ever has to hold it again.
4. **I-2's residue is likewise a test, not a paragraph**: once `malformed` is authorised by "a
   canonicalizer reads this field" rather than by the type, the three call sites (`packet.rs:208`, `:425`,
   `return_1040.rs:681`) enumerate the cells, and §3.3 test 2 reds on the dependent-SSN vector without
   anyone having to notice it.

After that, the remaining risk in this spec is entirely covered by instruments §8 already schedules. Go
build it.

---

WHAT WOULD MAKE THIS REVIEW WRONG: Both findings assume an implementer follows §3.3 as written — building
the sentinel fixture from or in the shape of an existing `testonly.rs` household, and authorising a `no
such state` verdict from the field's type; someone who instead treats §3.2's ★ mechanism sentence as the
authority, authors a structurally maximal fixture from scratch, and asks of every cell "what predicate
reads this class off this field" would land both correctly with no spec change at all.
