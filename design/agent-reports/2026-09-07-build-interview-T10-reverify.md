# Re-verification — interview T10 build + fold, at `eefc50f8`

Independent verifier, own worktree `/scratch/code/bitcoin_tax/.claude/worktrees/agent-a608e7f065e207dcb`,
`main` @ `eefc50f8` (the T10 fold plus the FR-90 follow-up commit). `export
CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before every cargo command; scoped runs only.
Every plant made here and reverted (via `cp` backup, restored with `cp` back — never `git checkout --`);
`git status --porcelain` empty at every checkpoint below; nothing committed, nothing pushed, no
subagents.

**FR-90 discipline followed throughout.** After every plant AND every restore, the touched file was
`touch`-ed before running anything; before the two full-scope confirming runs, `find crates -name
'*.rs' -exec touch {} +` was run first. No surprise red was observed at any point; nothing needed a
re-run after a touch.

**Baseline** (before any plant, full touch): `cargo nextest run --locked -p btctax-core -p
btctax-input-form -p btctax-forms -p btctax-cli` → **2616 tests run: 2616 passed, 5 skipped**. Final
confirming run (after every plant reverted, full touch): **byte-identical — 2616 passed, 5 skipped.**

---

## Part 1 — the build report's kills (P1–P21, plus P6b), all replanted today

Every plant reproduces the report's row **verbatim or with an explained, non-substantive drift** (line
numbers shift naturally as the fold added code; one message differs only because my first P6 plant hit
a shared helper before I re-targeted the exact closure the report names — corrected below and then
byte-matched).

| # | plant | test/command | RED text observed | verdict |
|---|---|---|---|---|
| P1 | `RoutingNumber::canonical`'s prefix check → `if false` | `the_routing_prefix_rule_is_the_instructions_two_ranges_and_their_edges` | `assertion left == right failed: prefix 00 is outside "01 through 12 or 21 through 32": 000000000` | PASS (byte-match) |
| P2 | ABA check-digit branch → `if false` | `t10_bank_numbers` (5 tests) | 2 of 5 fail: `"the IRS's own sample-check number carries a wrong check digit — that is the point of a sample"` and `"250250025" must fail the print boundary, not print` | PASS (byte-match) |
| P3 | advisory guard `if refund > Usd::ZERO && …is_none()` → drop the `is_none()` conjunct | `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` | `a filer who gave routing and account numbers must NOT be told the IRS will mail a check` | PASS (same defect class; report's exact old panic line moved, message equivalent) |
| P4 | same guard → drop the `refund > Usd::ZERO` conjunct | same | `assertion failed: !fires(Usd::ZERO, None)` | PASS (byte-match) |
| P5 | `"FOREIGN ADDRESS"` restored into `UnmodeledReturnOptionsOmitted`'s message | `the_advisory_does_not_name_a_cell_t10_now_fills` | `the advisory still names "FOREIGN ADDRESS", which T10 collects and the emitter prints: …` | PASS (byte-match) |
| P6 | `SpIpPin`'s `get` closure → constructs via `SecretView::set_masked(raw)` instead of the mask | `every_secret_field_is_asymmetric_written_never_read_back` | `SecretView::set_masked was given a string with a 5+ digit run (a raw secret?): "987654321"` | PASS (byte-match, once planted at the exact closure the report names — see note below) |
| P6b | same, but `SecretView::Set { masked: raw }` direct construction (bypasses the guard) | same | `SpIpPin: the secret view returned the whole value: Set { masked: "987654321" }` | PASS (byte-match) |
| P7 | `mask_pii`'s spouse-PIN branch → `if false` | `income_show_redacts_the_spouse_ip_pin_and_the_bank_numbers` | `assertion left == right failed  left: Some("654321")  right: Some("***")` | PASS (byte-match) |
| P8 | `foreign_address_is_live()` → `true` (hardcoded) in `ReturnHeader::build` | `the_foreign_address_prints_only_under_a_country` | `the foreign province must not print without a country — half an address is not an address  left: Some("Mud Province")  right: None` | PASS (byte-match) |
| P9 | map's 35c checking/savings FQNs + on-states transposed | `the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box` | `Checking: line 35c's own box, at its own measured on-state  left: None  right: Some("1")` | PASS (byte-match) |
| P10 | opener `seed` gains explicit carries for `spouse_ip_pin` and `direct_deposit` | `the_trailer_splits_identity_from_the_per_year_credentials` | `an IP PIN is issued per year — carrying December's would print a void credential  left: Some("654321")  right: None` | PASS (byte-match) |
| P11 | opener's four identity `clone()`s (`phone`, `foreign_country/province/postal_code`) deleted | same | `assertion left == right failed  left: ""  right: "555-0100"` | PASS (byte-match) |
| P12 | `direct_deposit` census helper dropped from the `named` union in the source-census test | `every_param_free_rule_is_censused_from_the_source_and_fires_on_both_paths` | `the fixture table and the source census must name the same param-free rules` — left carries `"DirectDepositNumberMalformed"`, right does not | PASS (byte-match) |
| P13 | `screen_direct_deposit(ri)` call removed from `screen_inputs_tiered` | same | `the commit gate must reach DirectDepositNumberMalformed on its own fixture  left: None  right: Some("DirectDepositNumberMalformed")` | PASS (byte-match) |
| P14 | `scrub_routing`'s `Ok` arm returns the real number | `scrub_axis::matrix::every_replaced_field_preserves_its_class_in_every_representable_state` | `§3.3's matrix has a row for header.direct_deposit.routing, which is NOT in the derived axis …` | PASS (byte-match) |
| P15 | `scrub_routing`'s `BadCheckDigit` arm returns a valid stand-in | same | `header.direct_deposit.routing @ malformed: scrubbing changed WHETHER (or why) the return refuses. … left: Some(DirectDepositNumberMalformed { cell: Routing, why: BadCheckDigit })  right: None` | PASS (byte-match) |
| P16 | map's `routing` FQN repointed at the 35d account widget | `the_direct_deposit_block_prints_the_filers_numbers_and_exactly_one_type_box` | `Checking: line 35b must carry the nine bare digits  left: None  right: Some("123456780")` | PASS (byte-match) |
| P17 | `#[serde(default)]` added to `DirectDeposit::routing` | `the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named` | `a direct-deposit block with no routing number must not parse — it is a mistyped row, not a lawful state` | PASS (byte-match) |
| P18 | emitter's spouse-IP-PIN write (3 lines) deleted | `the_spouse_ip_pin_prints_and_never_without_a_spouse` | `the spouse's IP PIN cell (f2_36) must carry the six digits  left: None  right: Some("654321")` | PASS (byte-match) |
| P19 | emitter's phone write deleted | `the_phone_number_prints_in_the_signature_block_and_nowhere_else` | `assertion left == right failed  left: None  right: Some("555-0100")` | PASS (byte-match) |
| P20 | `push_direct_deposit` fabricates a block (`RoutingNumber::canonical("123456780")`, Checking, fake account) when `header.direct_deposit` is `None` | `no_direct_deposit_leaves_the_whole_refund_block_blank` | `35b routing untouched  left: Some("123456780")  right: None` | PASS (byte-match) |
| P21 | map's `phone` cell repointed at `f2_38` (still censused as `unmodeled`) | `field_census::census_accounts_for_every_field` | `2024/f1040: ["…f2_38[0]"] are BOTH mapped and censused — a field cannot be one we fill and one we deliberately leave blank` | PASS (byte-match) |

**Note on P6.** My first attempt edited the shared `mask_ip_pin` helper (used by both `IpPin` and
`SpIpPin`), which caught on the taxpayer's `IpPin` field first (earlier in the sweep) and printed
`"IpPin: …"` rather than `"SpIpPin: …"`. Re-planted at the exact `SpIpPin` `get` closure the build
report names (`sections.rs:472`, `.map_or(SecretView::Empty, mask_ip_pin)` → an inline closure calling
`set_masked`/`Set` directly) and both P6 and P6b then byte-matched the report's cited text. Not a
finding — an artifact of my plant location choice, not the checker's.

---

## Part 2 — the fold's own kills, replanted today

### I-1 (four kills)

| kill | plant | command | RED / observed | verdict |
|---|---|---|---|---|
| K-1 | delete the `"your foreign address, if you have one"` row from `CARRIED_IDENTITY` | `every_leaf_the_seed_carries_is_named_in_the_report` | `` `header.foreign_country` is covered by "foreign address", which the report does not print: [...] `` | PASS (byte-match) |
| K-2 (class, derived fixture) | new carry planted in `seed`: `spouse_ip_pin: prior.header.spouse_ip_pin.clone()`, no phrase names it | same, with the current (post-fold) derived-fixture guard | `the seed writes \`header.spouse_ip_pin\`, and no phrase in the opener's report names it …` | PASS (byte-match) |
| K-2 (class, hand fixture + floor neutralised) | **identical plant left in place**; guard's loop reduced to only the pre-fold hand-written fixture (`a_year_with_money_in_it` + HoH + `spouse=None`, reconstructed from `git show f61a7ebc:…`); floor assertion neutralised (`true \|\| unreached.is_empty()`) | same | `PASS [0.151s] … 1 passed` — **green with the identical defect present** | PASS — reproduces the FR-88 pattern exactly: same defect, same checker, red with the derived fixture, green with the hand-written one |
| K-3 (the floor) | maximal-sentinel fixture dropped from the loop, floor left live | same | `` `seed` writes 181 leaves that NO fixture above reached, so the walk could never have asked whether the report names them … `` | PASS (message shape byte-matches; count is 181 here vs. 170 cited in the fold report — expected drift, the leaf set has grown since the fold as the codebase moved forward; not a defect) |
| K-4 | `cli.rs`'s long-help doc comment reverted to the pre-fold (`f61a7ebc`) wording (no phone/foreign-address sentences) | `cli::tests::the_shipped_help_names_everything_the_opener_carries` | `` `btctax income open-next-year --help` must say "your phone number" — the opener carries "your phone number" … `` | PASS (byte-match, including the full printed `--help` text quoted in the panic) |

### I-2 (two kills)

| kill | plant | command | RED / observed | verdict |
|---|---|---|---|---|
| kind-default | `create`'s `kind: None` → `kind: Some(DepositAccountKind::Checking)` | `the_account_type_is_unanswered_until_the_filer_chooses_it_and_refuses_until_then` | `a type nobody chose must read as unanswered … left: Some(Choice("Checking"))  right: None` | PASS (byte-match) |
| Kind leg inert | `screen_direct_deposit`'s `if dd.kind.is_none() { … }` → `if false { … }` | same, and `the_trailer_round_trips_through_the_toml_wire_and_a_misspelt_key_is_named` | `an unchosen account type must refuse` / `an imported block with no account type must refuse` | PASS on both (byte-match) — confirms the fold's own claim that this one plant reds two crates |

### I-3 (two kills)

| kill | plant | command | RED / observed | verdict |
|---|---|---|---|---|
| advisory unconditional | drop the `is_none()` conjunct in the `RefundByPaperCheck` guard | `a_malformed_routing_number_blocks_the_return_and_the_paper_check_notice_stays_silent` | `RefundByPaperCheck is guarded on the block being ABSENT — a malformed block silences it, so the filer is NOT told a check is coming` | PASS (byte-match) |
| ABA check inert | ABA check-digit branch → `if false` | same | `a bad check digit must BLOCK the return — this rule is not a warning  left: None  right: Some(DirectDepositNumberMalformed { cell: Routing, why: BadCheckDigit })` | PASS (byte-match) |

### M-1, M-2, M-3

| finding | plant | command | RED / observed | verdict |
|---|---|---|---|---|
| M-1 | the `(None, Some(_))` refusal arm in `form1040_full.rs`'s exhaustive match replaced with `{}` (the pre-fold silent skip restored) | `a_map_with_no_direct_deposit_block_refuses_a_return_that_has_one` | `dropping the filer's bank details must not be silent — the fill returned Ok on a map with no [direct_deposit] and a return that carries one` | PASS (byte-match) |
| M-2 | the retracted `"v1 never fills"` sentence restored to `RefundByPaperCheck`'s doc comment | `the_paper_check_advisory_is_silent_exactly_when_a_deposit_is_given` (its source-scan half) | `` `advisories.rs` still claims "v1 never fills" somewhere … `` | PASS (byte-match) |
| M-3 | `AddrForeignCountry`'s `set` — the clear-province/postal-code-on-empty block removed | `clearing_the_foreign_country_clears_the_row_so_a_later_country_cannot_resurrect_it` | `clearing the country must clear the row, not leave it dead but at rest  left: ("Mud Province", "XY1 2AB")  right: ("", "")` | PASS (byte-match) |

**N-1** carries no test by design (a wording-only fix); confirmed by reading `sections.rs`'s `AddrPhone`
help and `scrub.rs`'s phone note — both now read *"no figure on the return is computed from it; it
prints in the signature block"* (`AddrPhone`) / the parallel phrasing in `scrub.rs`. Doc-only, matches
the fold's description. No kill claimed, none owed.

**M-4** is filed as FR-89 (secret handling, logged, never gating per the 2026-08-27 owner ruling) — not
a kill claim, confirmed present in `FOLLOWUPS.md`.

---

## Part 3 — negative-claim checks

1. **Five instruments re-run, byte-identical to the build/fold reports and the controller's ledger:**
   - `line-coverage`: `375 money lines across 18 form(s) […], 31 exception(s) (ratchet 31), 0 unverifiable (ratchet 0), 17 not line-bound (ratchet 17)` ✓
   - `census-join`: `274 unmodeled entries across 13 maps, …` ✓
   - `stop-list`: `8 btctax-input-form sources, 4 state-bearing sources and 91 registry prompts scanned; no forbidden shape` ✓
   - `box-census`: `268 printed boxes across 19 archived editions of 9 information returns, every one decided (268 entries)` ✓
   - `field_census` suite (5 tests incl. `census_accounts_for_every_field`, `the_uncensused_register_may_only_shrink`): **all 5 PASS**
2. **`UNCENSUSED` register:** `UNCENSUSED_ENTRIES = 5`, `UNCENSUSED_FIELDS = 285`, and all five register
   rows (`f1040`, `f8283`, `f8949`, `schedule_d`, `schedule_se`) are TY2025 — confirmed by reading
   `crates/btctax-forms/tests/field_census.rs:75-102`. Matches the review's claim exactly.
3. **No emitted artifact carries a fixture's spouse-PIN digits (`654321`) or the raw secret-probe value
   (`987654321`).** `git grep -n "654321"` over the committed tree: every hit outside `.rs` test source
   is in `design/`/`reviews/` prose (documenting the fixture, not an emitted artifact) or is the
   unrelated EIN `98-7654321` in `docs/examples/examples.md` / the TUI-walkthrough docs (donation
   examples, not T10). `crates/btctax-core/tests/goldens/` — **zero hits** for `654321`,
   `987654321`, `spouse_ip_pin` or `direct_deposit`. `docs/examples/examples.md`'s T10 diff (verified
   with `git diff f8768e93~1 f8768e93 -- docs/examples/examples.md`) is exactly the six new `income
   show` keys (`spouse_ip_pin: null`, `phone: ""`, `foreign_country/province/postal_code: ""`,
   `direct_deposit: null` — **all blank**) plus the two advisory-text rewrites (SEVEN→FIVE, and the
   REFUND BY PAPER CHECK wording) — no tax figure changed, matching the build report's claim verbatim.
   The fold (`f61a7ebc`→`08c09ecc`) made **no further change** to `docs/examples/examples.md`.
4. **`bash scripts/pii-scan-generic.sh eefc50f8` → `pii-scan: clean (eefc50f8).` exit 0.** The script's
   shapes are SSN-like (3-2-4) and EIN-like (2-7) hyphenated digit groups; a bare 9-digit routing number
   is not that shape, consistent with the brief's note. The T10 sentinels used where a shaped identifier
   *is* at stake (`123456780`, `111111118`, `250250025` as routing numbers; `SENTINEL-ACCT-1` as an
   account) are documented in the build report §5 with their derivation (sequential/repeated-digit ABA
   numbers, the IRS's own sample-check number) — not arbitrary.
5. **The other non-`.rs` diffs across the whole T10→fold→FR-90 arc** (`git diff f8768e93~1 eefc50f8
   --stat`): the three TUI-walkthrough golden `.txt` files (confirmed to show exactly D-10's `✓`→`…`
   glyph change on Address plus the new `… Direct deposit` row — no tax figures) and
   `docs/man/btctax-income-open-next-year.1` (confirmed to show exactly K-4's man-page regeneration,
   byte-matching the corrected doc comment). No golden PDF, no golden JSON, no snapshot outside these
   changed.

---

## Environment

Baseline and final confirming runs both used the four-crate scope (`btctax-core`, `btctax-input-form`,
`btctax-forms`, `btctax-cli`): **2616 tests run, 2616 passed, 5 skipped**, byte-identical before and
after the whole plant campaign. `test(form_delta)` and
`the_write_hook_denies_new_archives_and_asks_once_per_new_directory` matched **zero** tests under these
filters in this worktree (not present under those names/patterns at this commit) — not encountered as
failures in the full scoped run either, so the brief's PDF-less-worktree caveat did not apply here and
no emitter claim is marked unverified. `crates/btctax-forms/forms/2024/f1040.pdf` and
`forms/2025/f1040.pdf` are both present; every emitter kill (P8, P9, P16, P18–P21, M-1) drove the real
bundled template.

---

## Counts

Every kill claimed by the build report and the fold report reproduces its red today, byte-identical or
with an explained, non-substantive drift (a plant-location artifact on P6, corrected; a leaf-count
drift on K-3 attributable to codebase growth since the fold, not a defect). Every review finding (I-1,
I-2, I-3, M-1, M-2, M-3, N-1) has a test that holds it, confirmed by planting its removal. K-2's
two-fixture demonstration reproduces the FR-88 pattern exactly as described: identical defect, same
checker, red with the derived fixture and green with the hand-written one. All five instruments and the
field-census suite are byte-identical to their pinned values. No emitted artifact carries a fixture
secret. `pii-scan-generic.sh` is clean. No tax figure moved anywhere in the T10→fold arc.

**Counts: C=0 I=0 M=0 N=0**
