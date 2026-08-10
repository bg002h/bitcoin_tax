# Nit-fold check — `git diff 87a605e9..9be921e2`

_Independent Opus check, read-only, dispatched against `reviews/BRIEF-scrub-nit-fold-check.md`.
Persisted VERBATIM before any further fold._

---

VERDICT: needs-changes — 0 Critical / 0 Important (nothing here holds a gate).
         1 of 8 findings not closed, 1 closed-then-reintroduced, plus 1 Minor + 3 Nit
         authored by this fold.

**Minor-1 (false citation): closed.** `scrub_axis.rs:180-186` no longer cites the survival test; it names the derived axis, and the replacement claim is TRUE (traced below). The two stranded statements are fixed with it — `:433` now reads "Both are §7's recorded decisions" against the two-element `expected` at `:475`, and the assert message `:480-483` names "(relationship, NAICS)". **But the third stranded item — the fixture doc at `:118-123` — was rewritten into a new false claim (see NEW DEFECTS).**

**Minor-2 (inode kill-test): closed, with a soundness caveat.** `scrub_refusal.rs:459/488-492` records `ino()` before and after and asserts inequality; `open_owner_only` (`fsperms.rs:22-29`) opens the path directly with `create+write+truncate+mode(0o600)` — no temp-file-and-rename — so there is **no false-green path**: a written-through file keeps its inode and the assert fires. The caveat is the flake direction (Minor, below).

**Nit — two collapsed strings: closed, and a third introduced.** `cat -A` on `cmd/tax.rs:240-241` and `scrub_axis.rs:898-899` shows `\` continuations with no trailing byte, so the emitted literals are `"…limitation, not a problem…"` and `"…every cell of this matrix…"`. A grep of the diff's added lines for a mid-literal run of ≥4 spaces returns exactly one hit — the new one at `scrub_refusal.rs:491` (see NEW DEFECTS).

**Nit — wrong error variant: closed.** `CliError::ScrubOutput` (`lib.rs:167-175`) is used at exactly the two sites (`cmd/tax.rs:239`, `main.rs:331`) and nowhere else; nothing on the scrub path now reaches `Usage` or `BadConfigValue`. The doc's factual claims check out verbatim: `Usage` is `#[error("usage: {0}")]` / "A CLI argument was malformed" (`lib.rs:114-116`), `BadConfigValue` is "a `cli_config` row held an unrecognized value (corrupt DB…)" (`lib.rs:117-120`). No test or doc referenced the old message text (grep: zero hits). One accurate note, not a defect: `restrict_file_to_owner(&path)?` at `main.rs:336` still surfaces as `CliError::Store` — correct for a chmod failure, but it means the variant doc's "or could not write it" is not exhaustive over the `--out` path.

**Nit — hand-counted "23 rows": closed.** Rewritten to "every row compared" (`scrub_axis.rs:886`); `grep -E '(all )?2[34] rows'` over `scrub_axis.rs` returns nothing, so no count can go stale. (For the record, the row set is 24 and equals the derived axis exactly, since checks (a) and (b) at `:905-923` are mutual containment.)

**Nit — the sentence that caused C-1: closed.** `scrub.rs:303-313` no longer delegates ("…is §3.2's job, not this map's" is gone); it states the obligation is discharged in `EinMap`, which matches `EinMap::map` at `:318-341`. Consistent with, not contradicted by, the inline comment at `:320-328`.

**Nit — `synthetic_malformed_ein` canonicalizes: closed, with a regression traded in.** The class break is gone for *every* `usize` (see below), but distinctness was total before and is now periodic (see NEW DEFECTS).

**Nit — the stale-draft note overstates: NOT CLOSED.** No text changed. `StaleNote`'s `Display` (`input_form_store.rs:155-163`) still emits `"discarded a stale draft for {year} (schema v{found}, expected v{expected})"`, and `cmd/tax.rs:206-208` still `eprintln!`s it unchanged; `input_form_store.rs` is not in the diff's six files. The commit message lists this nit in its "Nits, each real:" block, restating the finding but naming no edit — so the message reads as if it were addressed.

---

**IS THE NEW BOX-12 CLAIM TRUE? — Yes**, end to end.
1. `Box12Entry.code` is a plain `pub code: String` with no serde rename, inside `W2.box12: Vec<Box12Entry>` (`return_inputs.rs:29-32`, `:76`), inside `ReturnInputs.w2s`.
2. `diff_paths` (`scrub_axis.rs:50-91`) collapses each array to one `[]` segment and recurses, so a differing code yields exactly the string `w2s[].box12[].code`.
3. The sentinel instantiates it non-trim-empty with two distinct values, `"D"` and `"DD"` (`:187-196`), so any actual replacement — including one that normalised every code to a single value — produces a difference on at least one element.
4. `replaced_paths` therefore gains that path, and check (a) (`:905-914`) asserts `rows.contains(path)` for every derived path; `matrix()` has no such row, so the `assert!` fires and `every_replaced_field_preserves_its_class_in_every_representable_state` reds.
5. That test is untouched by this diff and is in the passing suite.

Boundary the comment does not state, and I am reporting as a caveat rather than a finding: the guard is conditional on the fixture keeping a populated `box12`. If someone set `box12: vec![]`, the path leaves the derived axis and nothing reds — the `..`-free literal forces field *presence*, not `Vec` non-emptiness, and no test pins the sentinel's element counts.

**IS THE INODE ASSERTION SOUND? — Directionally yes; not a proof, and it can flake red.** It has no false-green path (point 2 of Minor-2 above). But POSIX explicitly permits reusing an inode number once the last link is dropped, and ext4 does so eagerly: `__ext4_new_inode` picks the *lowest free bit* in the parent directory's block group, so an inode freed by `remove_file` and immediately re-created in the same directory can come back with the same number — which fires `assert_ne!` on a **correct** fix. It cannot flake on tmpfs (monotonic `get_next_ino`/`shmem` counter — this machine's `/tmp` is tmpfs, which is why the local 2666 are green) or on APFS (monotonic file IDs, so the macOS leg is safe), but GitHub-hosted `ubuntu-latest` has `/tmp` on the ext4 root filesystem and that is the leg `cargo test --workspace` runs on (`ci.yml:22-24,38`). Fix is two lines and makes it airtight: `fs::hard_link(&out, dir.path().join("pin"))` before the run pins the old inode so it cannot be freed *or* reused, and `pin` then also witnesses the property directly — stale bytes + 0644 if the fix holds, the scrubbed return at 0600 if it does not.

**DOES `synthetic_malformed_ein` STILL PRODUCE DISTINCT VALUES? — For every reachable `n`, yes; in general, no — this is a regression.** Computed, not reasoned: the new formula is class-correct for *all* `n` (always exactly 8 digits, so `canonical_ein` rejects; the old `n = 999_999` / `n = 1_000_000` inversions are gone, as is the old cross-range collision `mal(999_999) == synthetic_ein(999_999) == "99-1000000"`). But `f(n) = (n mod 10, (n+1) mod 10^6)` has period 10^6: **`f(0) == f(1_000_000) == "90-000001"`**, first collision at exactly those values. The old formula was injective for every `n`. Reachability is unchanged in order of magnitude — `EinMap` passes `n = self.0.len()`, so it needs a million distinct EINs on one return — and merging two *malformed* EINs cannot move a §6413(c) figure (both stand-ins stay unreadable, so `over_cap_needs_ein` still refuses). So the trade is defensible; it is just not the trade the comment describes.

On `debug_assert!` being compiled out: it does not weaken the class guarantee, because the `% 1_000_000` enforces that unconditionally in every profile. The sharper point is the opposite one — the modulo makes the assert's condition a **tautology** for every `usize`, so it can never be observed red (B1) and it is not what makes the guarantee structural; and it says nothing about distinctness, which is the property that actually changed.

---

**NEW DEFECTS:**

1. **[Minor] The inode kill-test can red spuriously on ext4.** `crates/btctax-cli/tests/scrub_refusal.rs:459,488-492`. Vector: CI's `ubuntu-latest` leg, `/tmp` on ext4; the freed inode is the lowest free bit in its block group and is handed straight back to `write_owner_only`'s `O_CREAT`, so `ino_before == ino_after` on a correct fix. Never a false green. Fix: pin the old inode with a hard link before the run (above).

2. **[Nit] A third collapsed multi-line string, in the fold that fixed the other two.** `crates/btctax-cli/tests/scrub_refusal.rs:491` — the emitted literal is `"…and narrowed          afterwards; …"` (10 literal spaces, counted). Test-only, so it reaches a maintainer rather than a filer, but it is the same missing `\` and the same class.

3. **[Nit] A new false claim in the fixture doc — about the SSNs.** `crates/btctax-core/src/tax/scrub_axis.rs:118-123`. Two errors and an omission: (a) the SSNs are **not KEPT** — `header.taxpayer.ssn`, `header.spouse.ssn` and `header.dependents[].ssn` all carry matrix rows, and check (b) at `:917-923` proves every row is in the derived axis, i.e. scrub replaces them (`class_preserving_stand_in`, `scrub.rs:358-378`); (b) "Gibberish in either REFUSES, and `screen_inputs` returns the FIRST refusal" is true for box-12 but **false for an SSN** — `return_refuse.rs:693-699` reads, verbatim, *"★★ NO SSN GATE HERE, DELIBERATELY"*, so a malformed SSN surfaces at `ReturnHeader::build`, a different instrument checked by `assert_same_header_error`; (c) the rule it introduces — "every string SCRUB REPLACES carries a `SENTINEL_*` token" — has five exceptions, not two: the three SSNs plus `header.ip_pin` (`"654321"`) and `w2s[].ein` (`"11-1111111"`/`"22-2222222"`), and the parenthetical names neither of the latter. Nothing rides on the sentence — `no_fixture_value_collides_with_a_stand_in` (`:491-520`) holds the actual precondition for the address/name/payer stand-ins — but this is a comment naming a mechanism that does not do what is claimed, i.e. Minor-1's class, one paragraph above Minor-1's fix. *(Pre-existing and out of scope, but it is the evidence: the survival scan list at `:448-470` still contains `SENTINEL_ssn`, `SENTINEL_dependent_ssn` and `SENTINEL_ippin`, none of which exist anywhere in the fixture.)*

4. **[Nit] One statement stranded behind the `% 1_000_000` edit.** `crates/btctax-core/src/tax/scrub.rs:239` — the doc line directly above still reads *"A synthetic EIN that **`canonical_ein` REJECTS**, distinct per `n` — eight digits, one short."* The edit made the first half unconditionally true and the second half true only for `n < 10^6`, and the added comment and `debug_assert!` both speak to the class, never to distinctness.

**WHAT WOULD MAKE THIS CHECK WRONG:** if `every_replaced_field_preserves_its_class_in_every_representable_state` is not actually green at `9be921e2` — I read checks (a) and (b) rather than running them, and both the box-12 verdict and the "24 rows == derived axis" claim rest on the brief's statement that the 2666-test suite passes.
