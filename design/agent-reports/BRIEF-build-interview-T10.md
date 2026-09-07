# Brief — interview build T10: the trailer (§5.4) — direct deposit, phone, spouse IP PIN, foreign address

Single implementer, shared main tree `/scratch/code/bitcoin_tax`, branch `main` (HEAD named at
dispatch). No subagents; no commit/push; never `git checkout --`/`git restore`/`git stash` (revert a
plant via a `cp` backup). Tests via `cargo nextest run --locked -p <crate> -E '<filter>'` (never
`cargo test`, never `--release`, never the whole workspace); `cargo fmt --all` and a clean
`CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings`
before finishing. Every guarantee lands with a kill seen red once; quote the red. Every pinned number
moved: old → new with cause. `/tmp` is a 32 GB tmpfs. Synthetic identifiers: SSNs from the
never-issued space (area 000/666, group 00, serial 0000); EINs only from `scripts/pii-scan-generic.sh`'s
`ALLOWED_EIN`; a routing number in a fixture must be one a bank cannot hold (see the validator you
build — e.g. a prefix outside 01–12 / 21–32 for the negative case, and for the positive case a
number whose checksum passes but whose prefix is documented as a test value). Process in force
(owner S6): ONE seam review and ONE re-verification after you.

★ **Standing rules:** no decision keys on a list you typed beside derived data; a build's kills must
ask what the NEXT SURFACE does with what the build wrote; a secret is asymmetric — written, never
read back; a blank is normal, and a line that is blank because the filer chose a paper check is
correct.

## The contract
`design/SPEC_interview.md` r2 **§5.4** (`direct_deposit: Option<DirectDeposit { routing: String,
kind: Checking | Savings, account: String }>` — 35b–d, `i1040gi--2025.txt:23963-24000`, the routing
rule *"nine digits … first two digits 01 through 12 or 21 through 32"* as the parse validator;
`phone: String`; `spouse_ip_pin: Option<String>` secret and asymmetric like the taxpayer's;
`foreign_country`, `foreign_province`, `foreign_postal_code` live iff `foreign_country` is
non-empty, printed in the header's foreign block), **§7 row T10** (its kills), **R14** (every new
field joins a classifier class by construction), **§2.2**'s row for direct-deposit-to-IRA / Form
8888 (stays a census/advisory refusal — line 35a's split-refund box is NOT built). Owner question
**Q4** (deposit vs paper check) is OPEN: build the field so either answer is expressible and the
advisory `RefundByPaperCheck` is silent exactly when a deposit is given. Build AS WRITTEN; the
tree's real names win; deviations recorded.

## Settled facts (controller-measured at `1f3cc137`)
- `crates/btctax-forms/forms/2024/f1040.map.toml:94`: *"UNMAPPED ON PURPOSE: the direct-deposit
  block (35b routing f2_25, 35c/d account f2_26) — v1 never …"* — T10 maps them; the foreign-address
  cells `f1_15` (country) / `f1_16` (province) are `unmodeled` at `:268-269` (postal code beside
  them); the TY2025 map has none of these cells yet (17 lines) — T8 maps the dependents grid; T10
  adds the trailer and foreign cells for 2025 through the label reader the same way.
- `Advisory::RefundByPaperCheck { refund }` (`advisories.rs:212`, rendered `:578`) exists and fires
  today on every refund; `ip_pin: Option<String>` (`return_inputs.rs:467`) is the taxpayer's, with
  the asymmetric secret handling in `btctax-input-form/src/seam.rs` (grep `ip_pin`) — mirror it
  exactly for the spouse; `foreign_country_names: String` (`:1248`) is the Schedule B foreign-country
  field, NOT the address — do not conflate.
- `form1040_full.rs:390` writes the identity block; the header's foreign block is recorded as `X`
  (unreachable) in `FIELD_PROVENANCE.md` §6a today.
- T3's classifier discipline; T1's `LEAF_SOURCE` (`FilerRecords` for these); T4b's opener (the
  mailing address and phone are identity — carried and named; the direct deposit and IP PINs are
  NOT carried: a PIN is per-year, an account is re-confirmed).

## What T10 delivers
1. **`direct_deposit`** on `ReturnInputs` with the routing validator (nine digits, prefix 01–12 or
   21–32, and the ABA checksum — cite the instruction for the prefix rule; the checksum is a
   documented banking rule, state its source), `kind` as a `Choice`, `account` free text; the 35b–d
   cells mapped (TY2024 and TY2025) and filled by the emitter; the 35a checkbox (split refund / Form
   8888) stays unmodeled with its census reason; `RefundByPaperCheck` silent when a deposit is
   present, present when not.
2. **`phone`** — printed in the signature block (map the cell; cite the extract line).
3. **`spouse_ip_pin`** — secret, asymmetric (`get` never returns digits; the TUI shows a mask; the
   TOML wire carries it under the same marker guard as the taxpayer's); printed in the spouse's
   IP PIN cell.
4. **The foreign address block** — three fields, live iff `foreign_country` non-empty; the three
   header cells mapped for both years; `FIELD_PROVENANCE.md` §6a's `X` retired for them.
5. **Fixtures** (§5.7): `direct_deposit` present on the coverage fixture; a foreign-address variant
   printed on a TY2024 fixture; `LEAF_SOURCE`, classifier rows, TOML round-trip, `income import`
   accepting the new keys and refusing unknown ones.

## Kills (each seen red once)
The routing validator: a prefix of 13, 20, 33 refuses; a checksum failure refuses; a valid one
passes; the advisory silent when a deposit is given and present when not (both directions); the
spouse-PIN asymmetry (`get` never returns digits; a snapshot never contains them; the marker guard
holds on import); the foreign block printed on a fixture and absent when `foreign_country` is empty;
the mapped cells read back from the filled PDF; the TY2024 golden byte-identical except the cells a
fixture newly populates (each explained); the opener does not carry the deposit or the PINs.

## Constraints
- Secret-handling defects are logged, never gating — but build the asymmetry anyway (it is the
  pattern the taxpayer's PIN already follows).
- Nothing else in the identity block changes; T8's dependents grid untouched.
- If context runs short: leave the tree compiling, fmt-clean and green, and say what is unfinished.

## Report — your FINAL action
`design/agent-reports/2026-09-07-build-interview-T10-implementation.md`: per numbered item what
landed, the mapped cells per year, the validator's rules with cites, every deviation, every kill with
its red text, every pinned number moved, suite lines per crate. Return only a 4-line summary plus
the path.
