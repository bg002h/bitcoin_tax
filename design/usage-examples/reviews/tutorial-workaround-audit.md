# P4 workaround-audit (SPEC §10) — the co-equal bug-hunt

*Audit date 2026-07-18, branch `feat/usage-examples` (post-P3). Author: a deliberately skeptical
journey author driving the ASSEMBLED surface adversarially — the real `btctax` binary (debug build,
pinned env: `BTCTAX_PASSPHRASE=pw`, `TZ=UTC`, `LC_ALL=C`, scrubbed `HOME`, nonexistent
`BTCTAX_PRICE_CACHE`, `BTCTAX_NOW` where a decision is recorded) across ~60 off-happy-path CLI
probes over the J1–J6 corpora, plus the `btctax-tui-edit` reconcile flows driven headlessly through
the in-crate `handle_key`/`type_str`/`TestBackend` harness (a temporary probe module, appended for
the run and removed after — `git diff` is empty; probe screens under the session scratchpad
`audit/tui/`). Every route-around found is catalogued below and classified **bug-to-file /
harness-artifact / intentional**; real bugs are filed as `FOLLOWUPS.md` **UX-P4-1 … UX-P4-12**.*

Per the §3.1 fence, nothing here was fixed in-cycle: this document files, it does not edit product
code. Known items (UX-P0-1/3, UX-P1-1..9, UX-P2-1, UX-P3-1/2, N-R1) are referenced, not re-filed.

## A. New bugs filed (UX-P4-*)

| # | Finding (repro in the FOLLOWUPS entry) | Class | Sev | Filed as |
|---|---|---|---|---|
| A1 | **Pseudo mode: `report --tax-year` prints a clean tax figure with NO `[PSEUDO]` indication** while a synthetic default contributes. Repro: vault whose sale consumes a pseudo-classified lot → `report --tax-year 2025` prints `TOTAL … 4041.50` (LT gain built on the fictional $0-basis/LT-default lot) with zero pseudo marker. Bare `report` DOES flag the lot + disposal rows `[PSEUDO]`; `verify` discloses (`PseudoReconcileActive`); export is blocked behind the attest phrase — the one silent surface is the primary number-bearing one. Violates the mode's own "loudly-flagged on-screen estimate" contract; the answered-ness class (a figure silently answering for the filer). | bug-to-file | **Important** | **UX-P4-1** |
| A2 | **TUI classify-inbound confirm modal states the acquired-at default backwards.** `draw_edit.rs:927` renders `acquired_at: (empty = default = receipt date, short-term)`; the engine (`fold.rs:1024`, `long_term_default_acquired`) persists **1 year + 1 day before receipt → LONG-TERM**, as the CLI help and the `SelfTransferInboundDefaultedAcquired` advisory correctly state. Confirmed end-to-end: confirming the modal on a 2025-05-23 receipt persists `Acquired 2024-05-22` (visible in Holdings). A rate-determining fact stated wrongly at the exact point of informed consent for a vault write. | bug-to-file | **Important** | **UX-P4-2** |
| A3 | **Record-then-conflict false success + inconsistent remedy hints.** `reconcile classify-inbound-self-transfer`/`reclassify-income`/`reclassify-outflow` accept a typo'd ref, a wrong-type ref (a Buy), or a duplicate re-decide with `Recorded decision decision|N` (exit 0); the error surfaces only on the NEXT `verify` as a `DecisionConflict` hard blocker. Voiding a nonexistent decision (`void decision|99`) also "succeeds" and becomes its own hard blocker (`void targets unknown event`) cleared only by voiding the void. Remedy hints are inconsistent: some variants carry "void the decision to clear this blocker", the void-of-unknown carries none, and the unknown-event `ReclassifyIncome` hint suggests the wrong verb for a mere typo. `set-donation-details` proves record-time validation is feasible in this architecture (it fails loud at record time). Conservative posture holds (verify gates; nothing silently wrong) — the cost is workflow, not correctness. | bug-to-file | Minor | **UX-P4-3** |
| A4 | **Value-validation gaps (input-contract class, extends UX-P1-3).** (a) Negative basis accepted on BOTH surfaces — CLI `--basis=-5000.00` (the `=` form slips the clap `-`-prefix guard) and the TUI form (which rejects `abc` as "bad USD" but not `-5000`) — and flows into gain math: `basis -5000.00 → gain 26799.23` (> proceeds). (b) `--acquired` AFTER the receive date accepted silently (factually impossible; the lot then also vanishes from earlier what-if sales). (c) `set-donation-details --donee-ein banana --appraiser-tin fruit` → "Donation details saved" — lands on Form 8283. | bug-to-file | Minor | **UX-P4-4** |
| A5 | **`--forms` silently ignored on a full-return year.** With full-return inputs present, `export-irs-pdf --tax-year 2024 --forms f8949` writes the whole 14-form packet with no notice that the explicit slice request was disregarded. (Distinct from UX-P1-2's stale help — which describes a third behavior, refusal — and from UX-P1-4's empty header, both re-confirmed in the same run.) | bug-to-file | Minor | **UX-P4-5** |
| A6 | **Bare `report` renders a fully-pending vault as empty.** J2-shaped vault (2 BTC of lots, the whole balance in a pending outbound Send): `report` prints `Holdings: none / Lots: none / Disposals: none` — indistinguishable from an empty vault, no pending line, no pointer — while `verify` knows (`pending 200000000`, `Pending reconciliation: 1`). The "where did my 2 BTC go?" panic moment. | bug-to-file | Minor | **UX-P4-6** |
| A7 | **Raw Rust `Debug` dumps in user-facing decision summaries.** CLI `bulk-void` preview and the TUI void list + bulk-void preview render `ClassifyInbound in … as SelfTransferMine { basis: Some(19000.00), acquired_at: Some(2026-01-01) }` (`main.rs:3742` formats the payload `{:?}`); the TUI column truncates it mid-field (`…{ basis: Non`). The repair surface — where users go after every A3 mistake — is the least legible one. The mnemonic-`(none)`-class find of this audit. | bug-to-file | Minor | **UX-P4-7** |
| A8 | **Bare io errors without path or hint.** Missing/wrong `--vault` → `error: io: No such file or directory (os error 2)` — no path, no "check --vault / run init". `--out` colliding with an existing file → `error: io: File exists (os error 17)` — no path. Contrast in-house precedent: `import nope.csv` → `io reading nope.csv: …`. | bug-to-file | Minor | **UX-P4-8** |
| A9 | **Insufficient-balance wording is flatly wrong.** `what-if sell --sell 0.6` with 0.5 BTC held → `no lots available to sell from that wallet as of that date` ("no" is false; the available balance is not shown; genuine-zero and insufficient collapse into one message). | bug-to-file | Minor | **UX-P4-9** |
| A10 | **`report --tax-year` exits 0 on NOT COMPUTABLE.** The refusal is loud in text (`NOT COMPUTABLE [TaxProfileMissing]`) but invisible to a script; `verify` sets exit 1 on hard blockers, `report` never does. | bug-to-file | Nit | **UX-P4-10** |
| A11 | **Event-ref discoverability is a documented workaround, not an affordance.** No `list`-refs verb; the sanctioned discovery path is export-snapshot CSV columns (`set-donation-details --help` says so; `select-lots --help` likewise) or stripping the trailing `#0` split-suffix from `report`'s lot ids; the Income section prints no refs at all (J4's refs embed a ms-timestamp a user cannot construct). Concrete trap, reproduced: pasting the tool's own displayed lot id (`…#0#0`) into `reclassify-income` records a decision that then hard-blocks as "targets unknown event" (A3 compounds it). | bug-to-file | Minor | **UX-P4-11** |
| A12 | **Grouped wording/affordance nits** (one entry, itemized): bad `--kind` error lists no valid kinds (vs `--as-kind`'s clap enum listing); `classify-inbound-income`/`set-fmv` args have blank help and no units (vs the exemplary `what-if sell --help`, which disambiguates sats/BTC); `config` sets the forward method but won't show it (read-back only in `verify`); `tax-profile --year` set-error never mentions `--show`; internal enum names on screen (`TreatmentC`, `Hifo`, `:: non_compliant`); "press 'v'" TUI keybinding language inside CLI `verify` advisory text; `set-donation-details` before `reclassify-outflow` points at removals.csv (circular — the removal isn't there either) instead of the missing prior step; TUI footer dev-speak "q: swallowed" (wraps mid-word); TUI editor defaults to year 2025 whose full-return commit then refuses ("2024 only") — late gate on the default year, and the opposite placement from the CLI, which stores 2025 inputs and gates at export. | bug-to-file | Nit | **UX-P4-12** |

## B. Known items confirmed / extended (referenced, NOT re-filed)

- **UX-P1-3 (`--amount` unit footgun) — confirmed and EXTENDED to the TUI**: the donate FieldForm labels the unit ("FMV (USD)" — better than the CLI's blank help) but a sats-scale value (100000 for a 100 000-sat outflow) passes straight into the confirm modal (`fmv: 100000`, no `$`, no ≈-value sanity line) and would persist. The negative/garbage arm of the same class is filed as UX-P4-4.
- **UX-P1-4 (empty "Filled IRS forms →" header) — confirmed live** on every full-return export in this audit, exactly as the J6 golden captures it.
- **UX-P1-6 (multi-lot 8283 "needs REVIEW" advice loop) — confirmed and EXTENDED to Section A**: a sub-$5,000 TWO-lot donation with `set-donation-details` fully completed still warns "needs REVIEW … Run `btctax reconcile set-donation-details …`" on every export (the advice loops; Section A requires no appraiser declaration at all, sharpening the misleadingness). A SINGLE-lot Section A donation with complete details clears with no warning — pinning the root cause to the same non-first-property-row rule UX-P1-6 describes for Section B. Recorded as a dated extension note under UX-P1-6 in FOLLOWUPS.
- **UX-P1-2 / N3 (stale `export-irs-pdf` help + `--forms` value naming) — confirmed**: the `--forms` value zoo (`f8949, schedule-d, schedule-se, form8283, form1040` — three naming conventions) is live; clap's did-you-mean tip ("a similar value exists: 'form8283'") softens the papercut. The new UX-P4-5 silent-drop is adjacent but distinct.
- **UX-P1-5 (DOB `[2012,106]`)** — not re-driven; still captured verbatim in the J6 golden.

## C. Standing refusals verified LIVE and correctly gated (intentional — keep as regression assertions)

1. `report --tax-year` with no profile → `NOT COMPUTABLE [TaxProfileMissing]` (J1 teaches the fix).
2. Unsupported year → `NOT COMPUTABLE [TaxTableMissing]` (2019 and 2030 both).
3. Unknown-basis inbound → `[UnknownBasisInbound]` hard blocker, `verify` exit 1 (J3's spine).
4. Oversold ledger → `[UncoveredDisposal] … dispose short by N sat` hard blocker.
5. Wrong passphrase → clean `wrong passphrase or corrupt key` (no leak); `init` over an existing vault refuses.
6. Malformed `BTCTAX_NOW` → exit 2 with the expected-format message; active seam prints the stderr banner (P0 KAT holds in the field).
7. §170(f)(11)(C) boundary exact: a claimed deduction of exactly $5,000 fires NO qualified-appraisal advisory (statute says *exceeds*); $6,000 does.
8. Pseudo-mode export → refused behind the exact attest phrase; `pseudo off` restores real-only. (The gap is only the `report --tax-year` banner — UX-P4-1.)
9. Post-sale `optimize accept` → skipped with the precise re-run remedy (`--disposal … --attest …`); the TUI 'z' status's `btctax optimize consult` remedy verified to exist.
10. `income import` unknown-key rejection is exemplary (names the keys, cites renames/removals, states why silent dropping is refused); unsupported-year full-return export refuses with "needs a supported tax year (TY2024)".
11. Import adapters fail loud on unrecognized and empty files; TUI tax-inputs commit on a no-tables year refuses loudly and KEEPS the draft.
12. **TUI empty-list sweep (23 Browse keys on an empty vault): every flow key yields an actionable status message** — several with exact remedy commands — no dead-end, no leaked internals, no panic. `q` is swallowed in list steps and typed in text fields (existing KATs hold); 8×Esc always unwound to the Browse baseline in probes.

## D. Harness-artifacts (not product findings)

- The `BTCTAX_NOW override active` stderr banner throughout probe transcripts is the P0 seam's disclosed integrity notice, not product noise.
- The `[exit N]` trailers in probe transcripts are the audit bench's convention (mirroring the golden's), not btctax output.
- The probe corpora re-embed the xtask CRLF constants; the TUI probes ran as a temporary in-crate test module (removed; tree byte-identical after restore).

## Tally

**~35 distinct route-arounds catalogued: 12 filed as new bugs (UX-P4-1..12 — 2 Important, 8 Minor, 2 Nit), 5 known items confirmed (2 materially extended: UX-P1-3 → TUI, UX-P1-6 → Section A), 12 refusal/gate behaviors verified live-and-correct, 3 harness-artifacts; headline finding: UX-P4-1 (pseudo-mode tax summary prints unflagged).**
