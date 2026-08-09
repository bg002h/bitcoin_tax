# r2 review — `design/SPEC_income_scrub.md`

_Independent re-review, Opus, read-only, dispatched against `reviews/BRIEF-scrub-spec-r2.md`.
Branch `feat/income-scrub` @ `1e807f5`. Persisted VERBATIM before any fold._

---

VERDICT: needs-changes

CRITICAL: 0
IMPORTANT: 4

I looked hardest at **§3.2/§3.3 together** (the validity-class rule and the instrument that is supposed to hold it) and at **§5's divergence list**. §2.2's four-disjunct predicate — the section the brief flagged as the safety gate — I could not break; see CLEAN below. The blocking findings are all in the fold's *other* new text, and three of the four are the same mechanism: **§3.2 states a rule over a mechanism, and §3.3/§5 then implement it against a hand-list of four fields that the mechanism does not stop at.**

---

    [I] §3.2 and §3.3 do not decide the `SsnError::NotDigits` payload, and no single
        comparison satisfies both as written: one branch emits a character of the
        filer's real entry into the file the command stamps shareable, the other
        drops the payload preservation r1 filed the section to add.

WHERE: §3.2 (`Missing → Missing`, `WrongLength(n) → WrongLength(n)`, `NotDigits → NotDigits`) and §3.3 test 2 ("the same `HeaderError` variant, **and the same `SsnError` variant inside it** — not merely `is_err()` equality"); `crates/btctax-core/src/tax/packet.rs:66-68` (`SsnError::NotDigits(c)` is populated from the filer's raw entry), `:93-102` (the enum), `:167-179` (`IpPin::canonical` returns the same `SsnError`, so this governs the IP PIN carve-out's malformed leg too).

FAILURE: Take a filer whose SSN cell holds `123-45-678O` (letter O for zero — the canonicalizer strips only hyphens and whitespace, so the offending character reaches the error). `Ssn::canonical` returns `NotDigits('O')`.

- Implement §3.3 test 2 as **value** equality — `assert_eq!(ReturnHeader::build(orig).err(), ReturnHeader::build(scrubbed).err())`, which is the idiom this repo already uses on this exact call at `return_refuse.rs:1526-1535`, and the *only* comparison that also holds §3.2's explicitly-payloaded `WrongLength(n) → WrongLength(n)` — and the scrubbed SSN must contain that same `'O'`. A character of the filer's real entry ships inside a file the command authorizes as safe to hand to a stranger, contradicting §3.2's own governing sentence ("*no original identity value is emitted in any class*") one paragraph above. It lands **green**, blessed by the test.
- Implement it as **discriminant** equality instead and the test no longer holds `WrongLength(n) → WrongLength(n)` — which is precisely the leg r1 filed: filer reports *"an SSN has 4 digits"*, the maintainer's copy says *"has 2 digits"*, both `WrongLength`, test passes.

r1's fix named the resolution verbatim — `NotDigits(_) → NotDigits('x')`, "recorded as the one deliberate coarsening, with the disclosure reason written down beside it". The fold transcribed the two legs whose payloads are safe and dropped the third leg *and* its reason. This is the dropped-term-becomes-invisible pattern the project CLAUDE.md names, applied to the one payload that carries content.

FIX: two clauses. §3.2: "`NotDigits(_) → NotDigits('x')` — the one deliberate coarsening, because the payload is a character of the filer's real entry and §3.2's governing sentence outranks payload fidelity here." §3.3 test 2: state the comparison as **value equality on `HeaderError`, with `NotDigits` compared by discriminant only**, and say why the asymmetry exists.

---

    [I] §5 names two surviving divergences because those are the two that RED an
        `AbsoluteReturn` comparison. At least three more replaced strings print
        verbatim on filed pages, and every test §3 names is structurally blind to
        all of them.

WHERE: §5 ("Two divergences survive the ledger refusal… The help names them"); `crates/btctax-core/src/tax/scrub.rs:243, :265-276`; `crates/btctax-core/src/tax/printed.rs:1095-1096` and `crates/btctax-forms/src/schedule_b.rs:84, :183-192`.

FAILURE: The mechanism §5 should have used is "every field `scrub_pii` replaces that reaches a printed cell or a user-facing message." What it actually used is "every field that makes `assert_eq!(a, b)` fail in `scrub_preserves_every_computed_figure`" — and that test compares `assemble_absolute`, whose `PrintedInputs` (`return_1040.rs:1388-1435`) carries no payer and no country list. Everything on the **printed-forms** surface (`assemble_printed_forms`, `ReturnHeader`) is therefore outside every instrument §3 names, and three more replaced strings live there:

- `foreign_country_names` → `ScheduleBLines.line7b_countries` → **Schedule B line 7b, printed verbatim** (`schedule_b.rs:183-192`). Scrub rewrites the content *and* the shape: `scrub_name_list` splits on `,` only, so `"Panama and Belize"` (one comma-free entry) becomes a single `Country1`, and any non-comma delimiter collapses the count.
- `int_1099[].payer` / `div_1099[].payer` → **Schedule B Part I / Part II payer rows, printed verbatim** (`schedule_b.rs:84`). A real payer string becomes a 6-character token, so anything length- or character-dependent on that cell (this crate has `wrap.rs`, `overflow.rs`, and a read-back `verify.rs`) does not reproduce.
- `b_1099[].payer` → embedded in the `Form1099BNeedsForm8949` refusal text (`return_refuse.rs:672-676`) — see the next finding, which is the sharper half.

Concrete vector: a filer reports "Schedule B line 7b prints my country list wrong." The scrubbed copy prints `Country1, Country2`. The maintainer, holding a help text that says every figure is preserved and names exactly two exceptions, closes it not-reproducible. That is §2.2's own justification for refusing ledger years ("sends the recipient after a bug that is not there") arriving through the door §5 left open.

FIX: replace §5's two-item list with the derivation — "every field `scrub_pii` replaces that reaches a printed cell or a refusal message" — and enumerate the current members (Schedule C line A + Form 8995-A 1(a); Schedule B line 7b; Schedule B Part I/II payer rows; the excess-SS advisory EIN). Add one sentence to §3.3 recording that the figure invariant is `AbsoluteReturn`-scoped and therefore cannot see the printed-forms surface, so that set must be maintained by derivation rather than by waiting for a red.

---

    [I] §3.2's ★ rule is stated over the mechanism; §3.3's matrix is keyed to the
        hand-list. A field `scrub_pii` replaces today has its class read by
        `screen_inputs` and is not class-preserved — and nothing the spec specifies
        can see it.

WHERE: §3.2 ★ ("every field `scrub_pii` replaces must preserve every property any screen, `ReturnHeader::build`, or form filler reads from it. §3.2's four fields are the currently-known instances, not the definition") vs §3.3's matrix `{SSN, EIN, IP PIN, business_description} × {absent, malformed, valid}`; `crates/btctax-core/src/tax/scrub.rs:271-273` and `crates/btctax-core/src/tax/return_refuse.rs:672-676`, `:798`.

FAILURE: Two instances, both verified in current source, neither in the matrix.

1. **Live violation.** `screen_inputs` reads `b.payer.trim().is_empty()` to pick the refusal wording — `"(unnamed broker)"` vs the payer's name (`return_refuse.rs:672-676`). `scrub_pii` sets `f.payer = format!("Broker{}", i + 1)` unconditionally, so an **empty** payer becomes non-empty. §3.2's ★ rule is violated as written, today. §3.3 test 1 compares only the `RefuseReason` **variant**, which is `Form1099BNeedsForm8949` on both sides — structurally blind. (I checked whether `RefuseReason`'s payloads could rescue this: the two payload-carrying variants, `NegativeAmount(String)` and `InconsistentDividendSubset(String)`, carry field/box *names*, never identity, so variant-vs-value equality makes no difference here. The difference lives in `Refusal`'s message, which §3.3 does not compare.)

2. **Unheld guarantee.** `screen_inputs:798` refuses when `foreign_accounts == Some(true) && foreign_country_names.trim().is_empty()`. `scrub_name_list` preserves that class correctly today — and nothing requires it to. Mutation: make `scrub_name_list` return `String::new()` unconditionally. The §3.3 disclosure test still passes (the names are certainly gone); test 1 cannot fire because **no fixture exercises the class** — `kitchen_sink_household` sets `foreign_accounts: Some(false)` (`testonly.rs:352`) and no other fixture sets it at all; and the matrix does not require such a cell because `foreign_country_names` is not on its field axis. The mutation ships green, and it turns a filing return into a refusing one — the exact inverse of §3.2's stated harm.

FIX: derive the matrix's **field axis from `scrub_pii`'s replaced set** rather than from §3.2's four, and fail the loop on a replaced field with no row — the same "enumerate from the mechanism, never from a hand-list" §3.3 already applies to the *state* axis. Then either preserve `payer` emptiness in scrub, or record it as a deliberate exception with the reason.

---

    [I] §7's first option for the synthetic-EIN / pii-scan conflict asks for a
        "documented structural window" that this repo has twice recorded does not
        exist for EINs — and the generator-keyed rule that satisfies it literally
        exempts real EINs from the PII scan.

WHERE: §7 ("Mint them inside a documented structural window with a matching `ALLOWED_EIN_SYNTHETIC` rule keyed to the generator… — **or** narrow the scrub.rs claim to SSNs"); `crates/btctax-core/src/tax/scrub.rs:57-58`; `scripts/pii-scan-generic.sh` (EIN section) and its SSN section.

FAILURE: Both the code and the scanner already state the finding's premise. `scrub.rs:57-58`: *"There is no 'impossible EIN' — the IRS has issued prefixes across nearly the whole 2-digit space — so this is merely synthetic, not provably unissued."* The scanner's EIN block: *"No structural rule is available… there is no 'impossible EIN' to build a mechanism from. Token-exact… remains correct here."* The SSN rule works only because group `00` is provably never issued; there is no EIN analogue.

`synthetic_ein(n) = format!("9{}-{:07}", n % 10, n + 1)` spans the entire `9x` prefix space. The rule "keyed to the generator" is therefore `^9[0-9]-[0-9]{7}$`, which exempts every EIN issued under the 90–99 prefixes — 94 and 95 (California), 91 (Washington), 99 — from a scan whose stated purpose is stopping a real taxpayer identifier from reaching a public repo. That is the ITIN trap the scanner's own SSN comment spends eleven lines warning about ("Widening an exemption is never the safe edit"), reproduced on the EIN axis, and nothing would go red: the scan passes either way. The two options §7 offers are presented as equivalent and are not — option A, taken at face value with the current generator, weakens a PII guard; option B leaves the spec's own intended workflow (commit a scrubbed return as a fixture) blocked.

FIX: decide it in the spec rather than in the build. Either (a) fix the window's exact shape *and* change `synthetic_ein` to emit only into it — a shape narrow enough that its real-EIN intersection is stated and accepted in writing — or (b) take option B and say plainly that committing a scrubbed return as a fixture is out of scope for v1. Do not ship a build step whose two branches differ in whether a security guard is widened.

---

    [M] §2.2's `year - 1` disjunct does not secure the mechanism its doc comment
        names.

WHERE: §2.2 (`|| digital_asset_activity(state, year - 1)   // M4 carryforward-consistency advisory`); `crates/btctax-cli/src/cmd/tax.rs:465-487`.

FAILURE: The M4 advisory needs the prior year's *profile*, not just its ledger: `s.resolve_screened(&state, year - 1, &tables)` yields `Ready { profile: None }` on a recipient who imported one year, so `carryforward_consistency` is skipped and the advisory does not reproduce **whatever this disjunct decides**. r1's verifier recorded exactly this (its "CORRECTION 2") and the fold adopted the fix that correction refuted. The disjunct is still a *correct member* of `ledger_contributes` — it does prevent emitting where the prior-year ledger fed this year's advisory — so the direction is safe (over-refusal) and this does not gate. But §2.2 mandates that "every disjunct names the mechanism that put it there," and this one will ship a doc comment a future maintainer will read as "the M4 advisory reproduces when this predicate is false."

FIX: keep the disjunct; change its comment to what it actually secures ("the prior year's ledger feeding this year's carryforward-out"), and add the residue to §5's divergence list: cross-year advisories do not reproduce from a one-year file.

---

    [M] §8 sequences §3.1 two steps before the normalization §3.3 says §3.1
        requires.

WHERE: §8 steps 3 and 5; §3.3 ("The existing figure invariant needs one more normalization **or it reds on a CORRECT scrub**").

FAILURE: §3.3 asserts the red is a consequence of §3.1 landing. §8 lands §3.1 at step 3 and the normalization at step 5, so if step 3's RED-first vector (two spellings of one employer over the SS cap) goes into the figure-invariant loop — the natural place, since that is the test the fix is proving out — the suite is red across the step-3 and step-4 gates, and this repo's rubric makes a red suite itself a blocking finding. r1's fix said to sequence the two together; the fold split them. Contingent on where the builder puts the vector, hence Minor rather than Important.

FIX: fold step 5 into step 3.

---

    [M] `--force` is a new flag on a command that already has an unconditional
        refusal, and §4 does not scope it.

WHERE: §4 item 3 and §5's permitted claim "load it with `income import --force`"; `crates/btctax-cli/src/cmd/tax.rs:57` → `crates/btctax-cli/src/input_form_store.rs:238-254`.

FAILURE: `import_return_inputs` already calls `coherence_clear_or_refuse`, which returns `CliError::ParkedDraftBlocksWrite` — the C-1 guard on "the sole copy of a screened return." A flag literally named `--force`, introduced with no stated scope, invites an implementer to gate that refusal on it too; that destroys irreplaceable data, which this rubric grades Critical. The spec's phrasing ("refuses a marked file without `--force`") does scope it by construction, which is why this is Minor and not Important — but the scope is inferred, not stated, and it is one clause.

FIX: §4 item 3: "`--force` overrides the provenance-marker guard and nothing else; the parked-draft refusal is unconditional."

---

    CLEAN — checked, and I could not break these

Recorded so the next round does not re-spend budget here:

- **§2.2's four disjuncts are complete, as far as I can reach.** `Severity` has exactly two variants, `Hard` and `Advisory` (`state.rs:18-21`), and every gating read of the blocker set filters on `Hard` (`admin.rs:234`, `:712`, `:972`), so disjunct 4 covers the whole projection-wide gate surface rather than one instance of it. Every other `state` read on the report/export path takes a `year` and filters on it (`schedule_d`, `se_net_income`, `crypto_income`, `crypto_charitable_gifts`, `render_gift_advisory`, `render_donation_appraisal_advisory`, `advisories_for`, `screen_absolute`). The pseudo *placeholder* channel (`resolve_and_screen` with `cfg.pseudo_reconcile`) fires only when the year has **no** stored profile, which scrub cannot reach. `year - 1` is the right carryforward window: computing the prior year's `carryforward_out` reads the prior year's stored profile, never the year before that. Carryforward *out* is safe: `write_back_carryover`'s own two gates are pseudo-active and Hard-blocked, both disjuncts here.
- **The marker cannot be silently stripped by btctax.** No in-repo path parses and re-serializes a scrubbed `ReturnInputs` TOML; the TUI store keeps JSON. And §4's own test mandate ("an assertion that **the emitted string** carries the marker") forces the marker into `scrub_return_inputs`'s return value, so the stdout path carries it identically to `--out` — the one asymmetry I went looking for and did not find.
- **§8 step 2 is compatible with the current signature.** `scrub_return_inputs -> Result<Option<String>, CliError>` already carries a `CliError`, and `main.rs:307-321`'s `None => "No full-return inputs set"` is the no-row case, which does not collide with an `Err` refusal.
- **§6's dependent-DOB drop moves no figure.** The only non-test readers of `date_of_birth` in the workspace are `packet.rs:306,314` and `advisories.rs:771-793,922`, all taxpayer/spouse. No reader of a *dependent's* DOB exists.
- **The IP PIN carve-out is coherent.** `IpPin::canonical` (`packet.rs:167-179`) errs only on malformed, so `absent or valid → None` is invisible to `ReturnHeader::build`'s `Result`, and a stand-in preserving `WrongLength(n)` for any n ≠ 6 can never canonicalize into a well-formed credential. Its one open edge is the `NotDigits(char)` payload it shares with the SSN — finding 1, not a second defect.

---

WHAT WOULD MAKE THIS REVIEW WRONG: every blocking finding assumes an implementer follows §3.3's and §5's *instruments* literally — the four-field matrix, the two-item divergence list, the `AbsoluteReturn` comparison — rather than re-deriving the field set from §3.2's ★ mechanism sentence each time; an implementer who reads ★ as overriding the lists would land three of the four correctly without a spec change.
