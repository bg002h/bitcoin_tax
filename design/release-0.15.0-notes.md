# btctax v0.15.0 — release notes (DRAFT, not published)

Prepared 2026-07-31. **Not tagged, not pushed, not published.** Paste into the GitHub release when the
publish gate clears; the repo has no in-repo CHANGELOG convention (v0.5.0–v0.14.0 all used GitHub
release bodies), so this file is a staging artifact, not a new convention.

Merge commit: `61feb91` — 36 commits from `feat/no-pen-deferrals`.

---

## ⚠️ Fixes a defect that was live in v0.14.0

**Form 8995 line 3 — the prior-year qualified business net loss carryforward — was never modelled.**
Its absence **inflated the §199A deduction and UNDERSTATED tax** for any filer carrying a prior-year
business loss. It is now transcribed, collected in the TUI's "Carryforwards from last year" section,
and carried year-to-year by `--write-carryover`.

If you filed a return with v0.14.0 or earlier **and** had a prior-year qualified-business loss
carryforward, re-run this year's return and compare Form 8995 lines 3, 4 and 16.

## Every form field is now accounted for

The field-provenance census closed: the `GAPS` ratchet went **16 → 0**. Every field on every mapped
form is now either mapped to a value, or recorded as carrying no decision **with a stated reason**.
A blank on a btctax return is now a blank because the inputs say so, never because nothing populated it.

Closed in this release:

- **Schedule C lines I and J** — the Form-1099 compliance pair, now asked.
- **Form 8283 page 2** — went out with no identifying header; a detached Section B page could not be
  tied to its return.
- **Form 8283 lines 5a/5b/5c** — the restriction questions, asked once for the whole return as
  *"did any donation have strings attached?"*. A **Yes** refuses the year rather than deduct at full
  fair market value, because a restricted gift's §170 deduction is smaller (Reg §1.170A-7) and btctax
  cannot tell which gift is affected.

## Married filing separately: the spouse's age-65 and blindness boxes

i1040gi allows a spouse's §63(f) boxes on a separate return *"if your spouse had no income, isn't
filing a return, and can't be claimed as a dependent on another person's return."* btctax captured
only the third condition and counted spouse boxes on joint returns alone — so an entitled filer
forfeited up to two boxes.

All three conditions are now captured and the boxes are claimable. **The gate fails closed:** any
unanswered or adverse condition forgoes the box. Forgoing costs a deduction you can recover by
answering; granting one you are not entitled to understates a signed return, which you cannot.

## Refusals that were too broad

- **A death question no longer blocks every return.** The date-of-death gates moved from mandatory
  declarations to skippable ones: silence now forgoes the age-65 box (overstating tax, the safe
  direction) and says so in an advisory, instead of refusing the return outright.
- **A malformed SSN** no longer blocks `report`, `optimize`, `what-if` or the TUI. It still blocks
  the filed PDF, which is the only place the format matters.

## Smaller changes

- The crypto slice derives **Schedule D line 17** and stamps its 1040 a **WORKSHEET** — it was
  previously mistakable for a filing copy.
- `report` headlines the **all-in** long-term capital-gains marginal rate (23.8% with NIIT, not 20%).
- **A checkbox on-state the widget cannot render is now rejected at fill time.** Previously such a
  value wrote a box that read back as "set" while rendering **blank** on the page.
- `export-irs-pdf` shows the same advisories `report` does; they previously appeared on one path only.
- Prior-year carryovers you have not stated are now named explicitly, rather than silently treated as
  zero.

## Known limitations unchanged in this release

No Form 8275-R (a position contrary to a *regulation* cannot be disclosed); the §1211/§1212 Capital
Loss Carryover Worksheet is unmodeled; carryforward families other than QBI remain import-only; the
Form 8995-A phase-in is out of scope and refuses rather than approximates. See `LIMITATIONS.md`.

## Validation

2542 tests. Five gates: `make check`, `cargo fmt --all --check`, `cargo +1.88 check --workspace
--locked`, `xtask check-isolation`, `scripts/pii-scan-generic.sh`. The TY2024 golden matrix
(md5 `c4e1853ed82d113ca5cd97ffd8abbf47`) is **byte-unchanged across all 36 commits** — every change
above either adds a field that was blank, or moves a figure only for filers whose inputs changed.

Six independent review rounds ran on this branch, the last three scoped to what the earlier ones could
not see. They found 9 blocking defects between them. Reviewer outputs are persisted verbatim under
`reviews/`.

---

## Publish checklist (not yet done)

- [ ] `scripts/.pii-patterns` authored by the owner — **gates both push and publish**
- [ ] revoke the temporary crates.io token from the v0.14.0 publish
- [ ] final pre-publish review (owner-approved model escalation), scoped to publish mechanics:
      inter-crate pins, `include_str!` escapes, tarball contents — **not** tax logic, which has converged
- [ ] `git tag v0.15.0`
- [ ] publish dependency-first, **not** `--workspace`; verify each crate lands via the SPARSE index
      (the crates.io API 403s without a User-Agent)
