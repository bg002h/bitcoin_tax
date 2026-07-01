# R0 architect review — SPEC_gift_chunk2_donee.md (round 1)

- **Artifact:** `design/SPEC_gift_chunk2_donee.md`
- **Baseline verified against:** HEAD `3a405f0` (`git rev-parse HEAD` = 3a405f0fa341…), matches the spec's declared baseline.
- **Verdict:** **BLOCKING — NOT 0C/0I.** 1 Critical, 2 Important, 3 Minor, 2 Nit.
- **Gate:** implementation may not proceed until C1 + I1 + I2 are folded and this loop re-runs to green.

---

## Highest-priority determinations (the two the mandate demanded)

### (a) Vault serialization format + does `#[serde(default)]` actually give back-compat?

**Format = JSON (self-describing).** `persistence.rs` stores the payload as a TEXT column
`payload_json` via `serde_json::to_string(&ev.payload)` (`persistence.rs:165`) and reads it back
with `serde_json::from_str(&payload_json)` (`persistence.rs:290`). It is **not** bincode / not a
non-self-describing format. So for the *general* case, `#[serde(default)]` on a newly-added
**struct field** does provide back-compat — and there is live precedent: `AllocLot.dual_loss_basis`,
`AllocLot.donor_acquired_at`, `SafeHarborAllocation.pre2025_method` (all `#[serde(default)]`,
`event.rs:151/153/167`).

**BUT the spec's specific change is NOT a struct-field add — it converts an enum UNIT variant into a
STRUCT variant, and for that case `#[serde(default)] does NOT help.** `OutflowClass` is an
externally-tagged enum (no `#[serde(tag=…)]`, `event.rs:104-109`). Today `GiftOut` is a **unit
variant** (`event.rs:107`) → it serialized into existing vaults as the bare JSON string `"GiftOut"`.
Turning it into `GiftOut { donee: Option<String> }` makes it a **struct variant**, which serde
deserializes only from a map `{"GiftOut":{…}}`. The mismatch is rejected at the *enum-shape* level,
before any field-defaulting runs.

**Empirically verified** (throwaway serde_json test built against this workspace's resolved deps, then
deleted):

```
OLD GiftOut serialized as: "GiftOut"
OLD Donate  serialized as: {"Donate":{"appraisal_required":true}}
NEW deserialize of OLD GiftOut: Err(Error("invalid type: unit variant, expected struct variant", line: 0, column: 0))
NEW deserialize of OLD Donate:  Ok(Donate { appraisal_required: true, donee: None })
```

So: **Donate is safe** (already a struct variant → adding `#[serde(default)] donee` works, proven
above). **GiftOut is BROKEN** — and because `load_all` propagates the first `from_str` error via `?`
(`persistence.rs:290`), a *single* pre-existing gift-out event makes the **entire vault fail to
open**. This is exactly the "existing vaults fail to load" failure the mandate flags as Critical →
**C1** below.

### (b) §2503(b) per-donee confirmation

**Confirmed, independently web-verified.** The annual exclusion is **per donee**, not an aggregate:
each recipient has their own exclusion; two donees at $15k each = **$0 taxable** even though the $30k
aggregate exceeds one exclusion. **TY2025 = $19,000** (also 2026). Form 709 is required when gifts to
**any single donee** of present-interest property exceed the annual exclusion; **future-interest**
gifts require filing regardless of amount (not detectable by this model — correctly disclosed);
gift-splitting is a §2513 election (out of scope, correctly disclosed). The spec's legal core (D3) is
sound. Sources: IRS Instructions for Form 709 (2025); IRS "Frequently asked questions on gift taxes";
IRS Gifts & Inheritances FAQ.

---

## Findings

### C1 — CRITICAL — `OutflowClass::GiftOut` unit→struct conversion breaks vault back-compat

**Where:** Design §D1 (spec lines 49-51, 85-86), SemVer claim (lines 12-13), and the back-compat
assertion "existing vaults deserialize (donee → None)".

**What:** As proven in (a), converting the `GiftOut` unit variant to a struct variant makes every
legacy `"GiftOut"` payload undeserializable; `load_all` then fails the whole vault. The spec's entire
back-compat claim rests on `#[serde(default)]`, which does not apply to a unit→struct enum-shape
change. The spec is also **internally contradictory**: it *mandates* a back-compat KAT (Task 1) that
its own design would fail.

**Fix (recommended — Option 1, minimal + precedented):** Do **not** put `donee` on the
`OutflowClass` variants at all. Add it to the **`ReclassifyOutflow` struct** instead:

```rust
pub struct ReclassifyOutflow {
    pub transfer_out_event: EventId,
    pub as_: OutflowClass,                 // UNTOUCHED — GiftOut stays a unit variant
    pub principal_proceeds_or_fmv: Usd,
    pub fee_usd: Option<Usd>,
    #[serde(default)]
    pub donee: Option<String>,             // NEW — struct-field add → back-compat SAFE (proven)
}
```

`ReclassifyOutflow` is always serialized as a map, so a `#[serde(default)]` optional field is genuine
back-compat (identical to the proven Donate case, and to the three existing `#[serde(default)]`
precedents). It is also *precedented in shape*: `principal_proceeds_or_fmv` and `fee_usd` already sit
at the `ReclassifyOutflow` top level even though they are conceptually per-class. Then:

- `resolve.rs:211/217`: keep `OutflowClass::GiftOut =>` / `Donate { appraisal_required } =>`
  **unchanged**; pass `ro.donee.clone()` into `Op::GiftOut { …, donee }` / `Op::Donate { …, donee }`
  (the enclosing `if let Some(ro) = …` already reads `ro.principal_proceeds_or_fmv`/`ro.fee_usd`).
- `state.rs` Removal, `fold.rs` both push sites, CLI `--donee`, removals.csv, Form 8283, D3 advisory:
  **exactly as the spec already has them** (they consume `Removal.donee`, which is unaffected by where
  the field lives on the event).

This *reduces* the touch surface vs. the spec: `OutflowClass` and all ~10 of its construction/match
sites stay byte-identical (`main.rs:709`, `reconcile.rs`, and tests at `reconcile.rs:128`,
`properties.rs:321`, `verify_report.rs:873`, `kat_tax.rs:564/954/1102/2049/2308`), so only the
`ReclassifyOutflow {…}` literals gain `donee: None`.

**Alternatives considered / rejected:** (2) a hand-written `Deserialize` for `OutflowClass` that
accepts both the legacy `"GiftOut"` string and the new map — keeps donee on the class but adds
bespoke unsafe-ish code + its own KAT; heavier, not right-sized. (3) a second variant `GiftOutTo` —
ambiguous no-donee case, ugly. (4) a load-time migration rewriting `"GiftOut"` → map — no migration
framework exists; disproportionate. **Option 1 is the right-sized, safe fix.**

### I1 — IMPORTANT — the mandated back-compat KAT must be re-specified for the corrected design

**Where:** Task 1 (spec lines 99-101).

The spec's KAT text — "a serialized `OutflowClass::GiftOut`/`Donate` JSON WITHOUT the donee field
deserializes to `donee: None`" — is ambiguous/wrong under the fix (post-fix, `GiftOut` never has a
donee field; the donee lives on `ReclassifyOutflow`). Re-specify the lock to pin **explicit legacy
`payload_json` strings** as they exist in real vaults today, e.g.:

- `{"ReclassifyOutflow":{"transfer_out_event":…,"as_":"GiftOut","principal_proceeds_or_fmv":"100.00","fee_usd":null}}`
  → deserializes to `ReclassifyOutflow { …, donee: None }` (this is the string form that C1 would have
  broken — it is the single most important assertion in the chunk).
- legacy Donate: `…"as_":{"Donate":{"appraisal_required":true}}…` (no top-level donee) → `donee: None`.

Keep the round-trip tests too (`--donee "Alice"` → `Some("Alice")`; no `--donee` → `None`). The KAT
should assert *deserialization succeeds*, not merely that the field defaults — the failure mode is a
hard `Err`, not a wrong value.

### I2 — IMPORTANT — D3 refactor must preserve two load-bearing behaviors + update existing tests

**Where:** Design §D3 (spec lines 66-82), Task 3 (109-119).

`render_gift_advisory` today has two non-obvious safety behaviors the refactor must **not** silently
drop:

1. **[R0-m6] table-unavailable → emit a note, never `None`** (`render.rs:1210-1215`): if gifts exist
   but no bundled table has the exclusion, it must still warn rather than return `None` (silently
   hiding Form 709 exposure).
2. **`any_gift` guard independent of total** (`render.rs:1194-1201`): a zero-FMV gift still counts as
   "gifts present".

The spec's D3 rewrite describes per-donee grouping but says nothing about carrying these forward, and
Task 3's "Files" lists only *new* KATs — implying the existing `gift_advisory_tests` module
(`render.rs:1567+`) is untouched. It is not: the over-exclusion assertion pinning `"donee identity is
not modeled"` (`render.rs:1632`) becomes stale and the no-table-note test (~`render.rs:1660`) must
stay green. Spec must explicitly (a) preserve the m6 note branch + any_gift guard, and (b) revise the
existing `gift_advisory_tests`. (Partial backstop: the existing tests will fail loudly if dropped —
but the spec should still call this out so it is designed, not discovered.)

### M1 — MINOR — citation drift (non-blocking; correct before folding)

- `forms.rs` `Form8283Row.donee` is at **line 288** (struct at 260), not "~278" (spec line 40/41).
- `render_gift_advisory` is at **`render.rs:1189`** (doc from 1178, body to 1233), not "~1153-1197"
  (spec line 36). The spec already said "re-verify" — the corrected span is 1178-1233.
- All other cited lines resolve cleanly: `event.rs` Income.business = 57 ✓, ReclassifyOutflow
  110-116 ✓, OutflowClass 104-109 ✓; `resolve.rs` Op 72-86 ✓, mapping 211-217 ✓; `state.rs` RemovalLeg
  149-163 / Removal 164-176 ✓; `fold.rs` gift push 1017 / donate push 1139 ✓; CLI `main.rs` 218-228 /
  695-717 ✓, `reconcile.rs` 54-72 ✓.

### M2 — MINOR — `donee` on a `Dispose` reclassify is meaningless under the fix

Putting donee at `ReclassifyOutflow` level means a Sell/Spend reclassify can nominally carry a donee.
It is harmless (resolve's `Dispose` arm simply never reads it, and the CLI only sets `--donee` for
gift/donate). Document that donee is ignored for `Dispose`; no validation needed (gold-plating).

### M3 — MINOR — removals.csv `donee` column contract

`donee` is a per-`Removal` label (not a summable number like `claimed_deduction`, which is
leg-0-only, `render.rs:664-666`). Emit it on **every** leg row (like `event`/`kind`/`removed_at`) and
**append it at the end** of the header/record (after `claimed_deduction`, `render.rs:646-657`) so the
existing column contract / any snapshot fixtures are not shifted. Spec says only "header + record" —
make the position + per-leg repetition explicit.

### N1 — NIT — Form 8283 donee: carrier-row vs all-legs

D2 (spec line 63) says populate `donee` on the "carrier row". Consistent with `fmv_method`
(leg-0-only, `forms.rs:386-390`). Fine — but `donee` is *identifying* (like description/dates, which
are on every leg), so all-legs would also be defensible. Pick one explicitly to avoid an
implementation coin-flip.

### N2 — NIT — verify the Rev. Proc. subsection cite

The $19,000 TY2025 figure is correct, but confirm the exact citation "Rev. Proc. 2024-40 §2.43"
(spec line 17) points to the annual-exclusion subsection before shipping the legal-grounding text.

---

## Cross-checks that PASSED (no findings)

- **Standalone / no engine-B leak (spec §7):** `grep` of `crates/btctax-core/src/tax/` shows
  `compute_tax_year` never reads `removals` at all — donee (a Removal field) cannot reach engine B.
  Removals recognize no gain (TP10); `Removal.claimed_deduction` is the existing precedent for a
  Removal field explicitly excluded from engine B (`state.rs:172-175`). The "assert a tax golden
  unmoved" KAT is the right guard (goldens in `tax_compute.rs` / `kat_tax.rs`). Confirmed achievable.
- **Fingerprint / dedup unaffected:** `ReclassifyOutflow` is a decision payload → `fingerprint()`
  returns `None` for it (`persistence.rs:96`), so adding a field does not touch FR1 dedup.
- **Compile blast-radius of `Removal.donee`:** no `Removal { … }` destructuring patterns exist
  anywhere (only `push`/struct-def/test constructors), so no exhaustiveness breaks; `Op::GiftOut` /
  `Op::Donate` match sites all use `..` (`resolve.rs:794/807-808`, `evaluate.rs:79-80`,
  `fold.rs:958/1028`) → adding an Op field is compile-safe; fold must add `donee` to its binding.
- **Touch-site completeness (D1 chain):** every site the spec names is real and present —
  event → resolve (211/217) → Op (73-86) → state Removal (164-176) → fold ×2 (1017/1139) → CLI
  (`main.rs:218-228/709`, `reconcile.rs:54-72`) → removals.csv (640-685). Mirrors the Income.business
  data-flow (`event.rs:57` → `resolve` `Op::IncomeInbound{business}` → `IncomeRecord.business`
  `state.rs:184` → income.csv `render.rs:694/703`). The ONLY defect is the serde mechanism for
  GiftOut (C1), not the flow shape.
- **D3 legal design:** per-donee grouping, labeled-donee taxable = `max(0, total − excl)`, filing
  trigger on any labeled donee > excl, Gifts-only for 709 with Donations excluded, donee-on-both for
  8283, and the None→"unlabeled" conservative-aggregate bucket (never silently dropped, with a
  label-them caveat) are all sound and match the confirmed §2503(b)/Form 709 rules. The conservative
  aggregate for the unlabeled bucket can only *over*-flag (safe direction). Ensure the unlabeled
  aggregate is reported as a caveat, not folded into a precise "taxable" total (don't fabricate a
  per-donee determination for unlabeled gifts).

---

## Required before green (re-review after fold)

1. **C1** — rewrite D1 to place `#[serde(default)] donee: Option<String>` on `ReclassifyOutflow`
   (leave `OutflowClass` untouched); update the SemVer/back-compat prose accordingly.
2. **I1** — re-specify the back-compat KAT to pin explicit legacy `payload_json` strings (legacy
   `"GiftOut"` string + legacy Donate map, both without donee → deserialize OK, `donee: None`).
3. **I2** — state that D3 preserves the [R0-m6] table-unavailable note + the `any_gift` guard, and
   revises the existing `gift_advisory_tests`.
4. Fold M1-M3 (citations, Dispose-donee note, removals.csv column contract); address N1-N2.

---

# Round 2 — re-review (post-fold)

- **Artifact:** `design/SPEC_gift_chunk2_donee.md` (revised).
- **Baseline re-verified:** `git rev-parse HEAD` = `3a405f0fa341…` — unchanged; spec's declared baseline still current.
- **Scope:** confirm C1 + I1 + I2 folds; scan for NEW C/I; per mandate, JSON format and §2503(b) per-donee are settled and NOT re-litigated.
- **Verdict:** **GREEN — 0 Critical / 0 Important.** C1, I1, I2 all CLOSED; no new C/I introduced. Ready to implement. (Residual: 2 carried-over Minors + 1 new Minor + 2 Nits — all non-blocking; fold-while-implementing or track in FOLLOWUPS.)

## C1 — CLOSED (verified against source)

The fold is the correct back-compat-safe **struct-field-add**, not the broken unit→struct enum change.

- **`OutflowClass::GiftOut` stays a UNIT variant** — `event.rs:105-109` unchanged in the design; legacy vaults' bare-string `"GiftOut"` payloads still deserialize (the exact failure C1 flagged is avoided by construction).
- **`donee` lives on the `ReclassifyOutflow` STRUCT** with `#[serde(default)]` (`event.rs:111-116`). This is a plain named-field struct always serialized as a JSON map, so an absent `donee` key defaults to `None` — identical mechanism to the live precedents `AllocLot.dual_loss_basis` / `donor_acquired_at` / `SafeHarborAllocation.pre2025_method` (`event.rs:151/153/167`) and to `ReclassifyOutflow`'s own `fee_usd`. Round-1's empirical test already proved the struct-field case round-trips (`Donate → donee: None`); the only broken path (GiftOut unit variant) is now untouched.
- **Touch-site chain complete + correct:** `ReclassifyOutflow.donee` (event.rs:111-116, NEW) → `resolve.rs:209` `if let Some(ro) = …` already borrows the struct, so `ro.donee.clone()` feeds `Op::GiftOut`/`Op::Donate` at the unchanged match arms (`resolve.rs:211-223`) → add `donee` to both `Op` variants (`resolve.rs:73-86`) → `Removal.donee` (`state.rs:165-176`) → both fold pushes (`fold.rs:1017/1139`) → CLI `--donee` (`main.rs`, `cmd/reconcile.rs:65`) → removals.csv → Form 8283 (D2) / Form 709 (D3). Every hop verified present in current source.
- **Diff-shrink claim holds:** all `OutflowClass::GiftOut` construction/match sites stay byte-identical; only the `ReclassifyOutflow { … }` literals gain `donee: None`.

## I1 — CLOSED

Task 1 now pins **explicit legacy JSON**: a bare-string `"as_":"GiftOut"` record + a legacy `Donate` map, neither carrying `donee`, asserting `serde_json::from_str::<EventPayload>` **SUCCEEDS** with `donee: None`. This is the right lock: the failure mode C1 described was a hard `Err` at the enum-shape level (whole vault fails to open), and this KAT asserts *successful deserialization*, not merely a defaulted value. The `"GiftOut"` bare string is the single most important assertion and it is now explicitly present. Adequate.

## I2 — CLOSED (verified against source)

D3 (spec lines 91-95) now explicitly carries forward both load-bearing behaviors and fixes the stale test:

- **[R0-m6] table-unavailable → note (never `None`)** — confirmed live at `render.rs:1209-1215` (`None => Some(format!(… table unavailable …))`). Spec commits to preserving it.
- **`any_gift` guard independent of total** — confirmed live at `render.rs:1194-1201` (zero-FMV gift still counts). Spec commits to preserving it.
- **Stale test** — the `"donee identity is not modeled"` assertion is real at `render.rs:1632` (inside `gift_advisory_tests`, `render.rs:1567+`); D3 revises it with per-donee assertions. Confirmed the string exists exactly where cited.

## No new Critical / Important

Internal consistency, right-sizing, and TDD-completeness all hold. Legal design intact and unchanged: per-donee §2503(b) (not aggregate), Gifts-only for 709, donee-on-both for 8283, unlabeled bucket kept as a conservative caveat (never silently dropped), standalone (no engine-B leak; tax golden asserted unmoved). Corrected citations verified: `forms.rs` `Form8283Row.donee` = **288** ✓; `render_gift_advisory` = **1189** ✓ (doc 1178-1188, body to 1233).

## Residual (all Minor/Nit — NON-BLOCKING; do not reopen the gate)

- **M4 (new, Minor) — Task-1 `Files` list omits the compile-forced fixture updates.** With no `Default` on `ReclassifyOutflow`, every struct literal must add `donee: None` to compile — ~24 sites the plan doesn't list: `kat_tax.rs` (×20: 562/608/654/952/963/1100/1394/1492/1577/1716/1775/1983/1995/2047/2258/2306/2729/2786 …), `transition.rs:490`, `lot_selection.rs:382`, `properties.rs:319`, and `event.rs:349`. Cannot cause a silent defect (compiler + the green gate force it), but the plan's file list should enumerate them so scope/review is complete.
- **M2 / M3 (carried over, Minor) — not folded.** M2 (note that `donee` is ignored for a `Dispose` reclassify) and M3 (removals.csv contract: emit `donee` on every leg row, appended at end of header/record) are still only "header + record". Non-blocking; fold before/at implement or track.
- **Nit (new) — `render_gift_advisory` docstring goes stale.** Lines 1178-1188 still say "donee identity is not modeled / total-exposure signal, not a per-donee determination." The D3 rewrite must update it (parallels D2's explicit "update the now-inaccurate doc/comment"); implied by "refactor," but worth stating.
- **Nit (new) — D1 phrasing "set `donee: op.donee`".** The fold arms destructure with `..` (`Op::GiftOut { sat, fmv, fee_sat, .. }`, `Op::Donate { … , fee_sat, .. }`); the implementer must add `donee` to the pattern binding. Intent unambiguous; trivial.
- **N1 — resolved** (spec explicitly picks the carrier row for Form 8283 `donee`, matching `fmv_method`). **N2** — Rev. Proc. §-cite left as-is (per mandate, §2503(b) not re-litigated).

**Bottom line:** the three blocking findings are genuinely closed against current source and the folds introduce no new Critical/Important issue. **The spec is R0 GREEN and ready to implement.**
