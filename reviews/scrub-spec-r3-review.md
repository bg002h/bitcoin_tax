# r3 review — `design/SPEC_income_scrub.md`

_Independent re-review, Opus, read-only, dispatched against `reviews/BRIEF-scrub-spec-r3.md`.
Branch `feat/income-scrub` @ `4b49c8c`. Scoped to the FOLD (`git diff c098169..4b49c8c`) — §2.2, §3.2, §3.3, §4.3, §5.1, §7, §8. Persisted VERBATIM before any fold._

---

VERDICT: needs-changes

CRITICAL: 0
IMPORTANT: 4

I looked hardest at **§3.2's new emptiness rule and §3.3's derived field axis** — the two places where the fold answered "you used a hand-list" with "derive it", which is the answer that is easy to write and hard to build. I could not break §3.2's `NotDigits` coarsening or §3.3 test 2's asymmetric comparison; that pair is correct and I record why below so the next round does not re-spend budget on it. Nothing I found leaks an original identity value, so there is no Critical. All four blocking findings are the same shape as r2's: **the fold fixed the instance the reviewer named and left the class one field over, or stated a derivation without naming anything an implementer can compute it from.**

---

    [I] §3.2's emptiness rule is keyed to `""`, but all three readers of an emptiness
        predicate key on `.trim().is_empty()` — and the repo already has a committed
        test whose comment warns about exactly this substitution.

WHERE: §3.2 ("**Every replacement preserves emptiness: `"" → ""`**"); §8 step 4 ("`"" → ""` emptiness on every payer/employer loop"). Readers: `crates/btctax-core/src/tax/return_refuse.rs:672` (`b.payer.trim().is_empty()`), `:798` (`foreign_country_names.trim().is_empty()`), `:901` (`c.business_description.trim().is_empty()`). The repo's own pin: `crates/btctax-core/src/tax/return_refuse.rs:1484` — *"Whitespace is not a name. This pins the `trim()`, which a naive `is_empty()` would miss."*

FAILURE: The equivalence class every screen induces is `trim().is_empty()`, not `is_empty()`. A rule written as `"" → ""` leaves whitespace-only values in the replaced set, so the class flips exactly as it does today:

- Vector A — `b_1099[0].payer = "   "` with totals and no basis confirmation. Original refusal reads *"the Form 1099-B from (unnamed broker)…"*; the scrubbed copy reads *"…from Broker1…"*. This is r2's I-3 leg 1, still live after the fold that was written to close it, because `"   " != ""`.
- Vector B — `schedule_c.business_description = "   "`. `screen_inputs` refuses `ScheduleCNoBusinessDescription` on the original (`return_refuse.rs:901`, and `:1487-1494` pins the three-space case); `scrub_pii` replaces it with `"Example business"`, and the scrubbed copy does not refuse at all.

Note the direction of the fix: the class to preserve is *trim-emptiness*, not the bytes — emitting `""` for a whitespace-only original preserves the class and leaks nothing, so this correction does not open a disclosure channel.

FIX: state the class as the predicate the readers use — "every replacement preserves **trim-emptiness**: a value with `trim().is_empty()` is replaced by `""`, never by a stand-in" — and carry the same wording into §8 step 4.

---

    [I] The emptiness rule is scoped to "every payer/employer loop", which excludes
        `business_description` — the one replaced field where the class flip removes a
        REFUSAL rather than changing a message — and `scrub.rs` carries a comment
        ordering the opposite that the spec does not correct.

WHERE: §3.2 ("It applies to **every payer/employer loop** in `scrub_pii`, not only the broker one"); §8 step 4 (same scoping). `crates/btctax-core/src/tax/scrub.rs:245-255` (the `schedule_c` block) and `:251` (`sc_out.business_description = "Example business".into();`, unconditional); `crates/btctax-core/src/tax/return_refuse.rs:897-905`; `crates/btctax-core/src/tax/return_inputs.rs:326-327` (`#[serde(default)] pub business_description: String`).

FAILURE: An empty `business_description` is not a hypothetical — it is what an imported TOML that omits the key produces, and `return_refuse.rs:1467-1480` is a committed test asserting that state refuses. So: a filer whose Schedule C description is blank hits `ScheduleCNoBusinessDescription`, runs `income scrub` **because of that refusal**, and hands over a file on which the refusal does not exist. That is §3.2's own governing harm — *"the scrubbed copy files where the original refused — defeating the purpose, since the filer is sending the file to reproduce a refusal"* — reached through the field §3.2's new rule declines to cover.

§3.2's ★ mechanism sentence ("every field `scrub_pii` replaces must preserve every property any screen … reads from it") does cover it. The *rule the fold added* does not, and two things push an implementer the wrong way at the moment the matrix reds:

1. §8 step 4 restates the payer/employer scoping, so `business_description` reads as deliberately excluded.
2. `scrub.rs:246-250` says *"It must stay NON-EMPTY: an empty description refuses the return (`ScheduleCNoBusinessDescription`), so scrubbing it to `""` would make the scrubbed copy refuse where the original filed"*. That reasoning is sound only for a **non-empty** original; as written it is an unconditional order in the opposite direction, and §7 corrects `scrub.rs:14` and the middle-group-`00` claim but not this one. A red matrix cell will be argued away against that comment rather than fixed.

FIX: scope the rule to **every string `scrub_pii` replaces**, not the payer/employer loops; and add to §7 / §8 step 6 the third `scrub.rs` comment correction — `business_description` stays non-empty *only when the original was*.

---

    [I] §3.3's "field axis DERIVED from `scrub_pii`'s replaced set" names no mechanism a
        test can compute it from, and the state axis over-generates cells that cannot
        exist — so the buildable version of both is a hand-list, which is r2's I-3 one
        level down, green on the day it is written.

WHERE: §3.3 ★★★ ("The loop enumerates `{every field scrub_pii replaces} × {absent, empty, malformed, valid}` and **fails on any member with no row** — so a field added to the scrubber cannot reach a release without someone deciding its class behaviour"); §8 step 4.

FAILURE: Two legs, both of which end in the same place.

**(i) The field axis has no stated source.** Rust offers no reflection over "which fields did this function change", so an implementer writes `const REPLACED: &[&str] = &[…]`. That list is correct today and silently wrong the first time someone scrubs a new field — at which point the spec's claim ("a field added to the scrubber cannot reach a release without someone deciding its class behaviour") is a guarantee no test holds, which §5's own rule forbids the document to make. The `..`-free destructure guard does **not** close this: it forces a decision when a field is added to `ReturnInputs`/`Person`/`Dependent`/`HouseholdHeader`, not when an existing field's classification changes from kept to replaced.

A mechanism does exist, and the spec should name it rather than leave it to be reinvented: `ReturnInputs` is `Serialize`/`Deserialize` (`crates/btctax-core/src/tax/return_inputs.rs:626` and the derives above it — it is the TOML type), so the replaced set is computable as *the set of paths at which `to_value(ri)` and `to_value(scrub_pii(&ri))` differ*, over a fixture whose every replaceable string is a distinct sentinel. Its blind spot is already modelled in this spec: a field whose fixture value happens to equal the stand-in shows no diff, which is precisely what §3.4's `assert_ne!(original.address_city, SCRUB_CITY)` precondition exists to stop — that assertion has to become general, one per derived field, not one field.

**(ii) The state axis means a different thing per field, and there is no verdict for "this field has no such state."** Checked against the actual types:

| field | `absent` | `empty` | `malformed` |
|---|---|---|---|
| `header.taxpayer.ssn` (`String`, `return_inputs.rs:197`) | **≡ `empty`** — both are `""` → `SsnError::Missing` | ≡ absent | yes |
| `w2s[].ein` (`Option<String>`, `:~289`) | `None` | `Some("")` | yes |
| `header.ip_pin` (`Option<String>`, `:308`) | `None` | `Some("")` | yes |
| `first_name`, `last_name`, `occupation`, `address_street/city/state/zip`, `dependents[].name`, `w2s[].employer`, `*_1099[].payer`, `business_description`, `foreign_country_names` (all `String`) | **no such state** | yes | **no such state** |

So over the cross product the loop "fails on any member with no row" is unsatisfiable — there is no fixture with a malformed `occupation` — and the implementer's only escape is an exception hand-list, which reintroduces exactly what the fold removed. This project's own conformance rule already names the missing third verdict: every member must be *accounted for* — mapped to a fixture **or explicitly recorded as carrying no such state, with a reason** — because a checker that cannot tell *"this cell encodes no decision"* from *"we forgot this cell"* is not a conformance check.

FIX: (a) name the derivation and its blind spot in one paragraph — value-diff of `ri` vs `scrub_pii(&ri)` over an all-sentinel fixture, with a per-field `assert_ne!` precondition; (b) give the matrix a third per-cell verdict (`no such state`, with the type as the reason) so the loop can fail on a genuinely missing row without failing on an impossible one.

---

    [I] §5.1's derived divergence table omits the IP PIN, whose replacement removes a
        printed cell entirely — and the rule as written also captures every header
        identity field, so the table applies a filter the rule does not state.

WHERE: §5.1 (the rule and the eight-row table); §3.2's IP PIN carve-out; §8 step 8 ("the claims name **every** member of §5.1's derived divergence table"). `crates/btctax-core/src/tax/scrub.rs:161-164` (`ip_pin: None`); `crates/btctax-forms/src/form1040_full.rs:449-450` (`if let Some(pin) = &header.ip_pin { text(w, p, &cells.ip_pin, pin.digits()); }`).

FAILURE: §5.1's rule is *"every field `scrub_pii` replaces that reaches a PRINTED CELL or a USER-FACING MESSAGE is a divergence, and the help names it."* `ip_pin` is replaced (with `None`, the maximal replacement) and it reaches a printed cell on Form 1040. It is not in the table.

Concrete vector, and it is r2's I-2 harm model on the one field where the cell *disappears* rather than changes: a filer reports "my IP PIN prints outside its comb on the 1040" — a live class in this repo, which has `wrap.rs`, `overflow.rs` and a read-back `verify.rs`. The scrubbed copy has `ip_pin: None`, so `form1040_full.rs:449` never fires and the cell is blank. The maintainer, holding a help text that enumerates the divergences and does not mention this one, cannot reproduce it. And §3.2's own carve-out states outright that *"neither §3.3 test can see it because nothing computes from it"* — confirmed: test 2 compares `ReturnHeader::build(..).err()`, and a valid IP PIN yields `Ok` on both sides, so the drop is invisible to every instrument §3 names. Nothing will ever surface this by reddening.

Separately, the rule over-generates in the other direction: `first_name`, `last_name`, `ssn`, `occupation`, `address_street/city/state/zip`, `dependents[].name` and `dependents[].ssn` are all replaced and all reach printed cells on Form 1040, and none is in the table. The table is therefore maintained by an unstated filter, which is the same defect r2 filed — a set chosen by something other than the rule that is claimed to choose it — and a future re-derivation will not reproduce the current eight rows.

FIX: add the `header.ip_pin` row (`reaches` = Form 1040 IP PIN cell, `class` = **printed**, and note that the replacement *removes* the cell rather than changing it), and state the filter in one clause — e.g. "the header identity the help already announces as replaced is not enumerated individually; a replacement that changes the **presence or shape** of a printed cell is, and the IP PIN drop is one."

---

    [M] §2.2's `year - 1` comment still names a mechanism the disjunct does not
        secure — r2's M-1 was answered by substituting a second inaccurate mechanism.

WHERE: §2.2 (`|| digital_asset_activity(state, year - 1)   // the prior year's ledger feeds THIS year's carryforward-in figures`). `crates/btctax-core/src/tax/compute.rs:415` (`carryforward_out`), `:450` (the mismatch warning), `crates/btctax-cli/src/cmd/tax.rs:463-487` (M4), `:506-…` (`write_back_carryover`).

FAILURE: `capital_loss_carryforward_in` is a stored `ReturnInputs` field carrying its own provenance tag, and `scrub_pii` preserves both (`scrub.rs:222-223`), so it travels in the scrubbed TOML verbatim and reproduces on the recipient's machine **with no ledger at all**. Nothing derives it live from the prior year's ledger: the prior year's ledger produces the prior year's `carryforward_out` (`compute.rs:415`), which is either compared against the declared in-figure by `carryforward_consistency` (`compute.rs:450`, the M4 path r2 already refuted) or persisted into the next year's stored inputs by the separate `write_back_carryover` command. So the new comment is a second wrong answer to the same question, in a section whose stated obligation is that *every disjunct names the mechanism that put it there*. As r2 found, the direction is over-refusal and it does not gate.

FIX: say what is true rather than picking a third mechanism — the disjunct is retained as **deliberate over-refusal**: the prior year's ledger is the input from which this year's carryforward figures were derived on the filer's machine, and scrub declines to prove the negative that no path reads it live.

---

    [M] §8 schedules the fixtures the matrix requires (step 5) after the test that
        fails without them (step 4) — the same defect the fold merged steps 3+5 to fix.

WHERE: §8 step 4 ("the two §3.3 class-level tests, each RED first, over the §3.3 fixture matrix … **fails on a member with no row**") vs step 5 ("a `b_1099` fixture to exercise the payer loop; a `foreign_accounts: Some(true)` fixture, **since no fixture is in that class today**").

FAILURE: §8 step 5 states plainly that the rows step 4's test fails on do not exist yet, so at the step-4 gate the suite is red — and §8 step 3's own note, added by this fold, says "A red suite is itself a blocking finding here." Contingent on the builder not pulling the fixture work forward into step 4, which is why this is Minor and not Important — the same reasoning r2 applied to its twin (M-2).

FIX: move the fixture creation into step 4, leaving step 5 with §3.4, the disclosure test and the KEPT-fields vector.

---

    [M] §8 step 8 instructs the help to name "every member" of a table whose last two
        rows §5.1 explicitly classes as NOT divergences.

WHERE: §8 step 8 vs §5.1's closing ★ ("The last two rows are *not* divergences and are recorded so the next reader does not re-derive them").

FAILURE: The table is an enumeration of the *derivation* (eight rows), not of the *divergence set* (six). Read literally, step 8 puts `w2s[].employer` and `g_1099[].payer` into the user-facing help as divergences. Harmless if it happens, but §8 is the instruction an implementer follows without re-reading §5.1's last paragraph.

FIX: "…name every member the table classes as **printed** or **message**."

---

    [N] "`w2s[].employer` reaches nothing" is true only under an exclusion the table
        does not state.

WHERE: §5.1's table, row `w2s[].employer` ("**nothing** — no printed cell, no message"); `crates/btctax-input-form/src/spec/sections.rs:642` (`get: |ri, a| ri.w2s.get(..).map(|w| FieldValue::Text(w.employer.clone()))`).

Per the brief's invitation, I went looking for a reader of `w2s[].employer` and `g_1099[].payer` and **agree with both verdicts**: neither reaches a printed cell (`btctax-forms` writes no W-2 and no 1099-G; `w2s[].ein` is likewise unprinted — the only `ein` cell in `btctax-forms` is Form 8283's `donee_ein`) and neither reaches a computed message. The one reader that exists is the input-form surface echoing its own stored value back to the user, which is not a divergence in §5's sense — but "nothing" is the wrong word for it, and it is that unstated exclusion, not the absence of a reader, that makes the row correct.

---

    CLEAN — checked in the fold, and I could not break these

Recorded so a fourth round, if there is one, does not re-spend budget here:

- **The `NotDigits(_) → NotDigits('x')` coarsening and §3.3 test 2's asymmetry are coherent, and `'x'` is a safe choice.** `Ssn::canonical` (`packet.rs:56-70`) checks in the order `Missing` (empty after stripping hyphens/whitespace) → `NotDigits` (**first** non-digit, via `find`) → `WrongLength`. Because `NotDigits` is checked *before* length, a stand-in whose first non-digit is `'x'` yields `NotDigits('x')` at any length, and `'x'` is itself a non-digit so the variant cannot collapse into `Ok`. `IpPin::canonical` (`:167-179`) has the identical order and strips only whitespace, so the carve-out's malformed leg inherits the coarsening rather than needing its own rule. Value-equality-except-`NotDigits`-by-discriminant is the only comparison that holds all three legs: `Missing` and `WrongLength(n)` hold by value, and `NotDigits` *must* be discriminant-only precisely because §3.2 mandates the payload change. `HeaderError` has exactly one `SsnError` carrier (`HeaderError::Ssn`, with `From<SsnError>`), and both the SSN and IP PIN paths funnel through it, so one rule covers both. No leak: the emitted payload is a constant.
- **Preserving emptiness leaks nothing**, in either the `""` form the fold wrote or the `trim()` form finding 1 asks for. An empty or whitespace-only string carries no identity, and the three predicates that read it (`return_refuse.rs:672`, `:798`, `:901`) are the complete set of emptiness readers in the screen — I enumerated them rather than sampling.
- **§7's out-of-scope call is safe.** Nothing else in the spec depends on committing a scrubbed return as a fixture: §3.4 deliberately routes around it by moving the scrub constants, and every fixture §8 step 5 requires is hand-authored in `testonly.rs`. I also checked the decision against the instrument it is about: `scripts/pii-scan-generic.sh` does have a structural SSN window (`ALLOWED_SSN_IMPOSSIBLE`, the middle-group-`00` clause) that `synthetic_ssn`'s output lands inside, and has no EIN analogue — so "narrow to SSNs" is not merely the safer branch, it is the only one the scanner can express.
- **§4.3's `--force` scoping is complete as one clause.** `import_return_inputs`'s parked-draft refusal is raised by `coherence_clear_or_refuse` before any marker logic, and the spec now says the flag does not reach it.

---

DID THE FOLD CLOSE r2?

- **I-1** (`SsnError::NotDigits` payload undecided) — **closed.** §3.2 carries the coarsening *and* its reason; §3.3 test 2 states the comparison and why it is asymmetric. Verified against `packet.rs` rather than against the fold's description of it.
- **I-2** (§5 named two divergences because two red an `AbsoluteReturn` comparison) — **partially.** The rule is now a derivation, the three members r2 named are present, and §3.3 records why the figure invariant cannot see the printed-forms surface. But the IP PIN — replaced, printed, invisible to both tests — is still missing, and the rule's boundary is unstated in both directions (finding 4).
- **I-3** (field axis keyed to a hand-list) — **partially.** The axis is now the right one, and `foreign_country_names` is in it. But the derivation has no computable source and the state axis has no "no such state" verdict (finding 3), and the rule the axis exists to enforce is keyed to the wrong emptiness predicate (finding 1) and scoped away from the one field where the class flip removes a refusal (finding 2).
- **I-4** (synthetic-EIN / pii-scan option A weakens a guard) — **closed.** Decided in the document, with the refutation and the premise citations, and the `scrub.rs` claim narrowing scheduled at §8 step 6.
- **M-1** (`year - 1` comment) — **not closed.** One inaccurate mechanism replaced by another (finding 5). Still Minor; still over-refusal.
- **M-2** (§8 steps 3 and 5 split) — **closed for 3+5, new instance between 4 and 5** (finding 6).
- **M-3** (`--force` unscoped) — **closed.**

---

WHAT WOULD MAKE THIS REVIEW WRONG: Every finding assumes an implementer builds §3.3 and §5.1 from the *rules and steps as written* rather than from §3.2's ★ mechanism sentence — someone who treats ★ as the authority, derives the replaced set by diffing `scrub_pii`'s output, and asks of each member "what predicate reads this" would land findings 1, 2 and 3 correctly with no spec change; and finding 4 additionally assumes the help is meant to warn a recipient about a printed cell that goes *missing*, not only about one whose contents change.
