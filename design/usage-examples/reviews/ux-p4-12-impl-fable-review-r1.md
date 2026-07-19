# Independent adversarial review — Phase 4 "Affordances" (UX-P4-5 + UX-P4-12 b–h, (i) deferred)

Reviewed against current source at `/scratch/code/bitcoin_tax` (HEAD `b101fef`), diff at `scratchpad/phase4.diff`. All asserted defects below were verified against source, and three were additionally reproduced with the built binary (`target/debug/btctax`).

---

## CRITICAL — GATES

None.

---

## IMPORTANT — GATES

**I-1 — (b) The new `--fmv` help documents a daily-close fallback that does not exist on this command. The help is factually wrong, and it shipped into the man page too.**
`crates/btctax-cli/src/cli.rs:535-537` (and regenerated `docs/man/btctax-reconcile-classify-inbound-income.1`): *"When omitted, the bundled daily close for the receipt date is used (or a Hard blocker if none exists)."* Actual behavior: `classify-inbound-income` with `--fmv` omitted stores `InboundClass::Income { fmv: None }` (main.rs:1029-1039), which folds to `Op::IncomeInbound` — and `fold.rs:891-898` matches `None` by firing the Hard `FmvMissing` blocker ("income inbound FMV missing") and booking a $0 `basis_pending` lot, with **no price consultation at all**. The daily-close synthesis exists only for (a) the bulk path (`bulk_classify_income` per-row `fmv_of`, reconcile.rs:646) and (b) pseudo-mode Phase C, which is gated `if pseudo_on` AND scoped to native `EventPayload::Income` rows (resolve.rs:1007-1030) — never a classified `TransferIn`. Even `set-fmv` cannot cure it: `manual_fmv` is consulted only in the `EventPayload::Income` arm of `build_op` (resolve.rs:286-290), not the TransferIn/inbound-class arm (resolve.rs:360-369).
**Empirical repro:** fresh vault, import a Coinbase `Receive` on 2025-02-01 (a date the bundled dataset covers — `events list` itself prints "~$1021.07"), run `reconcile classify-inbound-income <ref> --kind mining` (no `--fmv`) → `verify` shows `Hard blockers: 1 — [FmvMissing] ... income inbound FMV missing`, exit 1. The close existed and was not used; the blocker fires unconditionally. The help teaches the user to omit `--fmv` expecting a valuation that never happens. The `help_units.rs` KAT pins only "USD dollars, NOT sats" and the kind list, so it cannot catch this. **Fix:** correct the sentence (omitted `--fmv` → Hard `FmvMissing`, remedy = re-classify with `--fmv`, or use the bulk command for auto-FMV), regen the man page. *(Adjacent observation found during the repro, pre-existing and non-gating: the `FmvMissing` remedy hint "no local price for this date — run `btctax-update-prices`" (price_cache.rs:14 via render.rs:2418) is attached to this blocker even when the local price exists — worth a FOLLOWUPS note when folding.)*

**I-2 — (c) A per-account (scoped) standing order is read back as the vault-wide `forward_method:`, with the scope silently dropped and the FIFO default line suppressed.**
`MethodElection` carries an optional `wallet` scope (event.rs:245-259: scoped election governs *that exchange account only*; otherwise global election; else FIFO). `render::method_election_lines` → `ElectionLine` (render.rs:508-513) drops the wallet field, and the new config read-back (main.rs:559-577) labels every non-voided line `forward_method: …`.
**Empirical repro:** vault with one Coinbase buy; `config --set-forward-method hifo --exchange exchange:coinbase:default --effective-from 2099-01-01`; then `config` prints exactly:
`forward_method: HIFO (standing order effective 2099-01-01, in force)` — no scope qualifier, and the `forward_method: FIFO (default …)` line is suppressed because `orders` is non-empty. The truth: HIFO governs one exchange account from 2099; the forward method for the whole rest of the vault is FIFO. The read-back asserts the opposite of what the set path itself said one line earlier ("Recorded **per-account** standing order … for exchange:coinbase:default"). This is a wrong statement introduced by this diff — the (c) deliverable ("config shows what `--set-forward-method` recorded") misrepresents the scoped case. (verify's Standing-orders block has the same scope omission, but it is a pre-existing history listing, not a "forward_method:" claim.) The KAT (`config_shows_forward_method_standing_order`) exercises only a global order. **Fix:** carry the scope into the line (e.g. `ElectionLine` gains the wallet, or config renders `forward_method[exchange:coinbase:default]: HIFO …`) and keep the global default/global-order line visible when no *global* order is in force; add a scoped-order KAT.

**I-3 — (g) The new set-donation-details hint teaches an invalid command line.**
`crates/btctax-cli/src/cmd/reconcile.rs:1327-1330`: `` `reconcile reclassify-outflow <out-ref> donate …` ``. The actual grammar (cli.rs:566-584, and the codebase's own canonical citation at cli.rs:513) is `reconcile reclassify-outflow <out-ref> --as-kind donate --amount <usd> …` — `as_kind` is a required `--as-kind` value-enum flag, and `--amount` is required. A user pasting the suggestion gets clap's "unexpected argument 'donate'" error — the item existed precisely to replace a dead-end pointer, and the replacement pointer is itself a command that cannot parse. The KAT asserts only `msg.contains("reclassify-outflow")` (reconcile.rs:1626-1629), so it cannot catch this. **Fix:** `--as-kind donate` (keep the `…` for the rest), and strengthen the assertion to pin `--as-kind donate`.

**I-4 — UX-P4-5's spec KAT contract is not met: the warning emission is unpinned, and "packet unchanged" is pinned only as a path count.**
SPEC §4: *"KAT: warning emitted; packet bytes unchanged."* Delivered KAT (`export_irs_pdf.rs:540-591`) pins the `forms_ignored_full_return` **flag** in both directions (good, and the flag logic itself is correct — see clean-checks below), but nothing anywhere pins the actual stderr warning: the only occurrence of "ignored on a full-return year" outside `main.rs:719-726` is a comment, no journey passes `--forms` on a full-return year (examples.md uses `--forms` only on 2025 crypto-slice years), and no process-level test runs the binary on this path. A mutant deleting the entire `if report.forms_ignored_full_return { eprintln!… }` block — i.e. the whole user-visible deliverable of UX-P4-5 — survives the suite. Additionally the second assertion compares only `full_return_paths.len()` while its message claims "byte-for-byte the same set" — a same-size different-set regression would pass. Per the project's own standing rule (a fix isn't done until the mutation dies) and the explicit spec KAT wording, this is fix-or-file-blocking. **Fix:** a process-level KAT (the `config_dispatch.rs`/`help_units.rs` pattern) asserting stderr contains the warning on a full-return year with `--forms` and not without; compare the sorted file-name sets (or bytes, if the PDFs are deterministic) instead of counts.

---

## MINOR

**M-1 — (e) missed `Hifo` sites on user surfaces; the FOLLOWUPS "TreatmentC/Hifo gone" claim overclaims.**
(a) `main.rs:523` — the scoped-set confirmation prints `attests {:?} for {x}` → observed verbatim in my repro: "attests **Hifo** for exchange:coinbase:default". Same command surface (e) fixed, one line above the fixed output; `render::lot_method_display` is now `pub` and right there. (b) `main.rs:2079` — the bulk-void payload summary renders `MethodElection {:?} from {}` → "MethodElection Hifo from …"; aggravated by that formatter being the UX-P4-7 deliverable whose own doc comment (main.rs:2137) mandates "no `{:?}`". Original group was filed as a Nit, so these are Minor — but correct the FOLLOWUPS "(e) DONE … Hifo gone" wording when folding.

**M-2 — (f) only one of the two reworded core advisories is pinned.** `classify_inbound_self_transfer_cli.rs:114-131` pins the zero-basis advisory's new surface-neutral text (both positive and negative assertions — good). The defaulted-acquired advisory (fold.rs:1042-1047), reworded in the same commit, has no assertion on its new wording anywhere; a revert of that one string survives the suite.

**M-3 — (c) a vault whose only order is backdated/ignored gets a `forward_method:` line naming the ignored method, and the actually-governing method (FIFO) is never stated.** The suppression of the default line when `orders` is non-empty is deliberate per the code comment (main.rs:560-563), and the "(…, backdated/ignored)" note is honest — but the read-back then never says what the forward method actually *is*. Fold into the I-2 rework (state the governing method explicitly).

---

## NIT

**N-1 — (h)** The footer KAT (`draw_edit.rs:5982-6013`) pins one representative footer (`draw_void_list`) of ~23 changed; the grep sweep is clean today (verified: no footer "swallowed" literals remain — all remaining hits are comments/tests), but a source-scan assertion would pin all of them.
**N-2 —** The `config` arm now performs three full vault open/decrypt cycles per invocation (`set_forward_method` → `show_config` → the new `Session::open(...).load_events_and_project()`), and `config` (show) now requires a loadable event log + projection — a vault that hits the documented unknown-variant load failure can no longer show config (it previously could). Cosmetic/perf.
**N-3 —** `filter(|e| e.note != "voided")` (main.rs:566) is stringly-typed against the `ElectionLine.note` constant; a typed status enum would make the coupling structural.

---

## (i) DEFERRAL VERDICT: **SOUND**

Every factual claim in the FOLLOWUPS resolution block verifies against source:
- **I-11 is real and does what's claimed.** `input_form_store.rs:286-295`: `commit` returns `CommitOutcome::NoTables` and **writes nothing** both when `table`/`params` are absent and when they are for a different year, with the poisoning rationale documented in-function ("tables for a DIFFERENT year would `screen_inputs`-pass and write a committed row for a table-less `year`, poisoning it at resolve"). It is a *reviewed* guard: `design/input-form/reviews/SPEC-input-form-fable-r2.md` verifies it ("I-11 commit TY2024-only, NoTables ✓").
- **The CLI really stores without that gate.** `cmd/tax.rs:101`: `return_inputs::set(s.conn(), year, &ri)?` — no table check; gating happens downstream. And a committed row is load-bearing at the export dispatch (`admin.rs` full-return branch keys on `return_inputs::exists`), so the poisoning concern is concrete, not hypothetical.
- **The papercut is real as described**: the TUI defaults `selected_year` to 2025 (pinned by its own tests, main.rs:14504).

Given that, the plan's parenthetical default ("align to the CLI's store-then-gate-at-export") genuinely requires relaxing a reviewed fail-closed guard — which is §D's exact trigger ("implementation shows the … plan was wrong — stop and re-enter … A design change is never smuggled in as 'just another phase'"). Nor is the enumerated safe partial (early form-open notice) something the author could have done unilaterally: it is a *different* choice than the plan's recorded default, so implementing it autonomously would be the same smuggling in the other direction. The deferral names an owning phase (dedicated brainstorm/spec pass), enumerates the three resolutions, and restates the binding `[T-U-P4-12]` constraint. This is the process working, not an evasion. One bookkeeping caveat for the future pass: SPEC §4 still carries the now-known-conflicted "default: align to the CLI" text and must be explicitly superseded when the decision is made.

---

## Clean checks (verified, no finding)

- **§1 dollar-invariant:** holds. `fold.rs` changes are string-only in two blocker `detail`s; the `render.rs` refactor (`voided_targets`/`method_election_lines` out of `build_verify`) is a verbatim code move — identical filter/sort/note logic, identical note strings — and the byte-gated examples golden changed *only* the config lines, independently confirming `verify` output is unchanged.
- **UX-P4-5 flag:** set only at the dispatch (`admin.rs:247`, `= !forms.is_empty()`, reachable only when `return_inputs::exists`), `false` at both other construction sites (`admin.rs:394,564`); `export_full_return` never reads `forms`; sole consumer is `main.rs:719` (stderr). Never wrongly true or false.
- **(e) mapping:** `TreatmentC → "non-taxable, basis carries (TP8 c)"` / `TreatmentB → "taxable mini-disposition (TP8 b)"` (render.rs:518-523) — correct per the user-mandated TP8 policy.
- **(b) kind list:** help's five values exactly match `parse_income_kind` (eventref.rs:153-164); both `--fmv` args, the positionals, and `--business` got help; both man pages regenerated.
- **(d):** exit-2 usage error names `--show`, which exists (cli.rs); process-level KAT pins both substrings.
- **(f) scoping:** core advisories are surface-neutral in both CLI verify and TUI; the remaining bare "press 'v'" strings are TUI-local (main.rs:3399 etc.), where the keybind is meaningful — correctly left alone.
- **(h):** grep-clean; no footer "q: swallowed" remains anywhere.

---

## VERDICT

**NOT GREEN — 0 Critical / 4 Important to fold:** I-1 (false `--fmv` fallback claim in help + man), I-2 (scoped standing order misrepresented as vault-wide `forward_method:`), I-3 (invalid suggested syntax in the set-donation-details hint), I-4 (UX-P4-5 warning emission unpinned / packet-unchanged pinned only as a count, vs. the spec's explicit KAT contract). The (i) deferral is SOUND and does not gate. Re-review after folding.
