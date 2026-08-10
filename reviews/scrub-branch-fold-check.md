# Scoped fold check — `git diff b1356781..ef98b021`

_Independent Opus check, read-only, dispatched against `reviews/BRIEF-scrub-branch-fold-check.md`.
Checks whether the fold closed the whole-branch review's 1C/1I/4M/2N. Persisted VERBATIM before any
further fold._

---

VERDICT: **all-closed** (0 Critical / 0 Important; 2 Minor, 6 Nit, all non-blocking)

**C-1: closed.** `EinMap::map` (`scrub.rs:302-325`) now computes `canonical_ein(real)` once and branches the *stand-in* on it: `canonical.is_some() ⇒ synthetic_ein(next)`, else `synthetic_malformed_ein(next)` (`:251-253`, `format!("9{}-{:06}", n % 10, n + 1)` — eight digits, which `canonical_ein` (`return_1040.rs:681-687`, exactly-nine-digits) rejects). Keying is unchanged (`canonical` else raw), so §3.1's partition fix is not disturbed. The vector is real and non-vacuous: `malformed_ein_over_cap_household` (`scrub.rs:759-780`) puts two Taxpayer W-2s at box4 $6,000 each = $12,000 > the $10,453.20 cap with one eight-digit EIN, so `over_cap_needs_ein` (`return_refuse.rs:932-944`) fires; the new test asserts `canonical_ein(scrubbed.w2s[1].ein).is_none()` — which is the assertion that cannot be vacuous — plus refusal-reason equality and `excess_social_security` equality (0 vs the $1,546.80 the defect manufactured).

**I-1: closed.** `scrub_refusal.rs:661-673` no longer parses the emitted file for its expectation: `sent = scrub_pii(return_inputs::get(filer_vault, 2024))`, `landed` = the recipient vault's row. The file is now on one side only, so a dropped field is a real inequality rather than a shared `#[serde(default)]`. The fixture half is also closed — `store_wage_only_return` was replaced by `maximal_sentinel()` (`:611-620`), which is the only thing in the repo emitting `[[w2s]]` carrying a `box12` array-of-tables, so the `ValueAfterTable` shape is now actually serialized by a test.

**M-1: closed** — `cli.rs:446-451` now scopes the 0600 claim to `--out` and warns that a stdout redirect lands at the umask; `docs/man/btctax-income-scrub.1:19-21` carries the same two paragraphs, and `xtask/src/docs.rs:383` reds on a stale committed page, so the two cannot drift.
**M-2: closed** — `main.rs:326` does `let _ = std::fs::remove_file(&path);` before `write_owner_only`, and `open_owner_only` (`fsperms.rs:22-29`) applies `.mode(0o600)` only at create, so the file is now always created fresh. Not a new data-loss path: the old code already opened with `.truncate(true)`, so a failed write destroyed the prior content either way. (See Minor-2 — nothing reds if the `remove_file` line is deleted.)
**M-3: closed** — both stale sentences replaced with §7's actual decision; verified against `design/SPEC_income_scrub.md:605` ("there is no structural EIN window, so the generator-keyed allowlist is not available at any price"). `scripts/pii-scan-generic.sh:126-134` and `repo_hygiene.rs:287-292` now say the bucket is capped residue; the allowlist regexes themselves are byte-identical. I ran `scripts/pii-scan-generic.sh HEAD` (read-only, the one gate outside the brief's machine-checked list): **clean**.
**M-4: closed** — `cmd/tax.rs:201-208` binds the `StaleNote` and `eprintln!`s it; `StaleNote: Display` (`input_form_store.rs:155-163`), and stderr keeps the stdout TOML clean for a redirect.
**N-1: closed** — `scrub.rs:959-990` now reads "THREE normalisations", enumerates them (Schedule C line A, Form 8995-A line 1(a), `excess_ss_not_creditable[].ein`), and agrees with the closing "Three members, each with a reason."
**N-2: closed** — the serializer failure is no longer `BadConfigValue` (`cmd/tax.rs:238-242`) and the `--out` write error names the path (`main.rs:330-335`). See Nit-2/Nit-3 on the variant chosen and the message text.

## IS THE SENTINEL STILL MAXIMAL?

**Yes.** Nothing became `None`, nothing became empty, and no `Vec` shrank. The five changes are all *value* changes inside still-present fields: `box3/box4 1000/62 → 6000/6000`, `box12` codes `SENTINEL_… → "D"/"DD"` (still two entries), `CharitableClass::Cash30 → Cash60` (×2, still two gifts each), `ira_deduction_claimed 3 → 0` (a `Decimal`, never in an `Option`), `foreign_trust Some(true) → Some(false)` (**still `Some`**). Every `Option` in the literal is `Some`, every `Vec` has two elements, and the literal is still `..`-free — `grep -E '\.\.'` over lines 122-345 matches only a comment.

Evidence for the derived path set, and it is machine-held rather than read:
- `the_derived_axis_reaches_every_field_the_review_passes_were_about` (`scrub_axis.rs:357-373`) asserts exactly `w2s[].ein`, `header.ip_pin`, `foreign_country_names`, `b_1099[].payer`, `schedule_c.business_description`. **The fold does not touch this test**, and it is in the passing suite.
- Stronger: matrix check (b) (`:901-907`) asserts every matrix row path is *in* the derived axis, and `matrix()` has **24 rows** (counted, not read: `grep -oE` over lines 556-766). So a shrunken fixture reds on 24 paths, not 5.
- The new baseline guard (`:880-884`) asserts `screen_inputs(base) == None`, which is what makes all 24 rows able to discriminate; the `w2s[].ein @ malformed` cell (`:693-697`, both W-2s → `"XX-1111111"`) now sits over the cap, so `over_cap_needs_ein` can fire on it.

One forward-looking note, not a defect: the fixture now withholds $6,000 SS on $6,000 of SS wages (a 100% rate). If a box4-vs-box3 consistency screen is ever added, the baseline assertion reds loudly — the failure direction is correct.

Second question from the brief: **`synthetic_malformed_ein` fails `canonical_ein` for every reachable `n`, but not for every `n`.** `{:06}` pads, it does not truncate, so at `n = 999_999` it emits `99-1000000` — nine digits, which **canonicalizes**, silently inverting the class it exists to preserve; and that same value equals `synthetic_ein(999_999)`, the only collision between the two ranges (for all smaller `n` the strings differ in length: `9d-` + 6 digits vs `9d-` + 7). Verified by replicating both formatters and `canonical_ein` in Python. Reaching it needs ~10^6 distinct EINs in one return, so it is unreachable; `synthetic_ein` has the mirror-image break at `n ≥ 9_999_999` and predates this fold.

## NEW DEFECTS INTRODUCED BY THE FOLD

None blocking. Eight non-blocking, all in text the fold authored:

- **[Minor] A false citation, in the sentinel's own comment.** `scrub_axis.rs:173-176` says box-12 KEPT-ness "is asserted directly by value in `the_surviving_sentinels_...`, not via a token" — but the same commit *removed* `"SENTINEL_box12_code"` from that test's scan list and its `expected` set (`:434-463`), and `"D"`/`"DD"` appear nowhere in it. Nothing asserts box-12 preservation by value anywhere (`grep box12` across `scrub.rs`, `scrub_axis.rs`, `scrub_refusal.rs`). No coverage was actually lost — if scrub began replacing the code, `replaced_paths` would gain `w2s[].box12[].code` and matrix check (a) (`:892-898`) reds — but the comment names an instrument that does not do this. Same edit left two stale statements behind it: the test's doc "The three are §7's recorded decisions" (`:423-425`) and its assert message "(relationship, box-12 code, NAICS)" (`:466-468`) against a two-element `expected`; and the fixture doc's "Every string is a distinct `SENTINEL_*` token" (`:118-120`) is no longer true of `box12[].code`. This is the N-1 defect class, reintroduced by the commit that closed N-1.
- **[Minor] M-2's fix is unheld.** Deleting `main.rs:326`'s `remove_file` reds nothing: `the_out_file_is_owner_only_even_when_it_already_exists` (`scrub_refusal.rs:447-478`) asserts the *final* mode, which is exactly the window the review said it cannot see. A kill-test is cheap — pre-create the file, record `std::os::unix::fs::MetadataExt::ino()`, and assert the inode changed (i.e. the file was created fresh, not written through).
- **[Nit] Two new strings carry collapsed multi-line indentation.** `cmd/tax.rs:240` emits `"…This is a btctax                  limitation…untouched.                  Please report it."` to a user; `scrub_axis.rs:883` has the same in an assert message ("every cell              of this matrix"). A `\`-continuation is missing.
- **[Nit] Wrong error variant, twice.** `CliError::Usage` is documented as "a CLI argument was malformed" and renders as `error: usage: …`. It is now used for a serializer limitation and for an I/O failure — and the I/O case regressed from `Io` (accurate, via `?`) to `Usage`, so a full disk or unwritable directory now tells the filer they typed the command wrong. Exit code is unaffected (every `CliError` maps to 2, `main.rs:39-46`). N-2 asked for the path in the message, which is delivered.
- **[Nit] Hand-counted number.** `scrub_axis.rs:871` says "all 23 rows"; `matrix()` returns **24**.
- **[Nit] The sentence that caused C-1 is still there.** `EinMap`'s doc (`scrub.rs:294-297`) still reads "Preserving the malformed *class* into the output is §3.2's job, not this map's" — the exact delegation the review identified as where the obligation was lost — now contradicted by the inline comment six lines below it ("It is handled here now… rather than delegated anywhere").
- **[Nit] `synthetic_malformed_ein` canonicalizes at `n ≥ 999_999`** (detail above). Unreachable; a `debug_assert!(canonical_ein(&s).is_none())` would make it structurally impossible.
- **[Nit] The stale-draft note slightly overstates.** It prints "discarded a stale draft for {year}"; `scrub_return_inputs` never calls `s.save()`, so nothing is persistently discarded on this path. The load-bearing half ("the draft you were editing was skipped") does land.

## WHAT WOULD MAKE THIS CHECK WRONG

If `the_derived_axis_reaches_every_field_the_review_passes_were_about` and matrix check (b) are not in fact green at `ef98b021` — I read them rather than ran them, and every maximality conclusion above rests on the brief's statement that the 2666-test suite passes.
