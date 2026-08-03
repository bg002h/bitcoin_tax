# `income scrub` — pre-publish adversarial review (r1)

27 agents · 4 independent lenses · every finding adversarially verified (refute-by-default) · plus a completeness critic.
**22 raw findings → 19 survived verification**, plus 6 from the sweep.

Run: `wf_89bc6efa-da2`. Brief: `reviews/scrub-r1-BRIEF.md`. Persisted verbatim before folding.

---

## Confirmed findings

### [CRITICAL] `EinMap` keys on the RAW EIN string while §6413(c) compares CANONICAL EINs, so scrubbing splits one employer into two and manufactures an excess-Social-Security credit

**Location:** crates/btctax-core/src/tax/scrub.rs:63-75 (EinMap::map keys `real.to_string()`) and :258-264; vs crates/btctax-core/src/tax/return_1040.rs:681-687 (`canonical_ein`) and :782-811

**Failure:** The repo's own pinned fixture is the vector. `return_1040.rs:6022-6034` (`two_spellings`): ONE employer, a W-2 and its W-2c, Owner::Taxpayer, box 4 = $6,000 each ($12,000 > the $10,453.20 cap), EIN typed `"11-1111111"` on one row and `"111111111"` on the other. Original: `canonical_ein` collapses both to `111111111`, `eins.len() == 1`, credit = $0 — the test asserts exactly that, and `excess_ss_not_creditable` lists the $1,546.80 as stranded. After `scrub_pii`: the two RAW strings are distinct BTreeMap keys, so they become `"90-0000001"` and `"91-0000002"`, which canonicalize to `900000001` and `910000002` — TWO employers. `excess_social_security` then returns min(6000,cap)+min(6000,cap) − cap = **$1,546.80**, `screen_inputs` no longer refuses (both canonicalize), `total_payments` rises $1,546.80 (return_1040.rs:1887-1905), the refund rises by the same, and the `ExcessSsSingleEmployerNotCreditable` advisory disappears. The scrubbed copy is a DIFFERENT, tax-understating return that `income import` accepts — and this is precisely the failure the module's own EIN paragraph (scrub.rs:39-44) says it exists to prevent ("Replacing them independently would silently turn one employer into two and manufacture a credit"), at the same dollar figure a prior Fable review found. No test catches it: the three fixtures in `scrub_preserves_every_computed_figure` carry no EIN at all, and `ein_distinctness_is_preserved_exactly` uses two IDENTICAL spellings.

**Fix:** Key `EinMap` on `canonical_ein(real)` (falling back to the raw string only when it fails to canonicalize) so employers that are one employer stay one; and emit a synthetic that ALSO fails `canonical_ein` (e.g. 8 digits) for an input that failed it, so `ExcessSsEmployerUnknown` still fires. Then add a two-spelling W-2 pair to `ein_distinctness_is_preserved_exactly` and assert the count of DISTINCT `canonical_ein`s is preserved — it reds today.

<details><summary>Verifier reasoning</summary>

CONFIRMED, and empirically reproduced. Every link verified against source, then executed.

SOURCE CHAIN (all citations checked):
1. /scratch/code/bitcoin_tax/crates/btctax-core/src/tax/scrub.rs:71 — `self.0.entry(real.to_string())`. EinMap keys the RAW spelling.
2. crates/btctax-core/src/tax/return_1040.rs:681-686 — `canonical_ein` strips hyphens/whitespace, so "11-1111111" and "111111111" are ONE employer.
3. scrub.rs:59-61 — `synthetic_ein(n) = format!("9{}-{:07}", n % 10, n + 1)`; n=0 -> "90-0000001", n=1 -> "91-0000002", which canonicalize to two DISTINCT nine-digit ids.
4. return_1040.rs:783-811 — `excess_social_security` gates on `eins.len() < 2` after canonicalizing, then sums `per_employer.min(max)`.
5. return_1040.rs:6022-6035 — the repo's own pinned `two_spellings` fixture: one employer, box 4 = $6,000 twice, asserts the credit is $0.
6. return_1040.rs:1886-1905 — `excess_social_security` feeds `total_payments` (L33) -> `overpayment_refund`/`amount_owed`.
7. return_1040.rs:714-736 — `non_creditable_ss` also buckets by canonical EIN, so the "$1,546.80 stranded, ask your employer" advisory vanishes after the split.

EMPIRICAL PROOF (patched copy at /scratch/.verify-ein-probe, outside the repo; repo untouched; dir since removed):
  PROBE orig eins    : [Some("11-1111111"), Some("111111111")]
  PROBE scrubbed eins: [Some("90-0000001"), Some("91-0000002")]
  PROBE orig  excess_ss = 0        PROBE scrub excess_ss = 1546.800
  PROBE orig  stranded  = [NonCreditableSs { owner: Taxpayer, ein: "111111111", amount: 1546.800 }]
  PROBE scrub stranded  = []
  assertion failed: SCRUB MOVED THE CREDIT  left: 0  right: 1546.800
The three existing tax::scrub tests passed in the same run — the suite does not see this.

WHY THE EXISTING TESTS CANNOT CATCH IT (verified): `grep -n "ein:" crates/btctax-core/src/tax/testonly.rs` returns NOTHING — no fixture used by `scrub_preserves_every_computed_figure` carries an EIN at all. And `ein_distinctness_is_preserved_exactly` (scrub.rs:338-365) uses two IDENTICAL spellings "11-1111111"/"11-1111111", so it exercises the raw-string map on a case where raw equality and canonical equality coincide. This is a textbook B1 violation: the instrument was never watched discriminating.

REACHABILITY — NOT theoretical. `ReturnInputs::W2::ein` is a bare `#[serde(default)] Option<String>` (return_inputs.rs:57-58) and `grep -rn canonical_ein crates/` shows only THREE call sites, all read-side (return_1040.rs and return_refuse.rs) — nothing normalizes an EIN at import, in the CLI, or in the TUI. The field's own doc comment names the exact real-world cause ("one employer may issue several W-2s to one person — a corrected W-2, a mid-year payroll-system change, separate establishments under one EIN") and states the hazard verbatim ("two spellings of one employer are two employers to a string compare"). This is the v0.15.0 understatement bug the canonicalization fix exists to kill; `scrub_pii` reintroduces it downstream of the fix.

HARM IS ELEVATED BY THE --help TEXT (crates/btctax-cli/src/cli.rs:413-427), which makes two promises that are false: "W-2 EINs are replaced but keep their SAMENESS, because §6413(c)'s excess-social-security credit turns on having more than one employer", and "The output is TOML that `income import` accepts, and it computes an IDENTICAL return — a guarantee held by a test, not a hope." The emitter (crates/btctax-cli/src/cmd/tax.rs:164-183) deliberately emits re-importable TOML. So the scrubbed copy both (a) sends a recipient after a phantom $1,546.80 credit the filer's real return does not have, or conversely masks the real behavior, and (b) if re-imported — which the help explicitly invites — produces a return understating tax by $1,546.80 signed under §6065, the one direction this codebase promises never to go. The module's own headline guarantee (scrub.rs:6-10, "Scrubbing must not change a single computed number") is provably false, and its own EIN paragraph (scrub.rs:39-44) describes precisely this failure as the thing EinMap prevents.

MINOR IMPRECISIONS IN THE FILING (not disqualifying): (a) "screen_inputs no longer refuses" is misleading — `screen_inputs` refuses in NEITHER case, since both spellings canonicalize fine (return_refuse.rs:932-955); the parenthetical "(both canonicalize)" states the real mechanism. (b) "the same dollar figure a prior Fable review found" — that review's figure was $1,946.80 at $6,200/W-2; the pinned fixture's is $1,546.80. (c) the `total_payments` cite is 1886-1905, not 1887-1905. None of these touch the substance.

ADJACENT INSTANCE OF THE SAME ROOT CAUSE (the fix must cover it): scrub.rs:261 replaces any non-empty EIN, including a MALFORMED one. `Some("banana")` -> `Some("90-0000001")`, which canonicalizes successfully — so the original REFUSES with `ExcessSsEmployerUnknown` (return_refuse.rs:938-951, over cap + any EIN canonicalizing to None) and the scrubbed copy FILES. That is the brief's "file where the original refused" direction, from the same defect: EinMap ignores canonical identity and validity.

FIX: key EinMap on the canonical identity, not the spelling — e.g. `canonical_ein(real)` as the map key, falling back to the raw string only when it does not canonicalize, and leave a non-canonicalizable EIN un-replaced (or replace it with an equally-malformed stand-in) so the refusal survives. Land it paired with a red-first test using TWO SPELLINGS of one EIN over the SS cap, asserting `assemble_absolute` equality (which reds today), plus one asserting a malformed EIN still refuses after scrubbing.

SEVERITY: CRITICAL stands under the repo's own rubric (/scratch/code/CLAUDE.md: "wrong result / ... / an unmet guarantee"). The brief ranks DISTORTION second to DISCLOSURE, but this is a computed-figure change in the understating direction, on a permanently-published surface, that falsifies both the module's stated guarantee and the user-facing --help promise.

</details>


### [CRITICAL] `EinMap` keys on the RAW EIN string while §6413(c) is decided on the CANONICAL one, so scrubbing splits one employer into two and manufactures a $1,946.80 excess-SS credit

**Location:** crates/btctax-core/src/tax/scrub.rs:68-74 (EinMap::map -> self.0.entry(real.to_string())) and :258-264; vs crates/btctax-core/src/tax/return_1040.rs:681-687 (canonical_ein), :782-797 (excess_social_security), :714-736 (non_creditable_ss), and crates/btctax-core/src/tax/return_refuse.rs:931-957 (ExcessSsEmployerUnknown)

**Failure:** The scrub map is injective on the raw string, but the computation partitions employers by canonical_ein (which strips '-' and whitespace). The scrubbed partition can therefore only ever be FINER than the real one: it never merges employers, it splits them. Two confirmed failures, both reproduced end-to-end by running assemble_absolute / screen_inputs on both copies. (A) A FIGURE MOVES: one employer, a W-2 and its W-2c, box b typed '11-1111111' off the paper form and '111111111' off a payroll export, $6,200 box 4 each. Original: 1 canonical EIN => Schedule 3 line 11 = $0, amount owed $6,339, plus the excess_ss_not_creditable advisory naming the employer to ask. Scrubbed: raw keys differ => '90-0000001' and '91-0000002', both canonical => line 11 = $1,946.80, amount owed $4,392.20, advisory gone. assemble_absolute(original) != assemble_absolute(scrubbed). This is precisely the $1,946.80 understatement that return_1040.rs:772-781 documents a Fable review having killed; the scrub silently re-creates it inside a file the --help tells the user 'computes an IDENTICAL return'. (B) A REFUSAL VANISHES: a malformed or whitespace EIN ('12-345678', '   ') is non-empty, so it passes the filter(|e| !e.is_empty()) at scrub.rs:261 and is remapped to a WELL-FORMED synthetic. Over the §3101(a) cap the original screens Some(ExcessSsEmployerUnknown); the scrubbed copy screens None and files. The module doc at scrub.rs:39-44 states the governing rule -- 'Replacing them independently would silently turn one employer into two and manufacture a credit' -- and the code violates it. ein_distinctness_is_preserved_exactly (scrub.rs:338-365) cannot red on this: its 'SAME employer' pair is two IDENTICAL strings.

**Fix:** Key EinMap on canonical_ein(real) rather than the raw string, and pass through (or map to a deliberately non-canonical synthetic) any EIN that fails to canonicalize, so both the employer partition and its undecidability survive. Then extend ein_distinctness_is_preserved_exactly with the two_spellings vector that already exists in-repo at return_1040.rs:6022-6035, plus a malformed-EIN case asserting the ExcessSsEmployerUnknown refusal still fires on the scrubbed copy.

<details><summary>Verifier reasoning</summary>

CONFIRMED against source, both halves.

MECHANISM: `EinMap::map` (scrub.rs:68-74) keys `self.0.entry(real.to_string())` — the RAW string. §6413(c) is decided on `canonical_ein` (return_1040.rs:681-687), which strips '-' and whitespace and requires exactly nine digits. `synthetic_ein` (scrub.rs:59-61, "9{n%10}-{n+1:07}") is injective in n and always yields nine digits, so distinct raw strings always produce distinct CANONICAL synthetics, and a synthetic is never canonically-None. The scrubbed partition therefore strictly refines the real one: it can split one employer into two, never merge.

(A) FIGURE MOVES — verified. `excess_social_security` builds its BTreeSet from `canonical_ein` (return_1040.rs:782-797) and `non_creditable_ss` keys its BTreeMap the same way (:719-724). One employer spelled two ways ("11-1111111" / "111111111") is ONE canonical EIN pre-scrub (eins.len() < 2 => $0 credit, plus the ExcessSsNotCreditable advisory via advisories.rs:706-714) and TWO post-scrub ("90-0000001" / "91-0000002" => creditable - max). max = 168600 * 0.062 = 10,453.20; two W-2s at $6,200 box 4 give 12,400 - 10,453.20 = $1,946.80 — exactly the figure the repo pins at return_1040.rs:6018-6035 as the understatement a Fable review killed. `excess_ss_not_creditable` is an AbsoluteReturn field (:1338), so the advisory flips off too. assemble_absolute genuinely differs.

(B) REFUSAL VANISHES — verified. return_refuse.rs:932-944 refuses ExcessSsEmployerUnknown only when over-cap AND some W-2's canonical_ein is None. scrub.rs:261 filters on `!e.is_empty()`, not canonical validity, so a malformed EIN ("12-345678") or whitespace ("   ") is remapped to a WELL-FORMED synthetic and the screen goes silent. `scrub_return_inputs` (cli/cmd/tax.rs:169-183) applies no screen; `import_return_inputs` (cmd/tax.rs:49+) does no EIN normalization; no other site canonicalizes EINs (grepped). So nothing upstream or downstream repairs it. screen_inputs is on the live filing path (resolve.rs:96, input_form_store.rs:317), so "files where the original refused" is real.

TESTS CANNOT RED — verified. testonly.rs contains no `ein:` at all, so every fixture W-2 is ein: None and `scrub_preserves_every_computed_figure` exercises the EIN path zero times. `ein_distinctness_is_preserved_exactly` (scrub.rs:341-354) uses two byte-identical "11-1111111" strings for its "SAME employer" pair, so it passes identically under raw-string and canonical keying.

NOT A SETTLED FACT: brief fact #4 grants that EINs are remapped preserving distinctness; this finding says the equivalence relation chosen is the wrong one. Different claim.

SEVERITY: Critical stands. The project rubric makes "an unmet guarantee" Critical, and the guarantee unmet is the module's own headline (scrub.rs:6-10) and its explicit warning at :39-44 — "Replacing them independently would silently turn one employer into two and manufacture a credit" — with the code doing precisely that. The --help (cli.rs:425) states "it computes an IDENTICAL return — a guarantee held by a test, not a hope", which is false, and this is the last gate before a permanent crates.io publish. Softening consideration weighed and rejected as decisive: no filer signs the scrubbed copy, so the direct harm is to the debugging artifact (a maintainer chases a phantom $1,946.80 credit, or cannot reproduce a real refusal that the --help names as the primary use case), with the income-import round-trip the help endorses as an indirect path back onto a signed return.

FIX: key EinMap on `canonical_ein(real).unwrap_or_else(|| real.to_string())`, and leave a non-canonical EIN unmapped (or map it to a still-non-canonical value) so ExcessSsEmployerUnknown survives.

</details>


### [CRITICAL] `EinMap` keys on the RAW EIN string while §6413(c) compares CANONICALISED EINs — so two spellings of ONE employer become TWO employers in the scrubbed copy, manufacturing a $1,946.80 excess-SS credit that the original return does not have

**Location:** crates/btctax-core/src/tax/scrub.rs:71 (`.entry(real.to_string())`, in `EinMap::map`, scrub.rs:67-75), against crates/btctax-core/src/tax/return_1040.rs:681-687 (`canonical_ein` strips hyphens and whitespace) and return_1040.rs:782-797

**Failure:** `EinMap` is a `BTreeMap<String,String>` keyed on the W-2's `ein` string exactly as the filer stored it. `excess_social_security` (return_1040.rs:784) keys on `canonical_ein`, which strips hyphens and whitespace. The two disagree on exactly the input the codebase already knows about — return_inputs.rs:55-56 states the governing rule verbatim: "two spellings of one employer are two employers to a string compare", and return_1040.rs:772-777 records that this exact case cost a $1,946.80 phantom credit before it was fixed.

CONCRETE VECTOR. One taxpayer, one employer, a W-2 and its W-2c: `ein = "11-1111111"` (off the paper W-2) and `ein = "111111111"` (off the payroll-portal export), box 4 = $6,200 each. A trailing space from a copy-paste (`" 11-1111111"` vs `"11-1111111"`) does it just as well — `canonical_ein` strips whitespace, `to_string()` does not.

- ORIGINAL: both canonicalise to `"111111111"` → `eins.len() == 1` → return_1040.rs:795 `if eins.len() < 2 { return Usd::ZERO }` → Schedule 3 line 11 = **$0**. No refusal fires either: `over_cap_needs_ein` (return_refuse.rs:938-943) sees both EINs canonicalise, so it passes.
- SCRUBBED: the two raw keys are distinct, so `map()` returns `synthetic_ein(0) = "90-0000001"` and `synthetic_ein(1) = "91-0000002"` → canonicalise to `900000001` and `910000002` → `eins.len() == 2` → creditable = min(6200,10453.20)+min(6200,10453.20) = 12400 → 12400 − 10453.20 = Schedule 3 line 11 = **$1,946.80**.

The scrub therefore RE-INTRODUCES, in the shared copy only, the precise §6413(c) understatement the v0.15.0 fix exists to kill. Harm runs both ways: a filer reporting "why is my excess-SS credit $0?" sends a copy that credits $1,946.80 and the report is closed as not-reproducible; a maintainer handed the copy cold sees a live understatement bug that does not exist in the filer's return. It also falsifies the `--help` promise at cli.rs:422-423 ("W-2 EINs are replaced but keep their SAMENESS") and at cli.rs:425-426 ("it computes an IDENTICAL return — a guarantee held by a test, not a hope").

NEITHER TEST CAN SEE IT. The core fixtures carry no `ein:` at all (grep of testonly.rs: zero hits), so every W-2 in `scrub_preserves_every_computed_figure` has `ein: None` and the EIN map is never entered. `ein_distinctness_is_preserved_exactly` (scrub.rs:338-365) uses byte-IDENTICAL spellings (`"11-1111111"` twice), which is the half that works. All three scrub tests pass green today.

Note the asymmetry: the collapse direction is safe — two genuinely different employers always have different raw strings, hence different synthetics. Only the SPLIT direction is broken, and the split is the direction that manufactures the credit.

**Fix:** Key the map on the canonical form, not the spelling: `let key = canonical_ein(real);` and map on `key` when it is `Some` (falling through to the malformed path below when it is `None`). Then add the missing half of `ein_distinctness_is_preserved_exactly` — a third W-2 spelling the FIRST employer's EIN without hyphens, asserting it still scrubs to the same synthetic as `e[0]`.

<details><summary>Verifier reasoning</summary>

CONFIRMED. Every load-bearing claim checks out against current source, and I could find no place the case is already handled.

**The mechanism, verified line by line**
- `scrub.rs:65` — `struct EinMap(BTreeMap<String, String>)`; `scrub.rs:68-74` — `map()` does `self.0.entry(real.to_string())`. The key is the W-2's `ein` byte-for-byte. No trim, no digit filter.
- `scrub.rs:259-264` — the only caller: `if let Some(e) = w.ein.as_ref().filter(|e| !e.is_empty()) { w.ein = Some(eins.map(e)); }`. Raw `e` is passed straight in.
- `return_1040.rs:681-687` — `canonical_ein` filters out whitespace and `-`, then requires exactly nine ASCII digits. Its own doc comment says "`11-1111111` and `111111111` are ONE employer, and treating them as two understates tax (§6413(c))."
- `return_1040.rs:788-796` — `excess_social_security` builds `eins: BTreeSet<String>` from `w.ein.as_deref().and_then(canonical_ein)` and returns `Usd::ZERO` when `eins.len() < 2`.

So the scrubber decides employer identity by `String` equality and the credit decides it by canonical digits. Two spellings of one employer are one key on the read side and two keys on the scrub side, and the scrub side hands out two *distinct, well-formed* synthetics.

**No normalization anywhere upstream.** I grepped every write path to `W2::ein`: `return_inputs.rs:58` is a bare `Option<String>` with `#[serde(default)]`; the TUI/input-form setter (`btctax-input-form/src/spec/sections.rs:668`) stores `Some(s).filter(|t| !t.trim().is_empty())` — it trims only to decide emptiness and stores `s` **untrimmed**; `W2Ein` has no validator (only 3 hits repo-wide: the spec entry, a coverage map entry, and the enum variant); `income import` does no EIN massaging. The raw spelling reaches the DB and reaches `scrub_pii` verbatim. `cmd/tax.rs:169-183` then serializes the whole scrubbed `ReturnInputs` — `ein` included — to TOML.

**The arithmetic.** `synthetic_ein(n) = format!("9{}-{:07}", n % 10, n + 1)` → n=0 `"90-0000001"`, n=1 `"91-0000002"`, canonicalizing to `900000001` / `910000002`: distinct, nine digits, both valid. `testonly.rs:214` `ss_wage_base = dec!(168600)`, `tables.rs:140` `EMPLOYEE_OASDI_RATE = dec!(0.062)` → cap `$10,453.20`. Two W-2s at box 4 $6,200: original `eins.len()==1` → **$0**; scrubbed `eins.len()==2` → `min(6200,cap)+min(6200,cap) − 10453.20` = **$1,946.80**. The filer's $12,400 total also clears `withheld <= max`, so the early return does not save it.

**No refusal masks it.** `return_refuse.rs:931-945` `over_cap_needs_ein` fires only when some EIN *fails* `canonical_ein`. Both spellings canonicalize, and both synthetics canonicalize, so the screen passes on both sides. Verified verbatim.

**The tests genuinely cannot see it.** `grep -c ein testonly.rs` = 1, and that single hit is the word "reinstate" in a doc comment on line 676 — every fixture W-2 is built with `..Default::default()`, so `ein: None` and `EinMap` is never entered by `scrub_preserves_every_computed_figure`. `ein_distinctness_is_preserved_exactly` (scrub.rs:338-365) uses byte-identical `"11-1111111"` twice — the collapse half, which works — and never calls `assemble_absolute`. I ran them: all 3 scrub tests PASS today.

**Harm, constructed.** The population that reaches this is exactly the population that reaches for `income scrub`: someone over the §3101(a) cap staring at a $0 credit or a "not creditable" advisory and wanting to report it. Two additional distortions strengthen the finding beyond what was filed:
1. `non_creditable_ss` (`return_1040.rs:714-740`) buckets by `canonical_ein` too, so the original emits `Advisory::ExcessSsNotCreditable` for $1,946.80 while the scrubbed copy emits **nothing** — the very symptom being reported vanishes from the shared file.
2. Same root cause, sharper form: a whitespace-only `ein` (`" "`) survives `.filter(|e| !e.is_empty())` and is mapped to a *valid* synthetic. Original → `canonical_ein` = `None` → `ExcessSsEmployerUnknown` refusal; scrubbed → no refusal and possibly a credit. That is the scrubbed copy **filing where the original refuses**, which the module doc (scrub.rs:246-250) explicitly names as the thing it must never do.

**Severity: CRITICAL stands.** Not because a return is misfiled — the scrubbed copy is a debugging artifact and is never signed, and the finding's phrase "re-introduces the §6413(c) understatement" overstates that by a shade. It stands because this repo's rubric makes "an unmet guarantee" Critical, and this is the tool's *single* stated guarantee, broken on the *one* field the module singled out for special handling. `cli.rs:422-423` promises "W-2 EINs are replaced but keep their SAMENESS"; `cli.rs:425-426` promises "it computes an IDENTICAL return — a guarantee held by a test, not a hope." It is a hope. And this is going to crates.io, which is permanent.

**Not a restatement of a settled fact.** Settled fact #4 states the *intent* (remap preserving distinctness for §6413(c)); the finding is that the implementation of that intent fails in the split direction. Settled fact #5 (synthetic SSNs) and the collision question in "Where to look" are a different axis — this is not a collision, it is the inverse.

**Fix:** key `EinMap` on `canonical_ein(real).unwrap_or_else(|| real.to_string())` so identity is decided the same way §6413(c) decides it, and leave an EIN that fails canonicalization untouched (or map it to an equally-unparseable placeholder) so the refusal survives the scrub. Pair it with the negative test B1 demands: two spellings of one employer, box 4 $6,200 each, asserting Schedule 3 line 11 is $0 on both sides — it reds today.

</details>


### [IMPORTANT] `scrub_pii` normalizes absent/malformed identity into well-formed identity, so the scrubbed copy FILES where the original REFUSES — erasing at least four fail-closed screens

**Location:** crates/btctax-core/src/tax/scrub.rs:91, :111 (synthetic SSNs), :164 (ip_pin dropped), :251 (business_description), :261-263 (EIN); screens at crates/btctax-core/src/tax/packet.rs:58-73, :167-179, :204-211, :380-460 and crates/btctax-core/src/tax/return_refuse.rs:899-909, :931-957

**Failure:** Four screens key on exactly the malformedness the scrub launders away. (1) `Ssn::canonical` returns `Err(SsnError::Missing)` on an empty SSN and `WrongLength`/`NotDigits` on a typo, and `ReturnHeader::build` propagates it, failing the whole print/export. An SSN-less return is the DOCUMENTED normal state between `income import` and `set-pii` (return_inputs.rs:234-238; pinned by `an_uncaptured_ssn_does_not_block_the_report`, return_refuse.rs:1589-1595) — so the single most common real filer state exports cleanly once scrubbed. (2) A malformed `ip_pin` fails `ReturnHeader::build` via `IpPin::canonical` (packet.rs:452-457); the scrub drops it to `None`, so the refusal vanishes. (3) An empty `business_description` refuses (`ScheduleCNoBusinessDescription`, return_refuse.rs:899-909); scrub.rs:251 unconditionally writes `"Example business"` — the module doc at :246-250 reasons only about the OPPOSITE direction and misses this one. (4) A non-empty but malformed EIN while over the SS cap refuses (`ExcessSsEmployerUnknown`, return_refuse.rs:931-957); scrub.rs:261-263 filters only on `is_empty()`, so `Some("11-111111")` becomes a well-formed synthetic — refusal erased AND the §6413(c) credit now computes (see the CRITICAL). A filer whose report is "btctax refuses my return" hands over a file that does not refuse; the maintainer cannot reproduce it and may close it. `--help` (cli.rs:419-421) tells them "every fail-loud declaration" survives, and `scrub_preserves_every_computed_figure` cannot see any of this because it compares `AbsoluteReturn`, which is downstream of all four boundaries.

**Fix:** Make each replacement PRESERVE the screen-relevant property instead of overwriting it: emit a synthetic SSN only when the real one canonicalizes (otherwise emit an equally-malformed stand-in, e.g. `""` for missing, `"1{n}-00-000"` for wrong-length); keep `ip_pin` as `Some("000000")` vs `None` by whether the original canonicalized; leave `business_description` empty when it was empty; leave a non-canonicalizable EIN non-canonicalizable. Add a test that `screen_inputs(&ri).is_some() == screen_inputs(&scrub_pii(&ri)).is_some()` and the same for `ReturnHeader::build(...).is_err()` over a fixture set that includes an SSN-less, a bad-IP-PIN, an empty-description and a bad-EIN return.

<details><summary>Verifier reasoning</summary>

CONFIRMED against source. All four mechanisms verified: (1) Ssn::canonical (packet.rs:58-73) errs on empty/malformed and ReturnHeader::build propagates via FiledPerson::build (:204-211) and the dependent loop (:381-460); synthetic_ssn(1)="101-00-0002" canonicalizes, so the original refuses at print and the scrubbed copy does not. The SSN-less state is documented-normal (return_inputs.rs:231-238) and pinned by an_uncaptured_ssn_does_not_block_the_report (return_refuse.rs:1589). (2) IpPin::canonical (packet.rs:167-179) is propagated at packet.rs:452-457; scrub.rs:164 sets None and .transpose() on None is Ok(None), erasing the refusal (reachable via `income import` TOML, since the TUI validates through parse_ip_pin). (3) return_refuse.rs:899-909 refuses on an empty business_description and its own comment notes serde(default) makes "" an ordinary import outcome; scrub.rs:251 unconditionally writes "Example business", and the doc at :246-250 reasons only about the mirror direction. (4) canonical_ein (return_1040.rs:681-687) requires exactly nine digits, so Some("11-111111") is None and over the cap fires ExcessSsEmployerUnknown (return_refuse.rs:931-957); scrub.rs:261-263 filters only is_empty(), mapping it to a valid 90-0000001 — the refusal vanishes and §6413(c) now computes a credit the original never produced. The module doc claims EINs keep "distinctness and nothing else"; they also silently gain VALIDITY, the property the screen actually reads. Blindness verified: assemble_absolute (return_1040.rs:1441) calls neither ReturnHeader::build nor screen_inputs, so scrub_preserves_every_computed_figure sits upstream of all four boundaries. The --help (cli.rs:415-422) explicitly promises "reproduce a refusal" and that "every fail-loud declaration" survives; both are false here, and main.rs:314 adds no caveat. Not barred by the settled facts: fact 3 blesses dropping the IP PIN as a disclosure decision and is silent on the erased refusal (the fix is a same-shape non-credential placeholder, not a synthetic valid PIN); fact 4 blesses distinctness-preservation, not validity-conferral. Every cited line number checks out (cli.rs:419-421, packet.rs:452-457, return_refuse.rs:903/948/1589, return_inputs.rs:234-238). One temper: instance (1) is the least clear-cut, since preserving emptiness would make the common "no PII yet, wrong figure" report un-exportable for the recipient — that instance may warrant a documented normalization rather than propagating the blank; (3) and (4) have no such tension. Severity stays IMPORTANT and not CRITICAL: no PII escapes and no real filer's filed return moves, but this is a verified missing case across four fail-closed screens plus a false user-facing promise at a permanent-publish gate, one instance of which converts a refusal into a computed refundable credit. (Incidental, out of scope: `set-pii`, cited in cli.rs:450 and return_inputs.rs:234, is not an actual subcommand — stale doc, does not affect the finding.)

</details>


### [IMPORTANT] `the_identity_does_not_survive` names 6 of the 16 identity fields the scrubber replaces; the other 10 — including the ENTIRE spouse — are held by no test in the workspace

**Location:** crates/btctax-core/src/tax/scrub.rs:370-422; fixture at crates/btctax-core/src/tax/testonly.rs:256-320

**Failure:** The test asserts only: taxpayer `ssn`, taxpayer `last_name`, `address_street`, `ip_pin == None`, `dependents[].ssn`, `business_description`, `foreign_country_names`, and the `-00-` shape. Never asserted, though `kitchen_sink_household` populates every one: the whole spouse `Person` ("Jane"/"Doe"/"987-65-4321"/"Architect", testonly.rs:258), taxpayer `first_name` and `occupation`, `dependents[].name` ("Sam Doe", :264), `address_city`/`address_state`/`address_zip`, `w2s[].employer` ("ACME"/"GLOBEX"), and all four payer names ("First Bank"/"Broker LLC"/"State of IL"). The sibling invariant cannot cover them either: none of those strings reach `AbsoluteReturn` — the only strings there are `printed_inputs.schedule_c_header.{business_description,naics_code}` (return_1040.rs:1360-1372) and `excess_ss_not_creditable[].ein` (:692-699). And `scrub_pii` has exactly three call sites, all inside this module — there is no CLI integration test for `income scrub` at all (`grep -rn scrub_pii crates/`). Concretely: change :140 to `spouse: spouse.clone()` and the entire workspace stays green while a real filer's spouse's full name and SSN ship in a file the user was told is safe to send to a stranger. Under this repo's B1 rule ("no checker exists until it has been observed RED on a planted defect"), the module whose only job is disclosure has no disclosure checker for two-thirds of its surface.

**Fix:** Replace the hand-picked assertions with an exhaustive walk: destructure the scrubbed and original `Person`/`Dependent`/`HouseholdHeader`/W-2/1099 rows with no `..` and assert field-by-field that each replaced field differs and each kept field matches — so adding a field forces an assertion, the same discipline the scrubber itself uses. Verify by mutation (delete the spouse scrub; the test must red).

<details><summary>Verifier reasoning</summary>

CONFIRMED as IMPORTANT. Every load-bearing claim checks out against source; two peripheral details in the finding are wrong and one corroborating fact it missed makes it stronger.

VERIFIED — the assertion set. `the_identity_does_not_survive` (scrub.rs:370-422) asserts exactly: `taxpayer.ssn` (:374), `taxpayer.last_name` (:375), `address_street` (:376), `ip_pin == None` (:379), `dependents[].ssn`/`relationship` (:381-387), `business_description` (:398), `foreign_country_names` (:405,:411), and the `-00-` shape (:418). Nothing else.

VERIFIED — the fixture populates every unasserted field. testonly.rs:257-320: taxpayer `person("John","Doe","123-45-6789","Engineer")`; spouse `person("Jane","Doe","987-65-4321","Architect")` (:258); `address_city/state/zip` = Springfield/IL/62704 (:260-262); `dependents[0].name = "Sam Doe"` (:264); `w2s[].employer` = ACME (:275) / GLOBEX (:287); payers "First Bank" (:300), "Broker LLC" (:308), "State of IL" (:317). None is asserted anywhere.

VERIFIED — the sibling invariant cannot cover them. `scrub_preserves_every_computed_figure` asserts `a == b` between the ORIGINAL and SCRUBBED `AbsoluteReturn`, blanking only `printed_inputs.schedule_c_header.business_description` and `f8995a_parts_i_to_iii.part_i.col_a_name`. I ran the module (`cargo nextest run -p btctax-core scrub` — 3/3 PASS), which is itself the proof: if the spouse name, employer, payer or address reached `AbsoluteReturn`, that test would already be RED today, because kitchen-sink's "Jane"/"Doe"/"ACME"/"First Bank" all get replaced and amt_owing's Boulder/CO/80301 does too. So the invariant is structurally incapable of seeing any of them, exactly as claimed.

VERIFIED — no other coverage exists. `scrub_pii` is reachable only from the 3 tests in this module plus one production site. `crates/btctax-cli/tests/` has 39 integration files and not one exercises `income scrub`; `cmd/tax.rs`'s own `mod tests` has three test fns, none touching `scrub_return_inputs`. The planted defect `spouse: spouse.clone()` at scrub.rs:140 therefore leaves the entire workspace green while a real filer's spouse's full name, SSN and occupation ship in a file `main.rs:314` tells the user is shareable.

CORRECTIONS to the finding (neither changes the verdict):
1. "`scrub_pii` has exactly three call sites, all inside this module" is FALSE — there are four; the fourth is the production caller `crates/btctax-cli/src/cmd/tax.rs:176`. The intended point (no integration test) is correct.
2. "the only strings there are `schedule_c_header.{business_description,naics_code}` and `excess_ss_not_creditable[].ein`" is incomplete — `f8995a_parts_i_to_iii.part_i.col_a_name` (qbi_a.rs:137, fed from `business_name` at :377) is a third identity-bearing String in `AbsoluteReturn`. It carries the same business description and IS blanked by the test, so the conclusion is unaffected.

CORROBORATION the finding missed, which raises rather than lowers severity:
- The address fields are not merely unasserted, they are UNASSERTABLE on this fixture. `scrub_header` writes city="Springfield" (:142), state="IL" (:146), zip="62704" (:147) — byte-identical to kitchen-sink's own :260-262. An `assert_ne!` on those three against `kitchen_sink_household` can never pass even when scrubbing works correctly. That is a fixture that structurally cannot discriminate — the exact F2/F4 class this repo's B1 rule names ("an instrument trusted without ever being watched to distinguish a true case from a false one").
- `b_1099[].payer` is scrubbed at scrub.rs:271-273, but `the_identity_does_not_survive` runs only on kitchen-sink, which has NO `b_1099` (the only `b_1099` fixture is `amt_owing_household`, testonly.rs:498). That loop is never executed by the identity test at all.

WHY NOT DEFLATED. It is not a live leak — today every listed field IS scrubbed, so nothing ships identity, which is why it is not Critical. But the repo's own rubric makes it blocking: "A guarantee without a test that reds when it is removed does not exist. Mutation-verify" and B1's "no checker exists until it has been observed RED on a planted defect." The module doc asserts "The identity is actually gone" and the test carrying that name holds it for roughly a third of the surface. The realistic regression path is not the hand-planted `spouse.clone()` — it is the next 1099 type (NEC/R/MISC/K) added at the top level: the no-`..` destructure at :205-239 forces a DECISION but no test checks the decision was right, and `_` is precisely what an author writes meaning "money only". Immediately before a permanent crates.io publish, in the one module whose sole job is disclosure, that is a missing case, not a nit.

FIX: extend `the_identity_does_not_survive` to assert every replaced field — the whole spouse (all four), taxpayer `first_name`/`occupation`, `dependents[].name`, `w2s[].employer`, all four payer lists — and change kitchen-sink's city/state/zip off the scrub constants (or assert against `amt_owing_household`, Boulder/CO/80301) so the address is discriminable at all. Add a `b_1099`-bearing fixture to that test.

</details>


### [IMPORTANT] An empty `business_description` is overwritten unconditionally, so a return that refuses ScheduleCNoBusinessDescription files after scrubbing

**Location:** crates/btctax-core/src/tax/scrub.rs:245-255 (sc_out.business_description = "Example business".into()); vs crates/btctax-core/src/tax/return_refuse.rs:900-909

**Failure:** The comment at scrub.rs:246-250 reasons about one direction only -- 'it must stay NON-EMPTY: an empty description refuses the return, so scrubbing it to "" would make the scrubbed copy refuse where the original filed' -- and misses the inverse. business_description is #[serde(default)], so a TOML import that omits the key yields ""; that is the entire reason ScheduleCNoBusinessDescription exists (return_refuse.rs:196-201). Confirmed by running the screen on both copies: original screen_inputs = Some(ScheduleCNoBusinessDescription), scrubbed = None. A filer refused for exactly this reason is a prime candidate to run `income scrub` and send the file; the recipient runs it, sees a Schedule C computing happily, and reports 'works for me'. The --help at cli.rs:415-416 sells the command as the way 'to reproduce a refusal'.

**Fix:** Preserve the blank: `sc_out.business_description = if sc.business_description.trim().is_empty() { sc.business_description.clone() } else { "Example business".into() };` -- the `_sc` binding at scrub.rs:245 is already in scope. Add a test asserting screen_inputs returns the same RefuseReason on the original and the scrubbed copy.

<details><summary>Verifier reasoning</summary>

CONFIRMED at IMPORTANT. Every cited line says what the finding claims.

Source chain, all verified:
- return_inputs.rs:326-327 — `#[serde(default)] pub business_description: String`, so an import omitting the key yields "".
- cmd/tax.rs:49-103 (`import_return_inputs`) — parses TOML and calls `return_inputs::set`; NO refusal screen runs at import, so an empty-description row is genuinely storable.
- return_refuse.rs:900-909 — refuses `ScheduleCNoBusinessDescription` when `c.business_description.trim().is_empty()`.
- scrub.rs:245-251 — `sc_out.business_description = "Example business".into()` fires unconditionally whenever `schedule_c` is `Some`, with no empty check.
- cmd/tax.rs:169-183 (`scrub_return_inputs`) — reads the stored row, scrubs, serializes. No screen anywhere in the path.

The existing test at return_refuse.rs:1470-1509 proves both halves without executing anything: leg 1 sets `business_description: String::new()` on `ri()` and asserts `Some(ScheduleCNoBusinessDescription)`; leg 3 uses the same fixture with "Bitcoin mining" and asserts `reason(&ok) == None`. Scrub's transformation is exactly leg-1 → leg-3 on that fixture, so the refusal vanishes.

Not excluded by the brief's SETTLED FACTS: fact #1 closes the LEAK (the field rode through unscrubbed). This is the inverse direction the fix itself introduced, and the brief explicitly solicits it ("Can the scrubbed copy refuse where the original filed, or file where the original refused?"). The comment at scrub.rs:246-250 argues only the first direction, in writing.

The asymmetry is local and self-evidently an oversight: `scrub_name_list` (scrub.rs:174-183) explicitly returns `String::new()` on empty input precisely to preserve the `ScheduleBForeignCountryMissing` refusal in both directions. `business_description` is the one field where that was not done.

No test covers it: `scrub_preserves_every_computed_figure` compares `assemble_absolute`, which never runs `screen_inputs`, and all three fixtures carry non-empty descriptions. `the_identity_does_not_survive` asserts the output IS "Example business" — it pins the defect rather than catching it.

Harm is concrete and hits the advertised purpose: cli.rs:415-416 reads "so they can reproduce a refusal or a wrong figure without receiving your PII." A filer who believes the refusal is wrong (f1040sc.map.toml:104 notes the form directs a filer with no separate business name to leave line C blank — an easy conflation with line A) scrubs, sends, and the maintainer's run comes back clean.

Severity: not CRITICAL — no identity reaches the output, no figure on any filed return moves, no data loss. Not MINOR — a real defect with a named missing case that defeats the command's headline promise, matching the project's "real defect, missing case" bar.

One scoping caveat on the filed text that does not change the verdict: "sees a Schedule C computing happily" holds only when the empty description was the return's sole blocker, since `refuse()` returns the first reason and the recipient could land on a different one. Either way the reported refusal fails to reproduce.

Fix: guard the assignment with `if !sc_out.business_description.trim().is_empty()`, mirroring scrub_name_list; add a test asserting `screen_inputs` yields the same `RefuseReason` on the original and the scrubbed copy, which generalizes past this one field.

</details>


### [IMPORTANT] A malformed EIN is upgraded to a well-formed synthetic one, so the `ExcessSsEmployerUnknown` fail-closed refusal disappears from the scrubbed copy

**Location:** crates/btctax-core/src/tax/scrub.rs:261-263, against crates/btctax-core/src/tax/return_refuse.rs:936-955

**Failure:** scrub.rs:261 guards only the exactly-empty case: `w.ein.as_ref().filter(|e| !e.is_empty())`. Any non-empty string is handed to `EinMap::map` and comes back as a well-formed nine-digit synthetic. But `canonical_ein` rejects far more than the empty string — a dropped digit (`"11-111111"`, eight digits), a letter (`"XX-1111111"`), or whitespace-only (`"   "`) all return `None` (return_1040.rs:686; the rejections are pinned at return_1040.rs:6038-6044).

CONCRETE VECTOR. Taxpayer, two W-2s, box 4 = $6,200 each (aggregate $12,400 > the $10,453.20 cap), one of them typed `ein = "11-111111"`.
- ORIGINAL: `over_cap_needs_ein` (return_refuse.rs:938-943) finds a W-2 whose `canonical_ein` is `None` while the aggregate is over cap → `RefuseReason::ExcessSsEmployerUnknown` → **the return refuses and files nothing**.
- SCRUBBED: `"11-111111"` is not `is_empty()`, so it becomes `"90-0000001"`, which canonicalises fine → no refusal → the return assembles and computes a credit.

The filer's most likely reason to run `income scrub` on this return is to report that very refusal. The scrubbed copy they are told reproduces the problem silently fixes it. `import_return_inputs` (cmd/tax.rs:49-56) does not screen, and `scrub_return_inputs` (cmd/tax.rs:169-183) reads the stored row and scrubs it directly, so a malformed EIN is both storable and scrubbable. The guard as written handles the one case (`Some("")`) that needed no handling — it already canonicalises to `None` on both sides — and lets through every case that breaks.

**Fix:** Preserve the canonicalisability class, not just non-emptiness: if `canonical_ein(e).is_none()`, leave a fixed non-canonicalisable stand-in (e.g. `Some("00-000000".into())`) rather than a valid synthetic, so the screen still refuses on the scrubbed copy. Pair it with a test that plants an eight-digit EIN over the cap and asserts both copies refuse.

<details><summary>Verifier reasoning</summary>

CONFIRMED — it reproduces exactly as filed. I built an out-of-repo probe crate against `btctax-core` (repo untouched; `git status` clean) and ran the finding's own vector.

**The vector, executed.** `w2_only_household()` with two Taxpayer W-2s, box 4 = $6,200 each (aggregate $12,400 > the $10,453.20 cap), one EIN typed `"11-111111"` (a dropped digit):

```
  original EINs : [Some("11-111111"), Some("22-2222222")]
  scrubbed EINs : [Some("90-0000001"), Some("91-0000002")]
  ORIGINAL screen : Some("ExcessSsEmployerUnknown")
  SCRUBBED screen : None          >>> DIVERGES: true
```

And it is worse than the finding claimed — the figure moves too, not just the refusal. `assemble_absolute` on the same pair: **Schedule 3 line 11 excess-SS credit goes $0 -> $1,946.80**, and the whole `AbsoluteReturn` compares unequal. `"XX-1111111"` and `"   "` diverge identically.

**The controls discriminate**, so this is not a probe that reds on everything: `Some("")` -> DIVERGES false, `None` -> DIVERGES false, two well-formed EINs -> DIVERGES false.

**Every citation is accurate.** scrub.rs:261 is verbatim `w.ein.as_ref().filter(|e| !e.is_empty())`; return_1040.rs:686 requires exactly nine digits; return_1040.rs:6040-6044 pins `canonical_ein("11-111111") == None` and `canonical_ein("XX-1111111") == None`; return_refuse.rs:938-943 is the `any(... canonical_ein ... .is_none())` closure feeding `ExcessSsEmployerUnknown` at :948.

**Reachable, and over-represented among scrub users.** `parse_return_inputs_toml` (cmd/tax.rs:114-132) validates only unknown *keys* — no EIN format screen; the TUI (input-form/spec/sections.rs:668) only trims-and-nonempty. A dropped digit in a hand-typed nine-digit number is a mainstream typo. More pointedly, the population that hits this refusal is precisely the population told to report it: the `--help` at cli.rs:415 names the use case as "so they can **reproduce a refusal**", and cli.rs:425 promises "it computes an IDENTICAL return — a guarantee held by a test, not a hope." On this vector that promise is false.

**Nothing already handles it.** `screen_inputs` is the only gate (called at cli/resolve.rs:96) and it is what disappears. `excess_social_security`'s conservative `None => return Usd::ZERO` arm (return_1040.rs:791) cannot help — the scrubbed EIN is well-formed, so that arm is never reached.

**No existing test can see it.** The guarantee test's three fixtures are blind: kitchen-sink is `eins=[None, None]` (the `if let Some(e)` never fires — `None` is not `Some("")`), W-2-only is `[None]`, AMT-owing has no W-2s. `ein_distinctness_is_preserved_exactly` only ever compares byte-identical spellings.

**One sub-claim in the finding is wrong, and it matters for the fix.** The finding says the guard "handles the one case (`Some("")`) that needed no handling — it already canonicalises to `None` on both sides." It does not: my control shows the guard is *load-bearing* for `Some("")`. Delete it and the empty case gets upgraded to a synthetic and diverges too. The fix is to **widen** the predicate, not remove it.

**Same root cause, second broken promise (corroborating, not a separate filing).** `EinMap` keys on the raw string rather than `canonical_ein`, so `("11-1111111", "111111111")` — the two renderings `canonical_ein`'s own doc calls out as "off the paper W-2" vs "off a payroll-portal export" — map to two distinct synthetics: credit $0 -> $1,946.80, the exact understatement pinned at return_1040.rs:6030. That directly contradicts scrub.rs:39-44's bolded claim that "two W-2s that shared an employer still share one afterwards … Replacing them independently would silently turn one employer into two and manufacture a credit." One fix closes both: key the map on `canonical_ein(raw)`, and for `None` emit a placeholder that is *also* non-canonical (e.g. eight digits) so the un-decidability survives without passing 8 of the 9 real digits through into a file marked safe to share.

**Severity: IMPORTANT as filed, not inflated.** The repo rubric's "unmet guarantee" arguably reads Critical, and the module's central invariant plus a `--help` promise are both falsified. But the brief ranks DISCLOSURE above DISTORTION, and this is DISTORTION on a debugging artifact: no signed 1040 moves. The concrete harm is a maintainer running the scrubbed copy, seeing no refusal, and closing the report as not-reproducible — or chasing a phantom $1,946.80 §6413(c) understatement that exists only in the scrubbed copy. That is squarely Important.

Probe: /tmp/claude-1000/-scratch-code-bitcoin-tax/2b9d8116-e1de-4038-a101-5fd575fdcb0e/scratchpad/verify/

</details>


### [IMPORTANT] An uncaptured or malformed SSN is replaced with a well-formed synthetic one, so the filable-packet refusal (`HeaderError::Ssn`) disappears — and `scrub_preserves_every_computed_figure` structurally cannot see it, because `AbsoluteReturn` never reads an SSN

**Location:** crates/btctax-core/src/tax/scrub.rs:91 and scrub.rs:111 (`ssn: synthetic_ssn(n)` / `synthetic_ssn(100 + n)`, unconditional), against crates/btctax-core/src/tax/packet.rs:58-73 and packet.rs:208, 425

**Failure:** `scrub_person` and `scrub_dependent` write `synthetic_ssn(..)` unconditionally, with no reference to what was there. A blank SSN is the NORMAL state in this codebase, not an edge case: return_refuse.rs:693-698 records "NO SSN GATE HERE, DELIBERATELY", the report computes fine without one, and "the identity boundary is the FILABLE PACKET". return_refuse.rs:1583-1597 pins it — `assert_eq!(r.header.taxpayer.ssn, "", "the fixture captured no PII")` with the report still computing — and cli.rs:449-450 tells the filer SSNs live in a separate `set-pii` step they may never have run.

CONCRETE VECTOR. A filer who never ran `set-pii` (or who typo'd an SSN to eight digits) runs `report` and gets numbers, then tries to emit PDFs and hits `ReturnHeader::build` → `HeaderError::Ssn(SsnError::Missing)` (or `WrongLength`/`NotDigits`), packet.rs:63-71 via packet.rs:208. They run `income scrub` to hand the failure over. Every SSN in the scrubbed copy is now `"101-00-0002"` / `"102-00-0003"` / `"101-00-0102"`… — all nine digits, all accepted by `Ssn::canonical` (which validates length and digit-ness only, no SSA area/group rules) — so the recipient's copy **emits the PDF cleanly** and the reported failure is unreproducible.

The module's headline guarantee cannot catch this by construction: `scrub_preserves_every_computed_figure` compares `AbsoluteReturn`, and return_refuse.rs:1586-1587 states outright that "there is no number on the return that an absent SSN could make wrong". The refusal lives at the print boundary, which no test in scrub.rs exercises. Note this is a strictly different mechanism from the EIN case: here even a *present, well-formed* real SSN is fine to replace — the defect is only that ABSENT and MALFORMED are both silently promoted to VALID.

**Fix:** Make the replacement class-preserving in `scrub_person`/`scrub_dependent`: leave an empty `ssn` empty, map a value that fails `Ssn::canonical` to a fixed malformed stand-in, and emit `synthetic_ssn(n)` only for one that canonicalises. Hold it with a test asserting `ReturnHeader::build` succeeds on the scrubbed copy iff it succeeds on the original.

<details><summary>Verifier reasoning</summary>

CONFIRMED. Every cited line says what the finding claims.

SOURCE VERIFIED: scrub.rs:79-87 destructures Person with `ssn: _` and scrub.rs:91 writes `synthetic_ssn(n)` unconditionally; same for dependents at :103-111. `Ssn::canonical` (packet.rs:58-73) validates only non-empty / all-digits / len==9 — `synthetic_ssn(1)` = "101-00-0002" passes. `FiledPerson::build` (packet.rs:208) and the DependentRow loop (packet.rs:425) are the only SSN gates, both funnelling to `HeaderError::Ssn`. `AbsoluteReturn` (return_1040.rs:1194+) and `PrintedInputs` (:1388+) contain no SSN field at all (the only `ssn` in that file is a fixture at :6531), so `scrub_preserves_every_computed_figure` cannot observe the flip by construction. `scrub_pii` has one non-test caller (cmd/tax.rs:176) and no test in the repo crosses `ReturnHeader::build` on a scrubbed return.

THREE CORROBORATIONS THE FINDING DID NOT CITE, all strengthening it:
1. return_inputs.rs:234 states the codebase's own position: the taxpayer's PII is `#[serde(default)]` because it "is captured LATER (`btctax set-pii`) and is only enforced at export (an SSN-less return refuses there, not at import)". An SSN-less stored return is the DOCUMENTED DEFAULT, not an edge case. `set-pii` does not exist as a CLI subcommand anywhere in crates/ (prose references only), so TOML import is the sole path in — making empty SSNs even more common.
2. cli.rs:415-416, the scrub --help, promises the copy is for handing over "so they can reproduce a REFUSAL or a wrong figure." The brief explicitly asks whether every promise in the help is true of the code; for the whole `HeaderError::Ssn` class it is not.
3. The same module already applies the missing rule one field over: scrub.rs:261 `if let Some(e) = w.ein.as_ref().filter(|e| !e.is_empty())` deliberately leaves an empty EIN empty. The author knew emptiness is load-bearing shape; the SSN path has no analogue. Internal inconsistency, not a judgement call.

HARM CHAIN IS TIGHTER THAN CLAIMED: `HeaderError`'s Display (packet.rs:142) is "an SSN {e} — fix the identity and re-run" — it never names WHICH person (taxpayer, spouse, or which dependent). And `income show` masks via `mask_ssn` (cmd/tax.rs:139-144) to a fixed-width `***-**-NNNN` regardless of true length, so the digit count that would identify the malformed one is hidden. A filer with a spouse + 3 dependents who typo'd one SSN to 8 digits therefore CANNOT self-diagnose with btctax's own tools; scrubbing and handing the file over is the tool-suggested next step, and it is exactly the step that destroys the evidence — the recipient's copy prints cleanly, concludes user error, and the real UX defect never gets found.

NOT A SETTLED FACT: brief fact 5 covers only the middle group `00` of the EMITTED SSN, not the promotion of ABSENT/MALFORMED to VALID. Not the EIN case (guarded). Not one of the three already-fixed r1 issues.

NOT UNREACHABLE, NOT ALREADY HANDLED: nothing between the vault and `scrub_pii` preserves SSN shape; the scrubbed TOML is `income import`-able by design.

SEVERITY HOLDS AT IMPORTANT, not inflated and not Critical: no PII reaches the output (the worse direction per the brief) and no filed figure moves, so it does not rise to Critical. But it is a missing case that silently deletes an entire refusal class from a shareable copy, defeats an explicit user-facing promise, is reachable from the documented default state, is invisible to every existing test by construction, and the fix is ~3 lines (empty -> empty; malformed -> a synthetic of the same malformed class).

</details>


### [IMPORTANT] A return that REFUSES to print scrubs into one that prints a full 12-form packet — the scrub erases the very refusal the tool tells the filer to send it to reproduce.

**Location:** crates/btctax-core/src/tax/scrub.rs:91 (`ssn: synthetic_ssn(n)`) and :164 (`ip_pin: None`), against crates/btctax-core/src/tax/packet.rs:397/425/451-456 (`FiledPerson::build` / `Ssn::canonical(&d.ssn)` / `IpPin::canonical`)

**Failure:** The print boundary `ReturnHeader::build` is FALLIBLE and is the only thing that validates identity strings; `assemble_absolute` is infallible and never sees them. `scrub_pii` unconditionally mints a canonical synthetic SSN for the taxpayer, the spouse and every dependent, and unconditionally drops the IP PIN — so it upgrades an unprintable identity into a printable one. VERIFIED END-TO-END on the built binary (target/debug/btctax, nine_dependents_amt fixture): (1) a stored return with no `[header.taxpayer]` table — an explicitly supported state, `HouseholdHeader::taxpayer` is `#[serde(default)]` because "PII is captured LATER … an SSN-less return refuses there, not at import" (return_inputs.rs:236-239) — gives `export-irs-pdf`: `error: the 2024 return cannot be printed: an SSN no SSN was entered`; its scrubbed copy exports 12 forms + dependents_statement.txt cleanly. (2) `ip_pin = "1234"` gives `error: … an SSN has 4 digits`; its scrubbed copy exports 12 forms cleanly. The filer hits a refusal, does exactly what `cli.rs:415-416` tells them ("so they can reproduce a refusal … without receiving your PII"), and the maintainer receives a file on which the reported bug does not exist. This is the module's own stated failure mode (scrub.rs:6-8) running in reverse, and no test can see it because every test in scrub.rs stops at `assemble_absolute` and none ever calls `ReturnHeader::build` or the packet path.

**Fix:** Preserve each identity value's VALIDITY CLASS rather than its content: if `Ssn::canonical(real)` errs, emit a value that still errs (e.g. keep `String::new()`); only replace a canonical SSN with a canonical synthetic one. Same for the IP PIN — `None` stays `None`, `Some(malformed)` stays `Some(malformed-but-not-a-credential)`. Then add the kill-test the invariant is missing: for each fixture assert `ReturnHeader::build(&ri, y).is_ok() == ReturnHeader::build(&scrub_pii(&ri), y).is_ok()`, with an SSN-less and a bad-IP-PIN fixture in the set.

<details><summary>Verifier reasoning</summary>

CONFIRMED as a real defect, but the severity is inflated: IMPORTANT, not CRITICAL.

WHAT I VERIFIED IN SOURCE (every claim checked, none taken on trust)

1. The fallible/infallible split is exactly as alleged. `ReturnHeader::build` (packet.rs:381-460) is the only identity validator: `FiledPerson::build` → `Ssn::canonical` (packet.rs:58-73, errors on empty / non-digit / len≠9) at :397, the same for the spouse at :398-403 and every dependent at :425, and `IpPin::canonical` (packet.rs:167-179, errors on len≠6) at :451-456. It is reached only via `assemble_printed_return` (packet.rs:553). `assemble_absolute` never sees an SSN — return_refuse.rs:693-698 says so explicitly ("NO SSN GATE HERE, DELIBERATELY … The identity boundary is the FILABLE PACKET").

2. `scrub_pii` unconditionally upgrades identity printability. `synthetic_ssn` (scrub.rs:53-55) always yields 9 digits, so scrub.rs:91 and :111 mint SSNs that always canonicalize; scrub.rs:164 sets `ip_pin: None`, which `.map(IpPin::canonical).transpose()?` turns into `Ok(None)`.

3. The reachability premise holds. `HouseholdHeader::taxpayer` is `#[serde(default)]` with the quoted rationale at return_inputs.rs:232-239 ("PII is captured LATER … an SSN-less return refuses there, not at import"), and `Person: Default` gives `ssn: ""`.

4. The `--help` promise is where the reviewer says it is (cli.rs, `IncomeCmd::Scrub` doc): "so they can reproduce **a refusal** or a wrong figure without receiving your PII."

5. "No test can see it" is true. `scrub_pii`/`scrub_return_inputs` are referenced only by scrub.rs's own tests, main.rs:309 and cmd/tax.rs:176. Every scrub test stops at `assemble_absolute`; nothing anywhere composes a scrubbed return through `ReturnHeader::build`.

END-TO-END REPRODUCTION (target/debug/btctax, nine_dependents_amt fixture, throwaway vaults in the scratchpad)
- Delete `[header.taxpayer]`: original → `error: the 2024 return cannot be printed: an SSN no SSN was entered`. Scrubbed copy (`ssn = "101-00-0002"`) → imports and exports 13 files (12 forms + dependents_statement.txt), exit 0.
- Add `ip_pin = "1234"`: original → `error: … an SSN has 4 digits — an SSN has exactly 9`. Scrubbed copy (no `ip_pin` key at all) → 13 files, zero errors.

WHY NOT A SETTLED FACT. Settled fact 3 blesses DROPPING the IP PIN as a disclosure decision. This finding is about that drop's effect at a different boundary (print), and the SSN half is wholly independent of it. Not a restatement.

WHY THE SEVERITY IS INFLATED
- The brief itself ranks DISCLOSURE as the worse direction; nothing leaks here. No tax figure moves, no filer's return is misstated, no data is lost. The module's narrow written guarantee (`assemble_absolute(original) == assemble_absolute(scrubbed)`) is actually MET; what is falsified is the softer `--help` promise about reproducing a refusal.
- The erased set is narrow and I enumerated it: only `HeaderError::Ssn`. Every other refusal survives the scrub intact — the whole `return_refuse.rs` surface, the `FORM_QUESTIONS` unanswered loop inside `build` (scrub preserves all those tri-states), and `MfjWithoutSpouse` (scrub preserves `spouse` as Some/None).
- Scenario 1 is largely self-diagnosing: the refusal text is actionable ("fix the identity and re-run"), so a filer fixes it rather than reporting it, and a maintainer who did receive it would see `ssn = "101-00-0002"` sitting in the file next to a report of "no SSN was entered".
- The genuinely costly sub-case is scenario 2, and it is costly because of a *separate* mislabel I confirmed: an invalid IP PIN is reported as `an SSN has 4 digits` (HeaderError::Ssn wraps IpPin's SsnError at packet.rs:451-456). That is a refusal a filer with a perfectly good SSN would report as a bug, and the scrub does erase it. Worth fixing — a wasted round-trip, not a wrong return.

So: a real, verified, pre-publish-worthy defect sitting precisely in the blind spot the brief named ("What is NOT covered by the computed-figure invariant … the PRINTED packet"). Fix is small — have `income scrub` run `ReturnHeader::build` on the ORIGINAL and refuse (or loudly warn) when it errors, so an unprintable return cannot silently scrub into a printable one. But it is a debugging false-negative, not a wrong result, a leak, or an unmet stated guarantee, so it does not sit level with a PII escape.

</details>


### [IMPORTANT] The three `Person` fields the module documents as "★★ KEPT … scrubbing them would move the deduction and break the invariant" are `None` on all three fixtures, so the invariant asserts `None == None` and holds nothing.

**Location:** crates/btctax-core/src/tax/scrub.rs:92-97 (the KEPT block) vs :294-332 (the test) and crates/btctax-core/src/tax/testonly.rs:223-231/240/413/468 (the fixtures)

**Failure:** `kitchen_sink_household`, `w2_only_household` and `amt_owing_household` all build people through `person()` (testonly.rs:223-231), which is `..Default::default()` for `date_of_birth`, `date_of_death` and `blind`; `answer_all_live_declarations` iterates FORM_QUESTIONS only, and `blind` lives in SKIPPABLE_QUESTIONS (questions.rs:740-772), so it stays `None` too. Plant `date_of_birth: None` in `scrub_person` — a plausible hardening edit, since a date of birth is PII under every standard definition — and the suite stays green, because `None` replaced by `None` is a no-op on every fixture. Ship it and a 66-year-old filer's shared return silently loses the §63(f) aged addition ($1,950 single / $1,550 per box married) and its 1040 line 12a checkbox. Same for `blind`: `Some(false)` and `None` give the identical standard deduction, so only the print boundary distinguishes them — and no test crosses it (see the Critical). This is the repo's own B1 rule unmet: the guarantee has no test that reds when it is removed.

**Fix:** Add a fourth case to the loop: a household with `date_of_birth: Some(1955-03-02)`, `blind: Some(true)`, and a `date_of_death` + `taxpayer_died_during_year` pair, so every KEPT field is load-bearing on at least one vector and dropping it reds `standard_deduction`.

<details><summary>Verifier reasoning</summary>

CONFIRMED against source on every point.

FIXTURES: testonly.rs:223-231 `person()` sets only first_name/last_name/ssn/occupation and takes `..Default::default()` for the rest. All three fixtures in `scrub_preserves_every_computed_figure` build people through it — kitchen_sink_household (:257-258), w2_only_household (:417), amt_owing_household (:475). Greps over testonly.rs: `date_of_birth` = 1 hit (:267, the DEPENDENT's DOB, not a Person's), `date_of_death` = 0 hits, `blind` = 2 hits and both are the param names std_aged_blind_married/_unmarried (:62-63), never a Person field. So taxpayer/spouse date_of_birth, date_of_death and blind are None on all three.

NO BACK-FILL: answer_all_live_declarations (testonly.rs:34-40) iterates FORM_QUESTIONS only; blind is in SKIPPABLE_QUESTIONS (questions.rs:740-772, BlindTaxpayer/BlindSpouse), as is the §G-9 death pair (questions.rs:731). Nothing populates them.

NO OTHER INSTRUMENT: scrub_pii has exactly three callers repo-wide — the three tests in scrub.rs and btctax-cli/src/cmd/tax.rs:176, which no test exercises (no test references IncomeCmd::Scrub or scrub_return_inputs). So the invariant test is the only thing over these fields, and it compares None == None three times. Planting `date_of_birth: None` in scrub_person at scrub.rs:95 reds nothing.

HARM IS REAL AND CORRECTLY SIZED: return_1040.rs:50-58 — is_aged returns false immediately on `dob: None`, so the §63(f) aged addition and the 1040 line-12a box disappear. ty2024_params (testonly.rs:62-63) confirms the $1,950 unmarried / $1,550 per-box married figures the finding cites. For `blind`, Some(true)->None likewise moves the deduction; a fixture carrying Some(false) would not discriminate, which is exactly why none of the current ones can.

WHY NOT REFUTED/DOWNGRADED: The ★★ comment at scrub.rs:92-94 states "Scrubbing them would ... break the invariant this module exists to guarantee." That sentence is false of the suite as written — the repo's own `a-figure-with-no-reader` shape (a doc asserting a property nothing enforces). The counterfactual edit is the single most likely future edit against this file (DOB is textbook PII; HIPAA Safe Harbor strips birth dates), so it is not an exotic mutation. I weighed downgrading to Minor on the grounds that nothing wrong ships today and crates.io permanence therefore does not bear on it, but rejected that: testonly.rs:443-446 documents this exact failure mode as already-burned ("a test that merely iterated those two would assert None == None twice and pass forever while the emitter was broken ... the vacuity was real and was caught by probing, not by reading"). amt_owing_household exists solely to kill that vacuity for AMT, and the identical vacuity has reappeared in the same test for the three fields the module itself flags as most at risk. Under CLAUDE.md's "a guarantee without a test that reds when it is removed does not exist" and B1 seen-red-once, this is a missing case = Important, and the fix (populate DOB/DOD/blind on one fixture, or extend the_identity_does_not_survive across the boundary) is a few lines, so holding the gate costs near nothing.

Not a restatement of any SETTLED FACT (1-6 cover business_description, foreign_country_names, exhaustive destructuring, naics_code, IP PIN, EIN sameness, SSN middle group, and the figures-are-sensitive disclaimer — none touch fixture coverage of the KEPT Person fields).

</details>


### [IMPORTANT] `the_identity_does_not_survive` names 4 of ~15 identity fields; the SPOUSE — a whole `Person` of PII present in the primary fixture — is not asserted at all, so a leak there passes both tests.

**Location:** crates/btctax-core/src/tax/scrub.rs:370-422 (the test) vs :140 (`spouse: spouse.as_ref().map(...)`)

**Failure:** The test asserts only `taxpayer.ssn`, `taxpayer.last_name`, `address_street`, `ip_pin`, the dependents' SSNs, `business_description` and `foreign_country_names`. Plant `spouse: spouse.clone()` at scrub.rs:140 and both tests stay green — the taxpayer assertions still hold, and `scrub_preserves_every_computed_figure` is indifferent because names move no figure (that is the test's own stated rationale at scrub.rs:367-368). `kitchen_sink_household` carries a real spouse (Jane Doe, 987-65-4321, Architect), so the path is exercised on every run and asserted on never. Also unasserted anywhere: `first_name`, `occupation`, `address_city/state/zip`, dependent NAMES, and all five name loops (`w2s[].employer`, `int_1099/div_1099/b_1099/g_1099[].payer`) — only the EIN is covered, by a different test. The compile-time destructure at scrub.rs:120-137 catches a NEW field; it does not catch an existing field being bound and passed through.

**Fix:** Replace the hand-list with a structural assertion: collect every `String`/`Option<String>` in the original (serialize both to JSON and walk), and assert no non-empty original string value appears anywhere in the scrubbed tree except the ones explicitly whitelisted as computational (`naics_code`, `relationship`). Then a leak in a field nobody named still reds.

<details><summary>Verifier reasoning</summary>

CONFIRMED. Every element checks out against source, and the mutation-survival argument is closed, not merely plausible.

**The test really does omit the spouse.** `the_identity_does_not_survive` (crates/btctax-core/src/tax/scrub.rs:370-422) asserts exactly: `taxpayer.ssn` ≠, `taxpayer.last_name` ≠, `address_street` ≠, `ip_pin == None`, per-dependent `ssn` ≠ / `relationship` ==, `business_description == "Example business"`, `foreign_country_names` lacking the two real countries + count 2, and `taxpayer.ssn.contains("-00-")`. There is no assertion mentioning `spouse` anywhere in the file.

**The PII is real and exercised on every run.** crates/btctax-core/src/tax/testonly.rs:258 — `spouse: Some(person("Jane", "Doe", "987-65-4321", "Architect"))` in `kitchen_sink_household`, the fixture both tests use.

**The plant provably survives.** I verified the full field list of `AbsoluteReturn` (crates/btctax-core/src/tax/return_1040.rs:1194-1435): it carries no name, SSN, or address for taxpayer, spouse, or dependents — every field is a figure, flag, enum, or sub-part struct, and the only identity strings that reach it (`printed_inputs.schedule_c_header.business_description`, `f8995a…col_a_name`) are already zeroed on both sides by `blank_identity` at scrub.rs:322-327. So `scrub_preserves_every_computed_figure` cannot see any header identity string at all. Planting `spouse: spouse.clone()` at scrub.rs:140 makes the two sides *more* equal in a field that comparison never reads → green; `the_identity_does_not_survive` makes no spouse assertion → green; `ein_distinctness_is_preserved_exactly` touches only `w2s` → green. All three pass today (`cargo nextest run -p btctax-core scrub::` → 3 passed), so the premise holds. No file was modified.

**Not handled elsewhere.** `grep -rn scrub_pii crates/` returns only scrub.rs:300/355/373/396 and crates/btctax-cli/src/cmd/tax.rs:176 — there is no CLI-level, snapshot, or integration test of the scrubbed output. The compile-time guard does not help: `scrub_header`'s destructure (scrub.rs:120-137) *binds* `spouse`, and `spouse.clone()` uses it, so it compiles; `scrub_person`'s exhaustive `Person` destructure (scrub.rs:79-87) only fires when a new `Person` field is added, never when the call is bypassed. The finding states this distinction correctly.

**Ancillary claims verified too:** `first_name`, `occupation`, `address_city/state/zip`, dependent NAMES, and all five name loops (`w2s[].employer`, `int/div/b/g_1099[].payer`, scrub.rs:259-276) are asserted by nothing; only the EIN is covered, by a separate test.

**Not a restatement of a SETTLED FACT.** Fact 1 closed a missing top-level destructure; this is about test discrimination, and the brief explicitly asks "Would `the_identity_does_not_survive` catch a leak in a field it does not name?" — the answer is no, for roughly two thirds of the identity surface including an entire nested `Person`.

**Severity: IMPORTANT, not CRITICAL.** No identity survives today — scrub.rs:140 is correct — so there is no live disclosure and this alone would not make a shipped crate leak. But under the brief's own definition (Important = real defect, missing case, unsound assumption) this is a missing case in the *only* instrument holding the disclosure half of the guarantee, in a module whose `--help` (crates/btctax-cli/src/cli.rs:416-418) promises the user "Names, SSNs, the address … are replaced" — plural, i.e. the spouse's — and which is about to be permanently published. The module's own doc comment (scrub.rs:367-368) states the exact rationale the test then fails to honour: "a scrubber that quietly kept a field would still pass the invariant above, because names move no figure. Both halves are needed." The second half covers a third of the fields. The concrete harm is forward-looking and cheap to foreclose: any future edit to `scrub_header` that keeps a spouse field (or a refactor that drops the `.map(scrub_person)`) lands fully green, and the resulting file carries the user's "safe to hand to a stranger" authorisation onto a real SSN. Fix is ~6 assertion lines against `s.header.spouse`, plus the unasserted taxpayer/dependent/payer names.

</details>


### [IMPORTANT] Each dependent's EXACT date of birth is carried into the "safe to hand to a stranger" file, and the doc comment's justification for keeping it — "the DOB decides qualifying-child age. Both are read" — is false: nothing reads it.

**Location:** crates/btctax-core/src/tax/scrub.rs:112-115

**Failure:** Grepped every reader of `date_of_birth` in the workspace: the only consumers are `AgedBlindBoxes` (packet.rs:306/314), the advisories and the questions registry — all three read the TAXPAYER's and SPOUSE's, never a dependent's. `DependentRow` (packet.rs:423-427) carries name/ssn/relationship only; `dependents_statement.rs` prints the same three; `ctc_odc_credit` is a hardcoded 0. The only writer is the input-form wizard (`FieldId::DepDob`). VERIFIED in the emitted artifact: `btctax income scrub` on the kitchen-sink fixture writes `[[header.dependents]] date_of_birth = [2012, 106]` — a named child's exact date of birth — into a file the CLI has just told the user is safe to send. Removing it would move no figure on any vector, so the stated trade (identity retained because the computation needs it) is not being made; the taxpayer's and spouse's DOBs genuinely are computational, this one is pure retention. Exact DOB is one of the three classic quasi-identifiers, and the ZIP that would normally accompany it has been scrubbed precisely because the module accepts that reasoning.

**Fix:** Drop it (`date_of_birth: None`) — or, if a future CTC implementation will read it, quantise to the birth YEAR — and correct the comment, which currently instructs the next maintainer that it is load-bearing when it is not.

<details><summary>Verifier reasoning</summary>

CONFIRMED, with the harm construction corrected (the finding overstates it in one specific way).

## The central claim is true and I can point at every piece

**The retention and its stated reason** — `crates/btctax-core/src/tax/scrub.rs:112-116`:
```rust
// ★ KEPT: relationship decides child-vs-other-dependent, and the DOB decides qualifying-child
//   age. Both are read; only the name and SSN are not.
relationship: relationship.clone(),
date_of_birth: *date_of_birth,
```
The module table at `scrub.rs:31-37` reinforces it, listing "dates of birth and death" in the column headed **"preserved — the computation reads it"**.

**Nothing reads a dependent's DOB.** I grepped every `date_of_birth` occurrence in the workspace and walked each non-test hit:
- `packet.rs:306,314` (`AgedBlindBoxes::for_return`) — `t.date_of_birth` / `s.date_of_birth`: taxpayer and spouse only.
- `advisories.rs:771,780,793,922` — taxpayer/spouse only.
- `questions.rs:804,820` — `header.taxpayer` / `header.spouse` only.
- `DependentRow` (`packet.rs:423-427`) carries `name`, `ssn`, `relationship` — no DOB. Its printed counterpart `DependentRowCells` (`btctax-forms/src/map.rs:479-489`) is `name/ssn/relationship/ctc/odc`; the real 1040 dependents table has no DOB column, so this is correct — and it means the DOB is read by *nothing, not even the printer*.
- `dependents_statement.rs:122-131` `render()` prints the same three fields plus two empty check cells.
- `return_1040.rs:1837`: `let ctc_odc_credit = Usd::ZERO;` — the §3.4 conservative omission. And `advisories.rs:442-443` says so explicitly: *"btctax does not compute the credit and **does not know which dependents are under 17**"*. `ctc_provably_zero` deliberately bounds line 8 at `dependents × $2,000` precisely *because* it cannot read ages.
- `classifier.rs:329-337` `classify_dependent` destructures `date_of_birth: _` with the comment *"No classifiable leaves"*.
- The only get/set is the wizard field `FieldId::DepDob` (`btctax-input-form/src/spec/sections.rs:565-587`, `live: |_| true`) plus `income show`'s display formatter (`btctax-cli/src/cmd/tax.rs:232`). A field that only round-trips itself through the editor is not "read" in the sense the comment claims — by that standard `name` and `ssn` are read too, and they are scrubbed.

So the sentence "the DOB decides qualifying-child age. Both are read" is **false in the present tense**, and it is the sole recorded justification for keeping a child's exact date of birth.

**It does reach the shared file.** `scrub_return_inputs` (`btctax-cli/src/cmd/tax.rs:169-183`) is `toml::to_string_pretty(&scrub_pii(&ri))` — the whole struct, no masking (unlike `show_return_inputs`, which routes through `mask_pii` + `format_dobs_readable`). The committed fixture `crates/btctax-cli/tests/fixtures/examples/fullreturn_inputs.toml:59-60` shows the exact bytes: `[[header.dependents]]` / `date_of_birth = [2012, 106]`.

## The decisive argument is internal inconsistency, not re-identification math

`scrub.rs:143-146` replaces `address_state` with this reasoning: *"btctax computes no state tax, so **nothing reads it** — but a state plus a filing status plus an income is a long way toward identifying a household."* That is the module's own rule: *nothing reads it + contributes to identification ⇒ replace*. A dependent's DOB satisfies both halves identically, and got the opposite treatment on a premise that is factually wrong. Same file, ~30 lines apart.

## Where the finding overstates, and I am correcting it

- **"a named child's exact date of birth" is wrong.** `scrub.rs:110` sets `name: format!("Dependent{n}")` and `ssn: synthetic_ssn(100 + n)`. The child is not named in the output.
- **"one of the three classic quasi-identifiers" is accurate but the other two are absent.** Sweeney's triple is {5-digit ZIP, gender, full DOB}; the ZIP, state, street, city, names and SSNs are all replaced, and btctax never records gender. A bare DOB is shared by roughly ten thousand living Americans; it is not by itself a re-identifier.
- The adults' DOBs are retained anyway (correctly, §63(f)), so the marginal lift from a third DOB is smaller than the finding implies.

## Why it still blocks

The residual harm is real and cheap to remove. The intended use is posting the file to a maintainer or a public issue; a filer identifiable by their GitHub account thereby publishes their minor child's exact DOB alongside relationship, both parents' DOBs, and the exact income profile — and DOB is the second of the three fields (name, DOB, SSN) that make child identity theft a known vector, with the name trivially recoverable from a named parent. Against that: the fix moves no figure. Nothing reads the field, nothing refuses on it, and `scrub_preserves_every_computed_figure` and `the_identity_does_not_survive` would both stay green if it were replaced (e.g. keep the birth **year**, zero the month/day — that preserves any future qualifying-child age test and the wizard's answered-ness while dropping the exact date).

It clears the Important bar as an **unsound assumption** that drove a disclosure decision, not as a demonstrated identity leak. The aggravating factor is the publish: `crates.io` is permanent, and what ships is not just the field but a committed sentence asserting the computation needs it — which is the thing that stops the next reader from re-examining it. Note also that this is not settled fact #6: the claim is not "the figures are sensitive", it is "a field was retained on a stated reason that is false, in violation of the rule the same module applies to `address_state` twenty lines earlier."

Two adjacent observations, not findings under this brief: the `--help` at `cli.rs:410-412` ("Everything the computation reads survives unchanged: ... dates of birth and death") is a survivorship promise and stays literally true; and `FieldId::DepDob` has `live: |_| true`, so the wizard prompts every filer with a dependent for a datum nothing consumes — a collection-minimization question that belongs to the input-form surface, outside this brief's five files.

</details>


### [MINOR] The compile-time completeness guard covers 4 of the 10 structs the scrubber touches — and the six it misses are exactly the ones holding the free text it has to replace

**Location:** crates/btctax-core/src/tax/scrub.rs:12-19 (the claim) vs :241 (`ri.clone()`), :245-255 (ScheduleCInputs), :259-264 (W2), :265-276 (Form1099Int/Div/B/G)

**Failure:** The module doc asserts "Every struct touched here is taken apart with **no `..`** — `ReturnInputs` itself included", and the whole disclosure guarantee rests on it. Verified false: only `Person` (:79-87), `Dependent` (:103-108), `HouseholdHeader` (:120-137) and `ReturnInputs` (:205-239) are destructured. `ScheduleCInputs`, `W2`, `Form1099Int`, `Form1099Div`, `Form1099B` and `Form1099G` are mutated field-by-field on the clone at :241 with no pattern at all, and `Box12Entry` is never touched. That is the same clone-plus-targeted-mutation shape that let `business_description` and `foreign_country_names` ride through v1 of this module — moved down one level and left in place, in the six types whose free text (`employer`, four `payer`s, `business_description`) is the payload. The next PII field added to any of them ships silently into a file the tool calls safe to hand to a stranger, and the near-term candidates are concrete and uncollected today: W-2 box c "Employer's name, address, and ZIP code", boxes e/f (employee name and address), 1099-INT "PAYER'S TIN" and "Account number" — while this repo's standing rule is "if the form asks something our input surface cannot answer, collect it." Publication is permanent, so a leak added later still arrives under this command's "safe to share" authorisation.

**Fix:** Give each of the six the same treatment as `Person`: build the replacement from an exhaustive `let W2 { owner, employer: _, ein, box1_wages, … } = w;` pattern with no `..`, so a new field is a compile error that forces a classification. `ScheduleCInputs` is one line away already — :245 binds `Some(_sc)` and discards it.

<details><summary>Verifier reasoning</summary>

CONFIRMED ON THE FACTS, SEVERITY INFLATED ONE NOTCH.

Every citation verifies exactly. scrub.rs:14 states "Every struct touched here is taken apart with **no `..`** — `ReturnInputs` itself included." That sentence is literally false: the destructures are `Person` (:79-87), `Dependent` (:103-108), `HouseholdHeader` (:120-137) and `ReturnInputs` (:205-239) only. :241 is `let mut out = ri.clone();` and :245-276 mutate `ScheduleCInputs`, `W2`, `Form1099Int`, `Form1099Div`, `Form1099B`, `Form1099G` field-by-field on that clone with no pattern (the `if let (Some(sc_out), Some(_sc))` at :245 binds `_sc` unused — it is not a destructure). `Box12Entry` is indeed never touched. The six unguarded types do hold every remaining free-text field (`employer`, four `payer`s, `business_description`).

The blast-radius mechanism is real and I checked it end to end: `scrub_pii` has exactly ONE caller (btctax-cli/src/cmd/tax.rs:176-177), which does `toml::to_string_pretty(&scrubbed)` over the whole struct, so any newly-serialized field is emitted verbatim; the only tests are the three in-module ones, and `the_identity_does_not_survive` asserts on named fields only, so it would not red. Nothing else in the repo enumerates these fields — `scripts/pii-scan-generic.sh` greps COMMITTED files for SSN/EIN digit shapes and cannot see a scrub output at all.

The near-term-addition argument is better supported than the filer argued. Their own examples are weak: btctax fills neither a W-2 nor a 1099-INT (those are input documents, so box c/e/f and PAYER'S TIN are not on its emit path). The strong case is one they missed — `design/forms/extract/f1040sc--2024.txt` shows Schedule C line C "Business name", line D "Employer ID number (EIN)" and line E "Business address (including suite or room no.)", all three identity-bearing, all three uncollected today, and `design/forms/FIELD_PROVENANCE.md` (written 2026-07-30) records f1040sc as 105 fields / 13 mapped / 92 unaccounted, with the stated direction being to account for them. Schedule C IS a form btctax emits, and those three lines land in `ScheduleCInputs` — one of the six unguarded structs. So the concrete harm is a maintainer who adds `business_address` to `ScheduleCInputs`, reads :14, believes the compiler will stop them, and gets no error.

Why MINOR and not IMPORTANT. I enumerated every field of all seven types against the scrubber: `W2` (owner/employer/ein/box1-19/box12), `Form1099Int`, `Form1099Div`, `Form1099G`, `Form1099B`, `ScheduleCInputs` (owner, business_description, naics_code, accounting_method, expenses, other_gross_receipts, payments_requiring_1099, will_file_required_1099, qbi_w2_wages, qbi_ubia, is_sstb, is_cooperative_patron) and `Box12Entry` (code/amount). Every identity-bearing one is scrubbed today; the residue is `naics_code` (settled fact #2) and a W-2 box-12 letter code. There is NO input at HEAD for which a filer's identity survives through these six — the brief's ONE question answers "no" here. The filer's permanence argument is also misapplied: the crate published now carries no leak, and a PII field added later ships in a LATER publish that gets its own review; the `--help` authorisation persisting does not make today's tarball the vector. So this is a false sentence in a doc comment plus absent defense-in-depth, not an unmet output guarantee — the user-facing promise in cli.rs:415-417 ("Names, SSNs, the address, employer and payer names are replaced") is true as written. Worth fixing inline (add the six destructures, ~40 lines, and correct :14 to say what it actually covers), but it should not block a publish that leaks nothing.

</details>


### [MINOR] The invariant test's load-bearing comment — `business_description` is "the ONLY identity-bearing string that survives into `AbsoluteReturn`" — is false, and the counterexample is the EIN the scrub deliberately remaps

**Location:** crates/btctax-core/src/tax/scrub.rs:305-309 and :322-327; vs crates/btctax-core/src/tax/return_1040.rs:692-699, :1338

**Failure:** `AbsoluteReturn.excess_ss_not_creditable: Vec<NonCreditableSs>` carries `ein: String`, and `scrub_pii` remaps EINs by design — so original and scrubbed differ on that field whenever any single employer's box-4 withholding exceeds the §3101(a) cap. `scrub_preserves_every_computed_figure` passes today only because none of its three fixtures carries an EIN at all, which is the same blind spot that hides the CRITICAL above. The first fixture added with a stranded over-cap employer reds the invariant for a CORRECT scrub, and the comment tells the next author that such a red means a real bug.

**Fix:** State the true set (the Schedule C header pair, Form 8995-A `col_a_name`, and `excess_ss_not_creditable[].ein`) and normalise the EIN in `blank_identity` by comparing canonical-set SHAPE rather than value — while keeping the assertion that the distinct-employer COUNT is unchanged.

<details><summary>Verifier reasoning</summary>

CONFIRMED at MINOR. Every load-bearing claim checks out against source.

1. `NonCreditableSs` (return_1040.rs:692-699) has `pub ein: String`, and `AbsoluteReturn.excess_ss_not_creditable: Vec<NonCreditableSs>` (return_1040.rs:1338) is inside the struct the invariant test compares with `assert_eq!` (AbsoluteReturn at :1194 derives PartialEq). So a String EIN does reach `AbsoluteReturn`.

2. An EIN is identity by this module's own standard: `scrub_pii` deliberately remaps it (scrub.rs:258-264) and `ein_distinctness_is_preserved_exactly` asserts "the real EIN must not survive" (scrub.rs:363). Therefore the comment at scrub.rs:305-309 — "`business_description` is the ONLY identity-bearing string that survives into `AbsoluteReturn` … every other field there is a figure, a flag or an enum" — is factually false, and `ein` is the counterexample. (`printed_inputs.schedule_c_header.naics_code` is a second String, but settled fact 2 rules it non-identity, so the EIN is the live one.)

3. The trigger is exactly as alleged: `non_creditable_ss` (:714-736) buckets `box4_ss_withheld` by `canonical_ein` per Owner and emits an entry when one bucket exceeds `ss_wage_base * EMPLOYEE_OASDI_RATE` ($168,600 x 0.062 = $10,453.20, tables.rs:140, testonly.rs:214). `assemble_absolute` calls it on whichever `ri` it is handed (:1888, stored :1949), so `a` carries the real canonical EIN and `b` the synthetic one, and `a != b` for a correct scrub.

4. The vacuity is real: none of the three fixtures (`kitchen_sink_household` testonly.rs:240, `w2_only_household` :413, `amt_owing_household` :468) sets `ein` at all — grep for "ein" in testonly.rs returns a single unrelated prose hit — so `non_creditable_ss` returns empty on all three and the field is never actually compared. Kitchen sink's taxpayer box 4 is exactly dec!(10453.20), i.e. equal to the cap, not over it (`>` is strict), so even adding an EIN there would not fire. `scrub_pii` has exactly one non-test caller (btctax-cli/src/cmd/tax.rs:176); no test anywhere else covers this.

Not already handled elsewhere; not unreachable (a single employer over-withholding is precisely the case the whole NonCreditableSs feature was built to disclose). Severity is NOT inflated and NOT understated: there is no disclosure consequence — the scrubbed AbsoluteReturn and its `Advisory::ExcessSsNotCreditable` (advisories.rs:706-714) carry only the synthetic EIN — and no shipped figure moves, so this is a false statement in an explicitly load-bearing comment plus a latent false-red in the invariant test. MINOR, non-gating. It does not restate any settled fact.

One correction to the fix as implied by the filer: blanking `ein` inside `blank_identity` is insufficient. `non_creditable_ss` emits entries in `BTreeMap` key order (canonical EIN), while `EinMap` assigns synthetic EINs in W-2 vector order — so with two over-cap employers for one person whose real EINs sort differently from their W-2 order, the (ein, amount) pairs are permuted and the vectors still differ after blanking. The normalisation must blank and re-sort (e.g. by owner+amount), or the scrub must preserve EIN sort order.

</details>


### [MINOR] The `--help` and the success message promise fidelity the code does not deliver

**Location:** crates/btctax-cli/src/cli.rs:413-425; crates/btctax-cli/src/main.rs:312-317

**Failure:** "Everything the computation reads survives unchanged … and every fail-loud declaration" is false for the four screens in finding 2. "it computes an IDENTICAL return — a guarantee held by a test, not a hope" is false in the two-spelling EIN case, where the return moves $1,546.80 — and the test named as the guarantee is the one that cannot see it. "Every figure is preserved" in the `--out` message repeats it. Separately, `business_description` IS read by the computation (it reaches `AbsoluteReturn.printed_inputs.schedule_c_header` and Form 8995-A line 1(a)) and IS changed, so the scrubbed copy's filed Schedule C line A prints "Example business". The safety half of the help (what is removed) is accurate; it is the fidelity half that overstates.

**Fix:** After fixing findings 1 and 2, keep the wording; until then, say what is true — every figure the AbsoluteReturn computes is preserved, and name the identity strings that are replaced on filed pages (Schedule C line A, Form 8995-A line 1(a)).

<details><summary>Verifier reasoning</summary>

CONFIRMED at MINOR — verified against source, though part of it is derivative of other findings.

**What I verified (exact source):**

1. **`business_description` IS read by the computation and IS changed — so the "identical return" claim is false by design, permanently.**
   - `crates/btctax-core/src/tax/scrub.rs:251` sets `sc_out.business_description = "Example business"`.
   - It reaches the computed struct at `crates/btctax-core/src/tax/return_1040.rs:1981` (`printed_inputs.schedule_c_header.business_description`) and, via `return_1040.rs:1734-1741` → `crates/btctax-core/src/tax/qbi_a.rs:377` (`col_a_name: i.business_name.clone()`), Form 8995-A Part I col (a).
   - Both are FILED, not internal: `crates/btctax-core/src/tax/packet.rs:599` prints Schedule C line A; `crates/btctax-forms/src/form8995a.rs:145` writes `col_a_name` into the AcroForm.
   - The guarantee test itself concedes this — `scrub.rs:322-327` blanks both sites on both sides before comparing.
   So `--help`'s "Everything the computation reads survives unchanged" and "it computes an IDENTICAL return" are literally false for a Schedule C filer, and the scrubbed copy's Schedule C line A does print "Example business".

2. **The help text is STALE, not merely imprecise — and that is independently checkable.** `git show 2449ee4 --stat` shows the fix commit that introduced the `business_description` and `foreign_country_names` scrubbing touched **only** `crates/btctax-core/src/tax/scrub.rs` (166 lines, one file). The `--help` prose in `cli.rs:413-428` was authored in the earlier commit `31d5c79`, before those two fields were scrubbed, and its enumeration of what is replaced ("Names, SSNs, the address, employer and payer names") was never swept. This is exactly the repo's own "whole-surface sweep on taxonomy change" class.

3. **It has already propagated to a shipped artifact.** `docs/man/btctax-income-scrub.1` carries the identical stale paragraphs verbatim (generated from the `cli.rs` doc comment). The fix is therefore `cli.rs` **plus** regenerating the man page — an edit to `cli.rs` alone leaves the false text in a committed, distributed file, and this is immediately before a permanent crates.io publish.

4. **The EIN sub-claim's mechanism is real, and the named test genuinely cannot see it.** `EinMap::map` (`scrub.rs:68-74`) keys on `real.to_string()` — the RAW spelling — while the computation compares `canonical_ein` (`return_1040.rs:681-687`, strips `-`/whitespace). So `11-1111111` and `111111111` (one employer) become two distinct synthetic EINs, `synthetic_ein(0)="90-0000001"` and `synthetic_ein(1)="91-0000002"`, both of which canonicalise to 9 valid digits and are distinct → `excess_social_security` (`return_1040.rs:795`) goes from `eins.len() < 2` (credit $0) to a credit of `min(box4,10453.20)*2 - 10453.20`. And `grep -n "ein" crates/btctax-core/src/tax/testonly.rs` returns **no matches**: all three fixtures used by `scrub_preserves_every_computed_figure` carry `ein: None`, so the guarantee test never exercises `EinMap` at all. Caveat on the number: $1,546.80 is probe-dependent (box 4 = $6,000 each); the in-repo probe at `return_1040.rs:776` cites $1,946.80 for $6,200 each. The mechanism is right; the dollar figure is only right for their specific probe.

**Why not higher, why not lower.** Not Important: the money-moving half is owned by the EIN finding and by the screens finding — counting the help text again as blocking would double-charge one defect at the gate, and the help text moves no figure on its own. Not a Nit: the sentence is a fidelity promise a filer relies on when deciding to send a file, one third of it (`business_description`) is false permanently and by design rather than contingently on the other bugs being fixed, and the false text is already committed to a man page about to be published permanently.

**Not a restatement of a settled fact.** Settled fact 1 closes the *leak*; this finding is that the *help was never updated when the leak was closed* — a different artifact, and precisely the "look for what that pass missed" the brief asks for. The finding's own concession that the safety half ("what is removed") is accurate also checks out: `scrub.rs:88-99, 138-164` replace names, SSN, occupation, all four address fields, and drop the IP PIN — if anything the help understates what is removed, which is the safe direction.

</details>


### [MINOR] Every identity-shaped print-boundary refusal disappears: a missing/malformed SSN and a malformed IP PIN both become valid under scrubbing

**Location:** crates/btctax-core/src/tax/scrub.rs:91 and :111 (ssn: synthetic_ssn(n)) and :164 (ip_pin: None); vs crates/btctax-core/src/tax/packet.rs:208, :425, :451-456 (ReturnHeader::build -> Ssn::canonical / IpPin::canonical)

**Failure:** synthetic_ssn always yields nine digits and the IP PIN is dropped, so both fail-closed PRINT gates open. Confirmed: taxpayer ssn = "" gives original Err(HeaderError::Ssn(Missing)) -> scrubbed Ok (header BUILT); ip_pin = Some("12345") gives original Err(Ssn(WrongLength(5))) -> scrubbed Ok. This is not exotic: HouseholdHeader::taxpayer is #[serde(default)] because 'the taxpayer's PII is captured LATER (btctax set-pii)' (return_inputs.rs:234-240), and screen_inputs deliberately has no SSN gate (return_refuse.rs:693-698) -- so the DEFAULT state of a freshly-imported return refuses at export and exports cleanly once scrubbed. The whole class of 'btctax will not emit my packet' reports is structurally unreproducible from a scrubbed copy. Mitigating: HeaderError's Display is self-diagnosing, so the filer usually fixes it without reporting.

**Fix:** Preserve the SHAPE rather than the value -- emit String::new() for an SSN that fails Ssn::canonical, and a dropped-but-still-invalid marker (e.g. Some(String::new())) for an IP PIN that fails IpPin::canonical, so the gate that fired still fires. If that is judged not worth the complexity, amend the --help: the promise 'so they can reproduce a refusal' is not true of identity-shaped refusals.

<details><summary>Verifier reasoning</summary>

CONFIRMED as stated, at MINOR. Every mechanical step checks out against source, and the harm is real but narrow.

**The mechanism, verified line by line**
- `scrub.rs:53-55` — `synthetic_ssn(n) = format!("1{:02}-00-{:04}", …)` is unconditionally 3+2+4 = nine digits, for every `n`. It is applied unconditionally at `scrub.rs:91` (taxpayer/spouse) and `:111` (dependents); it never inspects the input SSN.
- `packet.rs:56-73` — `Ssn::canonical` strips whitespace and `-`, then `Err(SsnError::Missing)` on empty, `NotDigits(c)`, `WrongLength(n)`. `"101-00-0002"` → `"101000002"` → `Ok`. So a scrubbed household **always** canonicalizes.
- `packet.rs:208` + `:397` + `:425` — `FiledPerson::build` and the dependent loop both `?` on `Ssn::canonical`, converting via `From<SsnError> for HeaderError` (`:133`). `Person::ssn` is a bare `String` with `#[serde(default)]`, so `ssn: ""` gives `Err(HeaderError::Ssn(Missing))` on the original and `Ok` on the scrubbed copy. Confirmed.
- `packet.rs:451-456` vs `scrub.rs:164` — `ip_pin: Some("12345")` → `IpPin::canonical` (`:167-179`) → `WrongLength(5)` → `HeaderError::Ssn(WrongLength(5))`; scrubbed `ip_pin: None` → `.map(..).transpose()` → `Ok(None)`. Confirmed.

**It is reachable, not exotic** — `return_inputs.rs:239-240` makes `taxpayer` `#[serde(default)]` precisely because "the taxpayer's PII is captured LATER", and `return_refuse.rs:693-698` documents that `screen_inputs` deliberately carries **no** SSN gate ("The identity boundary is the FILABLE PACKET"). There is in fact no `set-pii` subcommand in `cli.rs` at all — only the TUI editor validates (`parse_ssn`/`parse_ip_pin`), so a TOML-imported return reaches the header build unvalidated. A freshly imported return refuses at export by default and scrubs into one that exports cleanly.

**No test can see it** — `AbsoluteReturn` (`return_1040.rs:1194+`) is figures only; `ReturnHeader::build` is called from `packet.rs:553`, outside it. So `scrub_preserves_every_computed_figure` structurally cannot observe this boundary, and `the_identity_does_not_survive` only asserts the SSN *changed*. This lands on the brief's own bullet ("Does the scrub break a REFUSAL? … file where the original refused?") and is not any of the six settled facts — settled fact 3 blesses *dropping* the IP PIN, not the refusal it dissolves.

**It contradicts a stated promise** — `cli.rs:415-416` (and the generated `docs/man/btctax-income-scrub.1`): "so they can **reproduce a refusal** or a wrong figure without receiving your PII." For the identity-shaped refusals that promise is false, and no decision record exists anywhere (grepped FOLLOWUPS.md and design/). That is exactly the failure `scrub.rs:190-202` was just fixed for ("the doc asserted a property nothing enforced").

**Why MINOR and not higher** — three real discounts, and the reviewer named only one:
1. The finding's sentence "the whole class of 'btctax will not emit my packet' reports is structurally unreproducible" is **overstated**. `HeaderError::Unanswered` (`packet.rs:386-390`) survives — `scrub_header` copies all eight declaration flags verbatim (`:153-160`) and the `ReturnInputs`-level answers are cloned. `HeaderError::MfjWithoutSpouse` (`:393`) survives — `spouse.as_ref().map(..)` keeps `None` as `None`. Every `screen_inputs` `RefuseReason` survives by deliberate design (the non-empty `business_description`, the preserved country count). Only the identity subclass vanishes.
2. Those are the self-diagnosing ones ("no SSN was entered — fix the identity and re-run"; "has 5 digits — an SSN has exactly 9"), so they rarely become bug reports.
3. **The obvious fix would make the tool worse.** A shape-preserving scrub (empty → empty) would make the *most common* scrubbed return — the freshly imported, PII-less one — non-exportable, blocking a maintainer chasing a figure bug through the PDF path. The current behaviour is defensible; what is missing is the recorded decision and an honest `--help`.

So: real, correctly cited, no leak, no figure moves, cheap remedy (qualify the `--help` sentence, and have `income scrub` print a note when `ReturnHeader::build` fails on the original but succeeds on the copy). MINOR is the right severity — the reviewer's own call was correct.

</details>


### [MINOR] `the_identity_does_not_survive`'s only dependents assertion is a `zip`, so a scrub that DROPS or truncates the dependent list asserts nothing — and the dependent count is absent from `AbsoluteReturn`, so the figure invariant cannot see it either.

**Location:** crates/btctax-core/src/tax/scrub.rs:381-387 (the `zip` loop) and :147-151 (the dependents map)

**Failure:** Plant `dependents: Vec::new()` at scrub.rs:147. `the_identity_does_not_survive` passes — `ri.header.dependents.iter().zip(&s.header.dependents)` yields zero iterations and every assertion inside is skipped. `scrub_preserves_every_computed_figure` passes too: `AbsoluteReturn` carries no dependents field, `standard_deduction` reads only `can_be_claimed_as_dependent_*`, and `ctc_odc_credit` is a hardcoded 0 (return_1040.rs:1298-1300). The whole suite stays green while the FILED 1040 page 1 loses its dependent rows, the "more than four dependents" checkbox (map.rs:471) flips, and `dependents_statement.txt` — an IRS-mandated attachment — vanishes from the packet. I confirmed all three artifacts are dependents-driven by exporting the nine-dependent fixture. Every one of them reaches a filed page and none is in `AbsoluteReturn`.

**Fix:** Assert the length first (`assert_eq!(s.header.dependents.len(), ri.header.dependents.len())`) before the `zip`, and use a multi-dependent fixture. A `zip` is never a coverage assertion — it is silent exactly when the collection it walks has been truncated.

<details><summary>Verifier reasoning</summary>

CONFIRMED as fact, DOWNGRADED on severity.

Every mechanical claim checks out against source:

1. scrub.rs:381 — `for (o, n) in ri.header.dependents.iter().zip(&s.header.dependents)`. `Iterator::zip` terminates on the shorter side, so with `s.header.dependents` empty the body never runs and both assertions (`assert_ne!(o.ssn, n.ssn)`, `assert_eq!(o.relationship, n.relationship)`) are skipped. Nothing else in `the_identity_does_not_survive` touches dependents, so it passes.

2. The figure invariant genuinely cannot see it. I read `AbsoluteReturn` in full (return_1040.rs:1194-1357) and `PrintedInputs` (1388-1435): no dependent count, no dependent rows. `assemble_absolute` never reads `header.dependents` — the sole `dependents` occurrence in return_1040.rs is a test at 5883. `ctc_odc_credit` is literally `let ctc_odc_credit = Usd::ZERO;` (1837), and `standard_deduction` keys off `can_be_claimed_as_dependent_*`, not the list. So `scrub_preserves_every_computed_figure` is structurally blind.

3. The downstream artifacts named are real and all live outside `AbsoluteReturn`: `ReturnHeader::build` builds the rows at packet.rs:418-429; form1040_full.rs:519 checks map.rs:471's `more_than_four_dependents` from the overflow split; btctax-forms/src/packet.rs:228-230 emits the `dependents_statement` attachment (btctax-cli/tests/export_irs_pdf.rs:200 confirms the `dependents_statement.txt` filename). advisories.rs:737 also reads the count.

4. "The whole suite stays green" is right: `scrub_pii` is invoked only at scrub.rs:300/355/373/396 and btctax-cli/src/cmd/tax.rs:176, and grep for "income scrub" across the tree finds no CLI test, journey, example or golden. Nothing else can red.

Not a restatement of a settled fact (settled fact 6 is about dependents being *sensitive*; this is about the list being *dropped*), and there is no structural guard: the exhaustive destructure in `scrub_header` forces classification when a FIELD is added, but does not hold the 1:1 mapping at scrub.rs:147-151 against a later edit.

Why MINOR rather than IMPORTANT: no live defect. Today's code preserves dependents exactly, and the synthetic SSNs pass `Ssn::canonical` (packet.rs:58-73, which only checks 9 digits), so the printed packet from a scrubbed return is correct as shipped. The harm requires a future edit — and a plausible one, since a disclosure-minded author could well decide a child's name and SSN should be dropped outright — but it is hypothetical at this gate. The finding's ALLEGED FAILURE section states the planted defect's consequences in the present tense ("the whole suite stays green while the FILED 1040 page 1 loses its dependent rows"), which reads as harm that exists now; it does not. What is actually established is a mutation-resistance gap on a guarantee the module documents but nothing holds (scrub.rs:36 lists "the NUMBER of dependents and each relationship" in the preserved column), plus the broader true observation that the distortion invariant's scope is `AbsoluteReturn` and therefore covers no printed-packet-only input. Worth fixing before publish because it is two lines — assert `s.header.dependents.len() == ri.header.dependents.len()` and use a >4-dependent fixture so the grid/checkbox/statement boundary is exercised — but it should not block the permanent publish on its own.

</details>


### [MINOR] The `--help` sells a guarantee "held by a test, not a hope" for the TOML round-trip; `scrub_return_inputs` has zero tests anywhere in the workspace.

**Location:** crates/btctax-cli/src/cli.rs:425-426 vs crates/btctax-cli/src/cmd/tax.rs:169-183

**Failure:** `grep -rn scrub` over `crates/` and `tests/` finds the emitter only at its definition and its dispatch — no unit test, no integration test, no journey golden. The test that does exist (`scrub_preserves_every_computed_figure`) never serialises to TOML and never re-imports; it compares two in-memory `AbsoluteReturn`s. So the promised half — "TOML that `income import` accepts" — is held by nothing. I hand-verified it works today (import → scrub → import → export gives field-identical PDFs on the 12-form nine-dependent packet; only the identity cells differ). The exposure is that `ReturnInputs` is `#[serde(default)]` on every field, so an emitter that ever drops or mangles a field re-imports it as a silent default rather than erroring — a distortion with no failure signal and no test that would red.

**Fix:** Add one CLI test: `scrub_pii(fixture)` → `toml::to_string_pretty` → `toml::from_str::<ReturnInputs>` → assert equal to the scrubbed value, over all three fixtures. That is the round trip the help text claims, and it reds if a future field breaks serialisation or is silently defaulted back.

<details><summary>Verifier reasoning</summary>

CONFIRMED, and every factual claim verified against source rather than accepted.

(1) No test exists. `grep -rn scrub` across the workspace finds `scrub_return_inputs` at exactly two sites: definition `crates/btctax-cli/src/cmd/tax.rs:169` and dispatch `crates/btctax-cli/src/main.rs:309`. All other "scrub" hits are unrelated words ("scrubbed HOME" in `crates/xtask/src/examples.rs:183`; `SecretBuf` "scrubs on drop" in `crates/btctax-store/src/vault.rs`). No `crates/btctax-cli/tests/*.rs` file exercises it; no xtask journey golden covers `income scrub`; `FOLLOWUPS.md` has no entry owning it.

(2) The existing test does not cover the round trip. `crates/btctax-core/src/tax/scrub.rs:293-332` (`scrub_preserves_every_computed_figure`) calls `scrub_pii` then `assemble_absolute` on both sides and compares two in-memory `AbsoluteReturn`s. It never touches `toml` and never re-imports. `ein_distinctness_is_preserved_exactly` and `the_identity_does_not_survive` are likewise in-memory.

(3) The promise is real and shipped. `crates/btctax-cli/src/cli.rs:425-426` reads verbatim: "The output is TOML that `income import` accepts, and it computes an IDENTICAL return — a guarantee held by a test, not a hope." Present in the built binary (`btctax income scrub --help`) and hence in the generated man page, and about to be published permanently.

(4) It works today — I reproduced this independently rather than trusting the filer. Using `target/debug/btctax` (built at HEAD 2449ee4), `import -> scrub -> import -> show` on three households: the nine-dependent AMT fixture, the kitchen-sink `fullreturn_inputs.toml` (MFJ, spouse, dependent DOB, Schedule A charitable array, Schedule C, multiple W-2s), and a hand-extended variant adding W-2 `box12` entries plus a `charitable_carryover_in` item — the nested array-of-tables-inside-array-of-tables shape no committed fixture covers. All three diff to ZERO non-identity differences; only names, SSNs, occupation, street, payer/employer and business_description move. So there is no live defect, exactly as the filer said.

(5) The silent-default exposure is correctly stated. `ReturnInputs` carries `#[serde(default)]` on essentially every field (`return_inputs.rs:644-826`), and `parse_return_inputs_toml` (`cmd/tax.rs:114-131`) uses `serde_ignored` to reject UNKNOWN keys — by construction it cannot detect a MISSING one, which silently takes the default.

The filer also understated their own best evidence. Two other places in this same codebase assert that struct-to-TOML serialization of `ReturnInputs` does not work: `crates/btctax-cli/tests/fullreturn_oracle.rs:40-42` ("Via `toml::Value::try_from` THEN string, NOT `toml::to_string(&ri)`: the latter fails `ValueAfterTable`") and `cmd/tax.rs:198-199` ("serde-toml requires scalar keys before nested tables, which the nested model violates"). `scrub_return_inputs` does precisely the thing those comments call impossible and succeeds only because toml 0.8 buffers into a document and hoists scalars above tables. That behavioral dependency is undocumented, contradicted by the repo's own comments, and observed by no test — a toml version bump could remove it with nothing going red. That is the concrete regression path the missing test would hold.

Severity stays MINOR, neither inflated nor deflated. Not IMPORTANT/CRITICAL: no filer is harmed today (verified lossless on the maximal household plus an extension), and the dominant regression path is LOUD, not silent — `toml::to_string_pretty(&scrubbed).map_err(|e| CliError::BadConfigValue{...})` at `cmd/tax.rs:177` surfaces a serialization failure as a CLI error rather than a corrupted file. The one genuinely silent path (a future field whose Serialize is lossy or skipped) is narrow, since serde emits every named field unless None or `skip`. Not NOT_A_DEFECT: the shipped `--help` states "a guarantee held by a test" and the round-trip half of that sentence is held by nothing, which is a false claim in user-facing documentation on a tool whose whole value proposition is trustworthiness — and it is the repo's own named recurring failure class (CLAUDE.md: "A guarantee without a test that reds when it is removed does not exist. Mutation-verify."). It does not gate the publish; it should be closed by a test that emits the TOML, re-parses it, and asserts equality with the in-memory scrubbed `ReturnInputs` (a fourth case in scrub.rs or a CLI-level test), which would also pin the toml-0.8 hoisting behavior the emitter silently depends on.

</details>


### [NIT] `w2s[].box12[].code` is the one String reachable from `ReturnInputs` with no recorded decision either way

**Location:** crates/btctax-core/src/tax/return_inputs.rs:29-32; absent from crates/btctax-core/src/tax/scrub.rs:31-37 (the kept/replaced table) and from the `--help`

**Failure:** Free text off a TOML import, carried through verbatim, and not classified anywhere. A W-2 box-12 code is a one- or two-letter code, so it is very unlikely to carry identity in practice — and preserving it is right, since `UnsupportedBox12Code` refuses on it and that refusal must survive. But under this file's own provenance rule the defect is the ABSENCE of a decision, not the value: every other reachable String is either replaced or has a written "KEPT because…" beside it.

**Fix:** Add one row to the kept/replaced table: kept, because `screen_inputs` refuses on it and a box-12 code is a taxonomy, not a person.

<details><summary>Verifier reasoning</summary>

CONFIRMED at NIT. Every factual component checks out against source. `Box12Entry.code: String` (return_inputs.rs:29-32) is reachable via `W2.box12` (line 76) from `ReturnInputs.w2s`; `scrub_pii` binds `w2s: _` (scrub.rs:209) under the legend "`_` = carries no identity", and the loop at scrub.rs:258-264 touches only `employer` and `ein`. `W2` and `Box12Entry` are never destructured in scrub.rs, so `code` has no compile-time witness and no per-field record. `out = ri.clone()` then `toml::to_string_pretty` (cmd/tax.rs:176-177) emits it verbatim. It is absent from the kept/replaced table (scrub.rs:31-37) and from the --help (cli.rs:413-427), both as claimed.

The uniqueness claim also holds. Complete String inventory reachable from `ReturnInputs`: box12.code, W2.employer, W2.ein, four `payer` fields, Person first/last/ssn/occupation, Dependent name/ssn/relationship, four address fields, ip_pin, business_description, naics_code, foreign_country_names. Every one except `code` is either replaced/dropped or carries an explicit "★ KEPT:" rationale (relationship at scrub.rs:112-113, naics_code at scrub.rs:252-254). So `code` is indeed the one reachable String with no decision recorded either way.

Severity is correctly filed as NIT and must not be raised. (a) No figure moves — `code` is read by the computation (return_refuse.rs:974-987: the INERT_BOX12_CODES allowlist refusal and the §402(g) elective-deferral aggregation), so preserving it is required, exactly as the finding concedes. (b) No promise is broken: the --help states "Everything the computation reads survives unchanged", which truthfully covers a field the computation reads; the --help is not an exhaustive enumeration, so its silence is not a false claim. (c) Disclosure risk is negligible: a return that FILES can only carry `{D,E,F,G,H,S,AA,BB,EE,DD}`; arbitrary text is reachable only on a refusing return (the case scrub exists for), and a filer would have to have typed identity into a two-letter code field. So the answer to the brief's ONE question is "no" in both directions.

What survives is a genuine but cosmetic provenance gap under this module's own doctrine: `naics_code` and `relationship` were "recorded as a decision rather than an oversight" and `code` was not. Worth noting for the fix: the gap is W2-wide, not code-specific — because `W2` is never destructured `..`-free in scrub.rs, a future PII field on `W2` (box 15 employer state ID, box f employee address) would ride straight through with nothing red, which is the same failure the module's "THE TOP LEVEL WAS THE ONE THAT MATTERED" note documents, one level down. Fixing it as a `..`-free destructure of `W2`/`Box12Entry` closes both the nit and that latent hole; a bare comment on `code` closes only the nit.

</details>


## Completeness sweep (new angles)

### [CRITICAL] The scrubbed TOML contains no LEDGER, so it cannot reproduce a btctax return at all — and `--help` calls the opposite "a guarantee held by a test, not a hope". The invariant test structurally cannot see this because it hands the SAME `LedgerState` to both sides.

**Location:** crates/btctax-cli/src/cmd/tax.rs:169-183 (emits only `ReturnInputs`); crates/btctax-cli/src/cli.rs:424-425 + docs/man/btctax-income-scrub.1 (the claim); crates/btctax-core/src/tax/return_1040.rs:1441-1447 (`assemble_absolute(ri, state: &LedgerState, …)`), :1511 (`compute_se_tax(state,…)`), :1580 (`crypto_income(state, year)`), :1650 (`crypto_charitable_gifts(state, year)`), :604-620 (`capital_net` → `schedule_d(state, year)`), :2002/:2013 (`digital_asset_activity(state, year)`); crates/btctax-core/src/tax/scrub.rs:295-303 (one `state`, both calls); crates/btctax-core/src/tax/testonly.rs:357-393

**Failure:** `scrub_pii` operates on `ReturnInputs`, which holds no lots, disposals or income records — every crypto figure comes from `LedgerState`, which `income scrub` never emits and `income import` cannot accept. Take the module's own kitchen-sink fixture: its `LedgerState` carries $20,000 of business mining income (→ Schedule C line 1 → the whole of Schedule SE) and a $20,000 long-term disposal (→ Schedule D). A filer with that shape scrubs, sends the TOML, and the recipient importing it into their own vault computes a return $40,000 light in AGI, with Schedule SE and Schedule D absent, the §170(e) crypto donation deduction gone, and the 1040 digital-asset checkbox flipped from Yes to No. The maintainer either reports "cannot reproduce" on a real bug or chases a $40,000 discrepancy that is purely the missing ledger — precisely the "sends the recipient after a bug that is not there" the module doc says is worse than not sharing. `scrub_preserves_every_computed_figure` passes `&state` to BOTH `assemble_absolute` calls, so it proves only "scrubbing does not move a figure GIVEN the same ledger"; it never touches the question of whether the emitted file reproduces the return, and it never will. The commit's end-to-end check ("re-imported into a fresh vault reports the identical AGI 2,123,995") can only have been run on a household whose figures do not depend on the ledger, which is exactly the case that hides this. Note the only channel that carries the ledger, `export-snapshot`, is UNSCRUBBED and emits wallet/account identifiers and donee names (crates/btctax-cli/src/render.rs:720-760) — so a filer who compensates for this gap makes a far larger disclosure than the one `scrub` prevents.

**Fix:** Re-scope the claim to what the test holds ("identical GIVEN the same ledger") and say plainly in `--help`/man that the file is the non-ledger half of the return only. Better: have `income scrub` detect a non-empty `LedgerState` contribution for the year and refuse, or emit a loud stderr warning naming the figures that will not travel — and never point the filer at `export-snapshot` without scrubbing it first.


### [IMPORTANT] `income scrub` is the one command touching a year's inputs that ignores the input-form draft, so on a parked year it tells the filer their year has no inputs, and on a WIP draft it silently emits a return the filer is not looking at.

**Location:** crates/btctax-cli/src/cmd/tax.rs:174 (`return_inputs::get`, no `coherence_clear_or_refuse`) vs :60 (import) and :191 (clear); crates/btctax-cli/src/input_form_store.rs:132-141 ("a draft shadows the committed row"), :234-237 ("a parked year has no committed row"); crates/btctax-cli/src/main.rs:317

**Failure:** §6.1 precedence makes the draft the working return. A PARKED draft is "the sole copy of a screened return" and its year has NO committed row — so `btctax income scrub --year 2024` prints "No full-return inputs set for tax year 2024." for a filer who plainly has inputs, and does it for exactly the population the `--help` targets ("so they can reproduce a refusal"). With a non-parked WIP draft the committed row still exists but is the pre-edit state, so scrub writes an older return than the one the filer is editing, with no warning that a draft shadows it. The four existing writers of the committed row all call `coherence_clear_or_refuse` precisely because a draft can shadow it; scrub, added as a fifth command on the same row, does not consider the draft at all.

**Fix:** Route `scrub_return_inputs` through `input_form_store::load` and scrub the `Loaded::Draft` when one exists; at minimum, distinguish the parked case from "no inputs" in the message and warn on stderr when a draft shadows the committed row.


### [IMPORTANT] `--out` is the only production file write in the CLI that bypasses `fsperms::open_owner_only`, so the scrubbed file lands world-readable (0644 under a normal umask) while every other decrypted artifact btctax writes is 0600.

**Location:** crates/btctax-cli/src/main.rs:312 (`std::fs::write(&path, toml)`); vs crates/btctax-store/src/fsperms.rs:18-47 and its callers crates/btctax-cli/src/render.rs:744-746, :926, :976, :994, :1170, crates/btctax-cli/src/cmd/admin.rs:767, :779, crates/btctax-store/src/vault.rs, crates/btctax-store/src/atomic.rs:12

**Failure:** `btctax income scrub --year 2024 --out /tmp/s.toml` leaves a file readable by every user on the machine containing the household's exact AGI, every W-2 wage box, estimated payments, each dependent's exact date of birth, any date of death, blindness, and filing status. De-identified is not anonymous — the `--help` says so itself — and DOB plus income plus filing status is a short walk to a household. render.rs:728-730 states the standard and even names this exact hole: CSV exports are opened owner-only "so decrypted PII matches the hardened permissions", closing "the hole that `Writer::from_path` + umask would leave". That hole is reopened here, and btctax-store's integration tests assert 0600 on every other file the product writes.

**Fix:** `btctax_store::fsperms::write_owner_only(&path, toml.as_bytes())` in place of `std::fs::write`, and add the mode assertion the store crate's tests already model.


### [MINOR] Every synthetic EIN the scrubber mints reds the repo's own `pii-scan` gate, so the intended last step of the workflow — commit the scrubbed return as a regression fixture — cannot be taken; the module's "safe by construction … needs no allowlist entry" claim is true of the SSNs and false of the EINs.

**Location:** crates/btctax-core/src/tax/scrub.rs:59-61 (`synthetic_ein` → `9N-NNNNNNN`) and :46-47 (the claim); scripts/pii-scan-generic.sh (`SHAPES` includes `\b[0-9]{2}-[0-9]{7}\b`; `ALLOWED_EIN` is a closed token-exact list of eight values; "No structural rule is available" for EINs)

**Failure:** `synthetic_ssn` clears the scan structurally — `ALLOWED_SSN_IMPOSSIBLE`'s `^[0-8][0-9]{2}-00-[0-9]{4}$` admits `1NN-00-NNNN` — but `synthetic_ein(0)` = `90-0000001` matches the EIN shape and appears in no allowlist, so CI's pii-scan job and the pre-push hook (which scans every commit in the pushed range, so fixing HEAD does not fix it) both fail on a provably synthetic token. Because a fresh EIN is minted per DISTINCT employer, the token set is unbounded: each new scrubbed fixture would need a hand edit to the list the script itself declares closed — the enumerate-the-outcomes-you-happened-to-see pattern CLAUDE.md rules out.

**Fix:** Mint synthetic EINs inside a documented structural window and add the matching `ALLOWED_EIN_SYNTHETIC` rule keyed to the generator (paired with a B1 planted-defect test), or narrow the scrub.rs claim to SSNs and say the EINs need an allowlist entry.


### [MINOR] The scrubbed file carries no marker distinguishing it from a real inputs TOML, and `income import` is an unconfirmed whole-blob upsert — so re-importing the wrong `.toml` silently replaces the vault's real identity with the synthetic one and destroys the IP PIN, which nothing can restore.

**Location:** crates/btctax-cli/src/cmd/tax.rs:169-183 (bare `to_string_pretty`, no header comment) and :49-103 ("`income import` is a whole-blob upsert", no confirmation); crates/btctax-core/src/tax/scrub.rs:91 (`ssn: synthetic_ssn(n)`) and :164 (`ip_pin: None`)

**Failure:** A scrubbed file and a real one have the same extension, the same schema and the same acceptance by `income import`; nothing on the page tells them apart, which is the repo's own "two blanks look identical" problem applied to a whole file. `btctax income import --year 2024 --file scrubbed-2024.toml` then overwrites the vault's real header — names, address, SSNs — with the synthetic identity, and DROPS the IP PIN outright, since the scrub emits no `ip_pin` key and the vault is the system of record. There is no diff, no confirmation and no undo. Worse, the synthetic SSN is well-formed, so it survives `Ssn::canonical` and would print on a filed 1040 as `101-00-0002` at `1 Example St, Springfield IL 62704`.

**Fix:** Prepend a provenance line to the emitted file (`# btctax income scrub — SYNTHETIC identity, real figures; do not re-import over your own return`) and have `parse_return_inputs_toml` refuse a file carrying that marker unless an explicit `--force` is given.


### [NIT] A TOML serialization failure is reported to the user as vault corruption, and a failed `--out` write does not name the path.

**Location:** crates/btctax-cli/src/cmd/tax.rs:177-180 (`CliError::BadConfigValue { key: "return_inputs[2024]", value: e.to_string() }`); the variant at crates/btctax-cli/src/lib.rs:117-120 (`#[error("unrecognized stored config value: key={key:?} value={value:?}")]`, documented as "a `cli_config` row held an unrecognized value (corrupt DB…)"); crates/btctax-cli/src/main.rs:312 (`map_err(CliError::Io)`)

**Failure:** If `toml::to_string_pretty` ever fails, the filer is told `unrecognized stored config value: key="return_inputs[2024]" value="<serializer error text>"` — a message that names the wrong subsystem, puts an error string in the slot reserved for a value, and points at a config key that exists in no file they can edit; the natural response (clear the row and re-import) is destructive. Separately, if the `--out` write fails, the user sees a bare OS error with no path, and any partially-written file is left in place.

**Fix:** Use a dedicated error (or `CliError::Usage`) naming the real failure, and attach the path to the write error.

