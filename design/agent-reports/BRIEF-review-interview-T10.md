# Brief — seam review of interview build T10 (the trailer)

You are an independent, adversarial BUILD REVIEWER in your own git worktree at the commit named at
dispatch (the T10 build commit). Read-only for the record: every plant is made in YOUR worktree and
reverted (`git checkout -- <file>` is fine there); no commits, no subagents.
`export CARGO_TARGET_DIR=/scratch/code/bitcoin_tax/target-review` before any cargo command; scoped
runs only (`cargo nextest run --locked -p <crate> -E '<filter>'`); the instruments are `cargo run -q
-p xtask -- line-coverage` / `census-join` / `stop-list`. **Environment, not findings:** six
`form_delta` tests and `harness_check::the_write_hook_denies_new_archives_and_asks_once_per_new_directory`
fail in a PDF-less worktree with a redirected target dir; if a bundled TY2024/TY2025 `f1040`
template is absent, say so and mark the emitter claims unverified, never passed.

## The one question
Can a refund be routed, a secret be read back, or a header cell be printed from anything but what
the filer entered this year? Concretely: (a) the routing validator refuses every prefix outside
01–12 / 21–32 and every checksum failure and passes a valid number; (b) `RefundByPaperCheck` is
silent exactly when a deposit is given; (c) the spouse IP PIN is asymmetric (never read back, never
in a snapshot, guarded on import like the taxpayer's); (d) the foreign block prints only when
`foreign_country` is non-empty and its three cells are mapped for both years; (e) the opener carries
address and phone as identity and never the deposit or the PINs; (f) the 35a split-refund box stays
unmodeled with its reason. Not a fresh audit of T1–T9 or T16; not a spec re-review.

## Settled (machine-verified by the controller; do not re-measure)
The T10 build report `2026-09-07-build-interview-T10-implementation.md` lists the mapped cells per
year, the validator's rules with cites, kills and deviations; the controller has confirmed the
suite line it states and the instruments' lines. Spec: `design/SPEC_interview.md` r2 §5.4, §7 row
T10, R14, §2.2 (Form 8888 stays a refusal/advisory); `i1040gi--2025.txt:23963-24000` (lines
35b–d and the routing rule); owner question Q4 (deposit vs check) still open — both answers must be
expressible. Prior builds: T4b's opener; T3's classifier discipline; the taxpayer IP PIN's
asymmetric handling in `btctax-input-form/src/seam.rs`.

## Seams
1. **The validator.** Prefixes 00, 13, 20, 33, 99 refuse; a nine-digit number with a wrong check
   digit refuses; a documented test routing number passes; eight and ten digits refuse; the
   messages name the rule and its cite. Plant: drop the checksum → red?
2. **The advisory.** Deposit given ⇒ `RefundByPaperCheck` absent; deposit absent ⇒ present with the
   refund; on a return with no refund neither; the packet manifest reflects the same.
3. **The secret.** `spouse_ip_pin`: `get` never returns digits; no snapshot, golden or `report`
   output contains them (plant a PIN on a fixture and grep every emitted artifact); the TOML marker
   guard refuses a bare PIN on import the way the taxpayer's does; the emitter prints it in the
   spouse cell only.
4. **The cells.** 35b/35c/35d, the phone, the two IP PIN cells and the three foreign cells resolve
   in both years' templates through the label reader (not a typed list); each has a `[census]`
   entry (the `UNCENSUSED` register's `f1040` count falls by exactly the cells mapped on TY2025);
   read back off a filled fixture; the TY2024 golden unchanged except the cells a fixture newly
   populates (each explained).
5. **Liveness and the opener.** The three foreign fields are live iff `foreign_country` is
   non-empty (a US address never asks them); `direct_deposit` is optional and its absence blocks
   nothing; the opener seeds the mailing address and phone and NOT the deposit or PINs (T4b's
   `carried_identity` names them).
6. **`LEAF_SOURCE` and the classifier** cover every new leaf; `income import` accepts the new keys
   and refuses unknown ones; the no-brick test covers any new question.

## Severity
A refund routed from a number the validator should refuse, a secret read back, a header cell
printed from a stale carry, or a kill that does not red is **Critical** or **Important**.
Secret-handling defects are logged as follow-ups, never Critical/Important (owner ruling
2026-08-27) — report them anyway. Design opinions outside the one question are Minor at most.

## Report — your FINAL action
Write `design/agent-reports/2026-09-07-build-interview-T10-review.md` in your worktree: Commands;
Summary; findings ordered by severity with Where / What is wrong / Evidence (the plant and the
observed output) / Minimal change; "Seams checked clean"; `Counts: C=<n> I=<n> M=<n> N=<n>`.
Return ONLY a 3-line summary plus the path.
