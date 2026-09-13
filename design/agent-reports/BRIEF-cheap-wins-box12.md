# BRIEF — FR-197: derive the W-2 box 12 code table instead of typing 11 of it

**Tier:** opus. **Worktree.** No subagents. **You own `crates/btctax-core/src/tax/return_refuse.rs`.**

## 0. The defect, measured

`return_refuse.rs:39` — `const INERT_BOX12_CODES: &[&str] = &["D","E","F","G","H","S","AA","BB","EE","DD","W"]`
— **11 codes** of roughly 26 the IRS defines. Anything else returns
`RefuseReason::UnsupportedBox12Code` and **the packet prints zero pages.**
★ **Code C** (group-term life over $50,000) and **code V** (income from an NSO exercise) are **already inside
box 1 wages**, so admitting them changes **no figure** — yet they block the whole return. That is FR-102's
shape: safe about the arithmetic, total about the outcome.

## 1. What to do — derive, don't extend

**`design/forms/extract/iw2w3--2026.txt` is archived.** It contains the IRS's own box 12 code table. So:
1. **Transcribe the full code list from the extract**, each code with its printed description verbatim.
2. For each code decide, with the instruction text as the authority, whether it is **inert** (the amount is
   already in box 1 or is informational and changes no line on the 1040), **consequential** (it changes a
   line — keep refusing, by name), or **unknown** (refuse, and say why).
3. ★ Make the list **derived or compiler-held**, per `CLAUDE.md`: a hand-typed list of 11 beside a set of 26
   that the IRS revises is the exact disease. If a full derivation is not possible, then **state in the
   source precisely what is covered and what is not** — an honest boundary is reviewable, a silent one is the
   defect.
4. ★★ **Do not simply widen the list to make more returns print.** `widening an exemption is never the safe
   edit`: enumerate the codes that are *provably* inert and default to refusing, so an omission fails closed.

## 2. B1

Plant a code you classify inert and show a return prints; plant a consequential one and show it still
refuses **by name**. Paste both. ★ Also show the refusal message names the code and what to do — a filer
holding a W-2 with code C needs to know why their return stopped.

## 3. Also yours, same file: the retracted paper-check sentences

`return_refuse.rs:2129` and `:2161` tell the filer *"a return with none is complete, and the refund then
arrives as a paper check."* **The IRS retracted that**: `i1040gi--2025.txt:23824-23827` — *"Starting in
October 2025, the IRS will generally stop issuing paper checks for federal disbursements, including tax
refunds, unless an exception applies."*
Rewrite both to say what is **true**: a return with no deposit instruction is still complete and filable, but
**a paper refund cheque can no longer be relied on**, so supplying routing and account numbers is the way to
be paid. ★ Keep the *"or delete the direct-deposit block"* exit — it is a real and lawful exit. Keep the
quoted instruction sentences. Do **not** weaken the strictness argument in the `:368-374` doc comment; update
only its factual clause.
★ Note for accuracy: **direct deposit IS built** (`DirectDeposit { routing, kind, account }`, map-wired,
`screen_direct_deposit`). Only **Form 8888** (splitting a refund across accounts) is unbuilt, and that is
already censused `unmodeled` (`return_inputs.rs:1191`).

## 4. Scope

**IN:** `crates/btctax-core/src/tax/return_refuse.rs` only. **OUT:** `advisories.rs`, `LIMITATIONS.md`,
`printed.rs`, `design/LONG_RANGE_PLAN_filing.md` — another agent owns those; if you see a stale sentence
there, **report it, do not edit it.**

## 5. Working rules

Own worktree; `CARGO_TARGET_DIR=<worktree>/target-box12`. **FOREGROUND everything** (FR-175). `make check`
plus `cargo fmt --all`. **Do not commit or push.**

## 6. Deliverable

Bash heredoc (not `Write`) to `design/agent-reports/REPORT-build-fr197-box12.md`. Short summary + path.
