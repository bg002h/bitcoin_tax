# Fable independent re-review — SPEC_usage_examples.md r1 (persisted verbatim)

*Persisted 2026-07-16 verbatim, per STANDARD_WORKFLOW §2. Reviewer: Fable (independent). Verdict: GREEN —
0 Critical / 0 Important. Non-blocking Minors/Nits recorded for the fold + plan phase.*

---

# Fable re-review — SPEC_usage_examples.md r1

**Reviewed against:** HEAD `ac04ce2` (same commit as r0; spec + FOLLOWUPS delta are uncommitted working-tree files). Every factual fold claim re-verified directly against source.

## VERDICT: **GREEN — 0 Critical / 0 Important**

## RESOLVED: I1–I6 (all genuinely resolved, not reworded)

- **I1 ✓** — §4.2 C-fullreturn now carries the donation leg (`import` donation CSV → `reconcile reclassify-outflow --as-kind donate` → `set-donation-details`, both verbs exist at `cli.rs:536/634`), with the oracle-deviation caveat. Verified against `crates/btctax-core/src/tax/packet.rs:537-540`: `f8283` requires `sch_a` present AND `line12 > FORM_8283_THRESHOLD` (= `dec!(500)`, `printed.rs:164`) AND `form_8283(state, year, donation_details)` rows — exactly as §4.2 states. §6.1's per-form table matches the actual push conditions in `crates/btctax-forms/src/packet.rs:70-158` key-for-key; kitchen_sink=13/14 stands; §14 gaps 2/3 closed with a P1 assertion, not a deferral.
- **I2 ✓** — §6.2 pins the two allowed enumeration mechanisms, forbids household-packet derivation, mandates the count==14 cross-assert against the §6.1 literal, and pins exact `{name}`-component matching (N4). Verified the mechanism would enumerate 14-or-fail-loud: `fill_full_return` (`btctax-forms/src/packet.rs:36-162`) destructures all 14 `PrintedForms` fields with no `..`; an all-arms fixture that falls short of 14 reds the assert rather than silently shrinking the gate — which is precisely the property I2 demanded. (One fixture-authoring subtlety noted as a Minor below.)
- **I3 ✓** — Verified `cli.rs` has no `#[command(version)]` anywhere; `docs.rs:350-351`'s comment confirms the deliberate omission. The Cargo.toml-sourced front-matter pin is implementable with zero binary change; adding `--version` is explicitly named an S2 escalation. (One pin-the-manifest Minor below.)
- **I4 ✓** — Inventory now correct, verified line-by-line: `session.rs:1097` is inside `bulk_resolve_conflict_plan` and takes `conflict_ev.utc_timestamp`, which `persistence.rs:217` copies from the CSV row (`utc_timestamp: ev.utc_timestamp`) — CSV-derived; `session.rs:1183` is `self_transfer_match_plan` (TransferIn dates) — CSV-derived; `session.rs:1134` bulk-void iterates decision events — clock-derived; `cmd/reconcile.rs:968` `set_forward_method(…, now)` receives `now` from `main.rs:470` and defaults `effective_from` to the made-date; `render.rs:2258` prints `recorded`; `main.rs:66` is the sole clock read; `main.rs:2000-2013` is `render_bulk_void_preview`. T-P0.2/T-P0.5 now mandate a clock-derived read-back and name the forbidden CSV-derived surfaces. FOLLOWUPS UX-P0-1 diff confirms the corrected citation with the explicit I4 note.
- **I5 ✓** — `init`'s `key_backup: PathBuf` at `cli.rs:39` is non-optional (required) and J1 supplies it; `FormArg` at `cli.rs:900-912` = `f8949, schedule-d, schedule-se, form8283, form1040` exactly as §5 lists; C-multilot added to §4.2 and J5 repointed at it; census authority is J6 alone; J1–J5 attribution corrected (J2 now correctly says slice `form_8283.pdf`, NOT census `f8283`, no Sch A in slice). `admin.rs:226-227` dispatch and `admin.rs:501-510` `{seq}_{name}.pdf` stems verified.
- **I6 ✓** — §7's born-green atom is the `regen == committed` cargo test landing in the same commit as the golden, correctly modeled on the committed-match half of `gen_docs_is_deterministic` (verified present, `docs.rs:352-368`); §9 rescopes atomicity to the P2 wiring commit with the perturb→red proof; §11's P1 row includes the test. The P1-close→P2 gate hole is closed; the three sections are now mutually consistent.

## CONFIRMED: Minors/Nits

M1 ✓ (§4.3 `income import --year --file` verified at `cli.rs:355-366` + `main.rs:236-239`; §14-5 resolved with correct facts) · M2 ✓ (§3.4 routes every production read) · M3 ✓ (`groff -k -man -T pdf` in both §2 and §7) · M4 ✓ (§13 reworded to match §6.3) · M5 ✓ (T-P0.6 caveat verified against `btctax-core/src/optimize.rs:469-484` — `ForbiddenBroker2027` at :477-478 does precede `ContemporaneousNow` at :479-480) · M6 ✓ (§3.3 stderr-scope sentence + §13(d)) · M7+N3 ✓ (filed as FOLLOWUPS UX-P1-2, owning phase P1, bundled) · N1 ✓ (§14-7 notes `underline_color`/`skip`) · N2 ✓ (J5 predicate wording fixed) · N4 ✓ (§6.2 exact-component matching).

## Regression scan — no new Critical/Important

New non-blocking **Minors to carry into the plan phase**:

1. **§5 "carries no census key" is overstated for 3 of 5 slice stems.** `f8949.pdf`/`schedule_d.pdf`/`schedule_se.pdf`'s name components ARE census keys — `admin.rs:501-506`'s own comment records that these three collided and the fix was seq-prefixing the *packet*, not renaming the slice. No gate hole (the §6.1 P1 assertion pins J6==14 independently), but the plan must pin the census scan surface (§6.2(b) "committed corpus + J6 packet manifest") to the J6 manifest / `{seq}_{name}`-shaped names only, or a naive corpus-wide scan blurs "census authority is J6 alone."
2. **§6.2 mechanism 1: "every optional arm populated" is necessary but not sufficient.** Three non-Option gates also bind: `sch_d` gates on `ScheduleDLines::must_file` (`packet.rs:123`), `f8959` on its internal `must_file` (`:149`), and `f8283` is double-gated on the filler returning `Some` (`:155-158`). The mandated count==14 assert makes any shortfall loud (so I2's property holds); note it so the fixture author isn't surprised by a red.
3. **§7 "workspace version from Cargo.toml"** — the root `[workspace.package]` has **no** `version` key; versions are per-crate (`crates/btctax-cli/Cargo.toml:3` = 0.6.1). Plan pins which manifest the generator reads.
4. **§5 footer vs J4** — "Crypto-slice journeys use 2025" but J4 pins `report --tax-year 2024` (deliberate, kitchen-sink oracle-consistency). Table governs; tighten the footer.
5. **Count drift** — §3.4's "~26 in tui-edit/main.rs" greps as 24 today (`OffsetDateTime::now_utc`); FOLLOWUPS UX-P3-1 still says ~30/~28. Harmless given "route every production read" + §14 gap 4, but reconcile the numbers at P3.

Nits: footer line 443 still says "*End SPEC r0*" in an r1 doc; §4.2 vs §6.1 cite two different files both as bare "packet.rs" (btctax-core/src/tax vs btctax-forms/src) and §3.2's "optimize.rs" is `btctax-core/src/optimize.rs` inside a sentence saying "all in btctax-cli" (the *print* sites are CLI-side; qualify the crates); "hyphenated" mis-describes `form8283`/`form1040` (the value list itself is exact).

## The single most important remaining thing

Pin the census scanner's input surface in the IMPLEMENTATION PLAN (Minor 1): scan the **J6 packet manifest only**, because three slice stems are byte-identical to census keys and a corpus-wide scan would silently re-attribute them — the one residual path by which "census authority is J6 alone" could erode in implementation, even though coverage itself stays guaranteed by the J6==14 assertion.

The spec is ready for the user gate and the plan phase.
