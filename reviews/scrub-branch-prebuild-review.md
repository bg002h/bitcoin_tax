# Whole-branch review — `feat/income-scrub` (`main..HEAD`)

_Independent Opus review, read-only, dispatched against `reviews/BRIEF-scrub-branch-prebuild-green.md`.
Harness B3: scoped to the WHOLE branch (24 commits) and pointed at INTERACTION. HEAD `344c1e11`.
Persisted VERBATIM before any fold._

---

VERDICT: needs-changes

CRITICAL: 1
IMPORTANT: 1

---

**[C] A malformed EIN is still upgraded to a well-formed synthetic one — the scrubbed copy FILES where the original refused, and computes a §6413(c) credit of $1,546.80 the original does not have. This is the half of r1's CRITICAL the spec assigned to §3.2 and the build never implemented, and the matrix row built to catch it is structurally vacuous.**

WHERE: `crates/btctax-core/src/tax/scrub.rs:631-633` (`w.ein = Some(eins.map(e))`, guarded only on `!e.trim().is_empty()`) and `:286-293` (`EinMap::map` always returns `synthetic_ein`, which is always nine digits and therefore always canonicalizes). Vacuous cell: `crates/btctax-core/src/tax/scrub_axis.rs:685-689` (`w2s[].ein` @ malformed) against `crates/btctax-core/src/tax/scrub_axis.rs:159` (`box4_ss_withheld: dec!(62)`). Screen: `crates/btctax-core/src/tax/return_refuse.rs:932-956`. Figure: `crates/btctax-core/src/tax/return_1040.rs:784-811`.

FAILURE: Take the branch's own `two_spellings_household` (`scrub.rs:718-740`) and change one character — `ein: Some("111111111")` → `Some("11111111")` (eight digits, a dropped digit; the ordinary hand-typing typo). Two Taxpayer W-2s, box 4 = $6,000 each, aggregate $12,000 > the $10,453.20 §3101(a) cap.

- ORIGINAL: `canonical_ein("11111111")` is `None`, so `over_cap_needs_ein(Taxpayer)` is true → `screen_inputs` refuses `ExcessSsEmployerUnknown`. `excess_social_security` takes its `None => return Usd::ZERO` arm → Schedule 3 line 11 = **$0**.
- SCRUBBED: `EinMap` keys the valid one on `"111111111"` → `"90-0000001"`; the malformed one falls back to its raw string as a key → `"91-0000002"`. **Both synthetics canonicalize.** `over_cap_needs_ein` is now false → **no refusal**; `eins.len() == 2` → creditable `= min(6000,cap)·2 = 12000` → line 11 = **$1,546.80**, `total_payments` and the refund rise by the same.

The filer's own return is blocked by that refusal — which is precisely why they run `income scrub` ("so they can reproduce a refusal", `cli.rs`) — and the file they are told is a faithful copy neither refuses nor computes the same return. `--help`/`btctax-income-scrub.1` state "every fail-loud declaration" survives; false here. `import --force` puts the manufactured credit into a real vault, where `export-irs-pdf` prints it.

Why nothing reds: `EinMap`'s own doc (`scrub.rs:278-281`) explicitly delegates class preservation — *"Preserving the malformed class into the output is §3.2's job, not this map's"* — and §3.2's implementation covers the SSN (`class_preserving_stand_in`) and the IP PIN (`scrub_ip_pin`) and **has no EIN leg at all**. SPEC §3.2 names the field: *"Applies to **SSN, EIN**, `business_description`, and — with a carve-out — the IP PIN."* The §3.3 matrix *does* carry a `w2s[].ein @ malformed` row, and `assert_same_refusal` *would* red on it — except `maximal_sentinel`'s two W-2s withhold $62 each ($124 total), so `over_cap_needs_ein` can never fire on any cell of that row. The cell is exercised and cannot discriminate: B1's exact failure mode, in the instrument built to close B1. Reachable from both write paths — `parse_return_inputs_toml` validates only unknown *keys*, and `sections.rs:668` stores `Some(s)` untrimmed with no format check.

FIX: give the EIN its class-preserving leg, mirroring `synthetic_ssn_like` — when `canonical_ein(e).is_none()`, emit a stand-in that also fails `canonical_ein` (e.g. eight digits, `format!("9{}-{:06}", n % 10, n + 1)`) so the undecidability survives; leave the canonical case as it is. Then make the matrix cell able to fail: raise `maximal_sentinel`'s `box4_ss_withheld` over the cap, or add one over-cap malformed-EIN vector to `scrub_preserves_every_computed_figure` (the one-character edit above). Verify it reds first.

*(Severity note: r1 filed this half as IMPORTANT, reasoning that no signed return moves. I am raising it because r1's own CRITICAL named it as the adjacent instance the fix **must cover**, the spec §3.2 mandated it by name, and the instrument built to police it was shipped never having been observed discriminating on this cell — with the last gate before an irreversible publish being this pass.)*

---

**[I] The TOML round-trip test cannot fail on a lossy emit, and its fixture is the thinnest household in the repo — so the emitter's fidelity is a guarantee no test holds.**

WHERE: `crates/btctax-cli/tests/scrub_refusal.rs:635-648` (the `assert_eq!(sent, landed)`) with the fixture at `:73-93`.

FAILURE: `sent` is `toml::from_str(read_to_string(&out))` and `landed` is the recipient vault's row, which is `parse_return_inputs_toml(&that same text)` round-tripped through the DB. **Both sides derive from the emitted file**, so the assertion tests the storage layer, not the emitter. Every `ReturnInputs` field is `#[serde(default)]`, so a field the emitter drops is absent from the file, defaults on both sides, and compares equal. Mutation: in `scrub_return_inputs` (`cmd/tax.rs:225-229`) emit `&{ let mut z = scrubbed.clone(); z.payments = Default::default(); z }` — the whole workspace stays green while every recipient's copy loses the filer's estimated-tax and extension payments and computes a different balance due. `z.schedule_a = None` does the same for an itemizing filer. The fixture is `w2s: vec![W2{..Default::default()}]` with everything else defaulted: no `box12`, no Schedule A/C, no dependents, no 1099s, no carryovers — so the array-of-tables-inside-array-of-tables shape this repo elsewhere asserts *cannot* serialize (`fullreturn_oracle.rs:40-42`, `cmd/tax.rs:198-199`: "`toml::to_string(&ri)` fails `ValueAfterTable`") is never emitted by any test. `scrub_return_inputs` works only because toml 0.8 buffers and hoists scalars above tables; nothing pins that.

FIX: in that test, compare against the in-memory scrub rather than the file — read the filer's stored row and assert `landed == scrub_pii(&stored)`. That is the only comparison that can red on emitter loss. Pair it with a richer fixture carrying W-2 `box12` entries, a `schedule_a.charitable` array, a `schedule_c`, one dependent and one `charitable_carryover_in` item, so the nested shapes are actually serialized.

---

**[M] The default output path — stdout — is not owner-only, and the help asserts it unconditionally.**
WHERE: `crates/btctax-cli/src/main.rs:329` (`None => print!("{toml}")`) vs `crates/btctax-cli/src/cli.rs:446` and `docs/man/btctax-income-scrub.1`. `--out` is optional; the natural invocation `btctax income scrub --year 2024 > s.toml` lands 0644 — the same world-readable exposure §4.1 exists to close — while the help says "The file is written owner-only and carries a marker line." Fix: qualify the sentence to `--out`, or note on stderr when stdout is not a TTY that a redirected file is not permission-hardened.

**[M] `--out` writes the full scrubbed return at the pre-existing file's mode, then narrows it.**
WHERE: `crates/btctax-cli/src/main.rs:321-322`. `write_owner_only`'s `.mode(0o600)` applies only at create; the code comment says so. For a pre-existing 0644 path — the case the test at `scrub_refusal.rs:447` names as the common one ("scrub is exactly the command a filer re-runs to the same path") — the content is written world-readable and only then restricted, and if `restrict_file_to_owner` errors the file stays 0644 with the content in it. The test asserts the final mode, which is exactly the window it cannot see. Fix: `let _ = std::fs::remove_file(&path);` before `write_owner_only`, so the file is always created fresh at 0600.

**[M] `pii-scan-generic.sh` and its own conformance test assert the opposite of what SPEC §7 decided about the synthetic-EIN window.**
WHERE: `scripts/pii-scan-generic.sh:127` ("SPEC_income_scrub.md §7 moves `synthetic_ein` into a documented STRUCTURAL window so future output needs no entry here at all") and `crates/btctax-cli/tests/repo_hygiene.rs:287` ("which is why SPEC_income_scrub §7 moves the generator into a structural window instead of growing this list"). §7 decided the exact reverse — *"there is no structural EIN window, so the generator-keyed allowlist is not available at any price"* — because `^9[0-9]-[0-9]{7}$` would exempt real EINs issued under 91/94/95/99. `scrub.rs:57-64` states the correct version. Both stale comments were written in `c897aefd`, before the §7 fold in `ffdf2497`, and they now sit inside the security control telling the next maintainer that the widening §7 refused is already sanctioned. Fix: replace both sentences with §7's actual decision (the bucket is permanent residue; committing a scrubbed return as a fixture is out of scope for v1).

**[M] `income scrub` discards the `StaleNote` from `input_form_store::load`.**
WHERE: `crates/btctax-cli/src/cmd/tax.rs:201` (`...load(s.conn(), year)?.0`). It is the only caller of `load` that drops it; `btctax-tui-edit` carries it to the status line. A filer with a schema-stale WIP draft gets the committed row scrubbed with no word that the draft they were editing was skipped — the softer form of the "silently emits a return the filer is not looking at" harm §7 exists to close. (No data loss: scrub never calls `s.save()`, so `load`'s in-memory `delete_draft` never persists.) Fix: bind the note and `eprintln!` it.

**[N] `scrub.rs:839-843` still says `business_description` is "the ONLY identity-bearing string that survives into `AbsoluteReturn`" and calls the block "ONE normalisation"; `:856-867` then adds the second and third members and closes with "Three members, each with a reason."** The code is right; r1's MINOR on this sentence was folded around it rather than into it. One-line delete.

**[N] r1's sweep NIT is still open verbatim.** `cmd/tax.rs:226-229` reports a `toml::to_string_pretty` failure as `CliError::BadConfigValue { key: "return_inputs[2024]" }` — documented as "a `cli_config` row held an unrecognized value (corrupt DB…)", whose natural remedy (clear the row and re-import) is destructive; and `main.rs:321`'s `?` surfaces a bare `io::Error` with no path.

---

**r1 FINDINGS vs THE BUILD** (seam 1 — read against current source, not against the spec's description):

| r1 finding | status |
|---|---|
| C — `EinMap` keys the RAW EIN, splitting one employer into two (filed 3×) | **closed** — `scrub.rs:287` keys `canonical_ein`, held by `two_spellings_of_one_employer_survive_as_one_employer` + a 4th figure-invariant vector |
| C — …its **named adjacent instance**: "scrub.rs:261 replaces any non-empty EIN, including a MALFORMED one… **the fix must cover it**" | **NOT closed** → C-1 |
| I — scrub normalizes absent/malformed identity into valid, erasing 4 screens | **partially**: SSN closed (`class_preserving_stand_in`), IP PIN closed (`scrub_ip_pin`, correctly using `IpPin::canonical`), `business_description` closed (`replace_preserving_emptiness`); **EIN not closed** |
| I — `the_identity_does_not_survive` names 6 of 16 fields (spouse untested) | **closed** — `check_person` closure over both people + a planted spouse; payers/employers covered by `the_surviving_sentinels_are_exactly_the_deliberately_kept_fields` |
| I — empty `business_description` overwritten unconditionally | **closed** |
| I — malformed EIN upgraded → `ExcessSsEmployerUnknown` vanishes | **NOT closed** → C-1 |
| I — uncaptured/malformed SSN → well-formed synthetic | **closed** (`Missing→""`, `WrongLength(n)→"1".repeat(n)`, `NotDigits(_)→"x"`; all three verified against `Ssn::canonical`'s NotDigits-before-length ordering) |
| I — a return that refuses to PRINT scrubs into one that prints | **closed** for the SSN and IP PIN legs; the EIN leg is a `screen_inputs` refusal, still open (C-1) |
| I — `Person` KEPT `date_of_birth`/`date_of_death`/`blind` are `None` on every fixture | **closed**, though not where r1 aimed: `maximal_sentinel` sets all three `Some`, so dropping any one puts a new path in the derived axis and `every_replaced_field_preserves_its_class_in_every_representable_state` fails at "no row for it". `date_of_death` is still `None==None` inside `the_identity_does_not_survive`; the matrix carries it |
| I — dependent's exact DOB retained on a false "both are read" | **closed** — dropped, comment corrected, and the drop forced a new matrix row |
| M — `..`-free guard covers 4 of 10 structs | **closed** — all ten destructured; `Box12Entry` still is not (§7 recorded only `code`) |
| M — "`business_description` is the ONLY identity-bearing string in `AbsoluteReturn`" is false | **code closed** (all three normalized + re-sort); **sentence still present** → N-1 |
| M — `--help` promises fidelity the code does not deliver | **closed** — I checked the new help member-for-member against §5.1's table: 4 printed + 2 message + the IP-PIN presence row + the cross-year residue, all present, none extra |
| M — every identity-shaped print refusal disappears | **closed** |
| M — the dependents `zip` asserts nothing if the list is dropped | **closed** — `len()` equality and a non-empty precondition assert *before* the loop |
| M — `--help` sells a round trip "held by a test"; `scrub_return_inputs` has none | **partially** → I-1 |
| N — `w2s[].box12[].code` has no recorded decision | **closed** |
| **sweep** C — the scrubbed TOML carries no LEDGER | **closed** — `ledger_contribution`, four disjuncts, one RED-first CLI test each plus a positive control |
| sweep I — scrub ignores the input-form draft | **closed** (`input_form_store::load`, parked included) |
| sweep I — `--out` bypasses `fsperms`, lands 0644 | **closed**, with the write-then-restrict window as M-2 |
| sweep M — synthetic EINs red `pii-scan` | **closed as a decision** (out of scope for v1), with the inverted comments as M-3 |
| sweep M — no marker; re-import destroys the real identity | **closed** — marker + pre-parse text scan + `--force` scoped to it alone (`coherence_clear_or_refuse` still runs unconditionally after) |
| sweep N — serialization failure reported as vault corruption; `--out` error has no path | **NOT closed** → N-2 |

**What fell out between r1 and the spec:** only the sweep NIT. `CliError::BadConfigValue` and the pathless `--out` write error appear nowhere in `SPEC_income_scrub.md` — no §, no §7 bullet, no §8 step — and are still live at HEAD. Everything else r1 filed reached the spec. **The C-1 loss is at the spec→build boundary, not r1→spec**: §3.2 names the EIN explicitly, `EinMap`'s doc forwards the obligation to §3.2, and §3.2's implementation has no EIN leg — with the §3.3 matrix cell that would have caught it made vacuous by a sentinel fixture $10,329 under the §3101(a) cap.

**On the three places the brief flagged as thin ice, I disagree with two.** (a) `replace_preserving_emptiness` per-field: the loop *does* close — I enumerated every string `scrub_pii` writes and every one preserves trim-emptiness, including the two that route through `synthetic_ssn_like`/`scrub_ip_pin` (both map their `Missing` arm to `""`) and `scrub_name_list`. The real gap is one level up: *validity*-class, not emptiness, and only for the EIN. (b) The parallel string walk in `assert_emptiness_class_preserved` zipping a shortened array is not live — `scrub_pii` changes no collection length, `diff_paths` (`scrub_axis.rs:79-86`) records a length difference as an axis member, and the matrix then fails on a path with no row; `the_identity_does_not_survive` additionally pins `dependents.len()`. (c) `ledger_contribution` returning the first disjunct is still verdict-neutral: `ledger_contributes` is literally `.is_some()`, and the CLI calls `ledger_contribution` once for both the verdict and the message, so there is one source of truth. Separately: the 0.16.0→0.15.0 delta visible in `git diff main..HEAD` on `Cargo.lock`, the ten `Cargo.toml`s and `docs/examples/examples.md` is **not** a branch edit — `git diff 06923c74..HEAD --name-only` touches none of those files; it is main's release commit `e01a150b` that the branch has not merged, and a merge takes 0.16.0.

WHAT WOULD MAKE THIS REVIEW WRONG: if some path I did not find normalizes or rejects a non-canonicalizable W-2 EIN before it reaches storage, making C-1's vector unreachable — I checked `parse_return_inputs_toml` (unknown keys only), `import_return_inputs`, `return_inputs::set` and `sections.rs:663-668` (stores `Some(s)` untrimmed), and found none.
