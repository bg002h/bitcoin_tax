# SPEC — the full-return INPUT SURFACE (P8)

*Ceremony scaled down per `STANDARD_WORKFLOW.md` §8: one spec covering both cycles, no separate plan
document (the build order is §7 below). The gates are NOT scaled down — Fable reviews to 0C/0I.*

---

## 1. The problem

btctax fills a complete, filable 1040 packet (P0–P7 green, validated against two independent engines and
a paper round-trip). **A human still cannot drive it.**

The only way to enter the non-crypto side of a return is:

```
btctax income import --year 2024 --file inputs.toml
```

…where `inputs.toml` deserializes into `ReturnInputs` via serde. There is **no example, no template, no
schema documentation** in the repo. A user cannot discover — without reading Rust — that money fields
are *quoted strings* (`Decimal`), that `owner` is `"taxpayer"` but `filing_status` is `"Mfj"`, or which
fields are required.

**And `income import` never validates.** `screen_inputs` — the ~20-rule fail-closed refuse guard —
appears **zero times** in `cmd/tax.rs`. It runs only later, at `report` and `export-irs-pdf`. So a
malformed SSN, a spouse-owned W-2 on a Single return, or a Schedule C with no business name **parses and
persists silently**, and surfaces much later as a compute-path error with no pointer back to the file.

*The tax is never wrong* — the downstream gate is genuinely fail-closed. This is a usability defect, not
a correctness one. But it is the wrong place to learn you mistyped an SSN.

**And the PII story is incoherent.** The vault encrypts; `income show` masks; `RefuseReason` is careful
never to print SSN digits — and then the input path asks the user to write SSNs, an IP PIN, and
dependents' dates of birth into a **plaintext file on disk, beside the vault**.

## 2. Scope

**IN — Cycle 1 (discoverability + validation):**
- `btctax income template` — a fully commented TOML skeleton, to stdout. No vault, no PII, offline.
- A **drift alarm**: a KAT that fails the build when a new `ReturnInputs` field is not documented.
- **Import-time screening**: `income import` runs `screen_inputs`.
- Parse errors that name the file and point at `income template`.

**IN — Cycle 2 (close the plaintext-PII hole):**
- `btctax income set-pii --year N` — interactive, no-echo prompts for identity; merges into the vault
  row. Identity never touches disk.

**OUT (filed, with rationale):**
- A guided full-return TUI form. `btctax-tui-edit` has **no form engine** — it is per-flow state machines
  and one fixed `[FieldBuffer; 9]`; `ReturnInputs` is dominated by nested collections. That is a new form
  engine plus ~10 flows. Deferred; see §8.
- An external machine-readable schema (JSON Schema). **Declined** — see §4.

## 3. ★ The load-bearing fact that shapes everything

**An EMPTY SSN is deliberately NOT a `screen_inputs` refusal.** From `RefuseReason::SsnMalformed`'s own
doc:

> "An *uncaptured* (empty) SSN is deliberately NOT this: the tax math never reads an SSN, so a household
> that has entered no PII still gets a report. The filable packet is what refuses it."

**Therefore every `screen_inputs` refusal means the data is WRONG, not merely UNFINISHED.** Staged entry
— money now, identity later — is already expressible without tripping the screen. This is what makes
Cycle 2's split coherent: money boxes (merely confidential) may live in TOML; identity
(identity-theft-grade) never needs to.

## 4. Rejected: an external machine-readable schema

I proposed one; the architect declined it and is right.

**The Rust structs already ARE the schema, and they are compiler-audited.** `first_negative_amount`
destructures every input struct with **no `..`**, so adding a field breaks the build until someone
classifies it. Any renderer written *in this repo* consumes the structs directly and inherits that —
**strictly stronger** than conformance to an external file.

An external schema would be a **third representation** to keep in sync, and it cannot express the
validation that actually matters: "the sum of box-12 codes D/E/F/G/S across this owner's W-2s must not
exceed the §402(g) limit." JSON Schema can say *"this is a string."* `screen_inputs` says the truth.

**The real shared validation source already exists. The only missing act is CALLING it at entry time.**

*(A JSON Schema for third-party editor completion — taplo, Even-Better-TOML — is genuinely nice and
cheap via `schemars`. Filed as an optional follow-on. Editor sugar, not architecture.)*

## 5. Design decisions

### D-1 — `income template` emits, it does not write
Prints to **stdout**. No `--out`, no vault access. `btctax income template > inputs.toml` is the idiom,
and piping keeps the command trivially safe: it cannot clobber a file or touch the encrypted store.

### D-2 — The template carries EXAMPLE VALUES, commented out
Every field present. Money demonstrated as a quoted string. Enum variants spelled inline at the field
(`# one of: "Single" | "Mfj" | "Mfs" | "HoH" | "Qss"` — note the trap that `filing_status` is CamelCase
while `owner` is snake_case). Refusal footguns annotated **at the field that trips them** (box-12
allowlist, MFS tri-state, Schedule B Part III, the SALT sales-tax election).

The **PII block is present but COMMENTED OUT**, headed by a plaintext warning and a pointer to
`income set-pii`. A user who wants TOML-only PII can uncomment it; the default path never asks them to.

### D-3 — ★ Import REFUSES on a screen failure; `--force` stores anyway
Given §3 — every `screen_inputs` refusal means the data is **wrong** — the default is **fail-closed**:
print the refusal, write nothing, exit non-zero.

`--force` stores anyway (still printing the refusal), matching the project's existing D-4 `--force` idiom
on `tax-profile set`.

> **The architect argued for warn-and-store** (fail-closed is guaranteed downstream anyway; staged entry
> is designed-in; storing lets the user iterate with `income show`). I take the other side and record the
> disagreement for the reviewer to settle: writing data the engine has already judged *wrong* into an
> encrypted vault the user then trusts is the wrong default for a fail-closed tool, and "the downstream
> gate will catch it" is the reasoning that produced the current defect. Staged entry does not need this
> escape hatch (§3), and where it genuinely does, `--force` is one word.

### D-4 — Screening is ADVISORY about what it cannot see
`screen_inputs` is input-only. The **compute-dependent** refusals (`ScheduleCLoss`, `KiddieTax`,
`QbiAboveThreshold`, `AmtScreenTriggered`) need the assembled ledger and **cannot** be surfaced at entry
— by *any* UI, TUI included. Import must not imply it has cleared the return. The success message says
what was checked and what was not.

### D-5 — `set-pii` MERGES, never clobbers
The precedent exists: `import_return_inputs` already preserves computed carryover across a re-import.
Symmetrically: a money-TOML re-import must **preserve a vault-entered header** when the file supplies
none. Otherwise the user's second `income import` silently wipes the SSNs they entered interactively.
**This is the highest-risk defect in the cycle and gets an explicit KAT.**

## 6. The drift alarm (the schema pipeline, at zero new representations)

A KAT that:
1. takes the template, uncomments its example values, parses it via the **real** `parse_return_inputs_toml`;
2. asserts it equals a golden `ReturnInputs` fixture **constructed without `..Default::default()`** — so
   the *compiler* forces the fixture to name every field;
3. and equality then forces the *template* to carry every field.

**Add a field to `ReturnInputs` ⇒ the fixture fails to compile ⇒ the template must document it.** No
schema file, no codegen, no third representation.

## 7. Build order (TDD; each step red → green)

1. `income template` + the drift-alarm KAT. *(Closes `p1-per-field-subcommands`' disposition.)*
2. `screen_inputs` at import + refusal/parse-error UX + `--force`.
3. `income set-pii` (no-echo prompts) + the merge-not-clobber KAT (D-5).
4. Man pages regenerate (`make docs`), `income show` disposition.

## 8. Follow-ups this phase OWNS (burn down here, per the in-phase rule)

- **`p1-per-field-subcommands`** — disposition. `income template` + `set-pii` IS the answer for v1; the
  per-field editors and the TUI form are deferred with the sizing recorded.
- **`p1-show-as-json-not-toml`** — `income show` emits JSON, so it does not round-trip. With a template
  in hand, **copy-forward is the primary year-over-year workflow**, and it needs `show` to emit TOML.
  Resolve or re-defer with a reason.
- **`p1-ssn-normalization-P6`** — **OVERDUE** (its owning phase has passed). `income import` stores the
  SSN as entered. Decide: canonicalize at capture, or confirm the packet-time canonicalization is the
  single source and close the item.

## 9. Acceptance

- A user who has never read the source can run `btctax income template > inputs.toml`, fill it in, import
  it, and get told **at import** — naming the field — if it is wrong.
- SSNs, the IP PIN and dependents' DOBs can be entered **without ever writing them to disk**.
- Adding a field to `ReturnInputs` **breaks the build** until the template documents it.
- `make check` green; 0 Critical / 0 Important from the independent review.
