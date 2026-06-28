# CONTINUITY — bitcoin_tax (TaxApp)

_Last updated: 2026-06-28 (session 71ab70cd). Written at a pause; safe to exit & restart._

## How to resume (read this first)

1. `cd /scratch/code/bitcoin_tax`
2. Start Claude Code and paste:
   > Resume the bitcoin_tax work. Read CONTINUITY.md and continue from "Next steps". First run `/workflows` to check whether the deep-research run finished.
3. If the deep-research workflow is still listed in `/workflows` as running, let it finish. If it was interrupted by the restart, resume it (see "Background workflow" below) — completed sub-agents return cached results, so resume is cheap.

---

## Project goal

A desktop/local app for **US taxpayers** to compute federal tax on **Bitcoin sales**. It ingests
exchange transaction-history spreadsheets, keeps a **per-lot transaction database**, and helps the
user **choose which lots to sell in the most tax-advantaged way** (specific-ID / FIFO / HIFO,
tax-loss harvesting, short- vs long-term).

## Where we are in the workflow

We adopted a **spec-and-plan-gated, review-to-green** standard workflow (`STANDARD_WORKFLOW.md`,
authoritative via `CLAUDE.md`). Phase order: Brainstorm → Spec → Plan → Implement → whole-diff
review → Ship; every "→ green" is an independent review loop.

**Current phase: RECON** (§3 — research that feeds the spec). Two parallel recon tracks:

### Track A — Legal research (deep-research workflow)  ✅ DONE
- Report saved: **`legal/research/REPORT_us_btc_tax_TY2025-2026.md`** (raw JSON in tasks/wz1qm50yu.output).
- Verification: 6 angles · 23 sources · 112 claims → 25 verified → **25 confirmed, 0 killed** → 10 findings.
- Findings are SOLID on the foundational layer (property classification, realization, holding period,
  basis, spec-ID/FIFO, per-wallet transition, 1099-DA, Form 8949).
- ⚠️ COVERAGE GAPS — the workflow did NOT verify these in-scope topics (primary sources ARE archived;
  they just need a second verification pass before entering the SPEC):
  wash-sale §1091; rate brackets/NIIT/$3,000 loss limit; staking/mining/airdrop/fork income timing;
  gifts & charitable donations; de minimis. See the report's "Open questions".
- ⚠️ Per workflow rules, the report is **recon, not a reviewed artifact**. Its legal claims must be
  verified against the primary sources in `legal/` before anything hardens into the SPEC.

### Track B — Primary-source legal archive ("legal defense")  ✅ DONE
Verbatim primary authority archived locally with provenance (URL + retrieval date + SHA-256) so the
legal basis of every calculation is defensible even if the source sites reorganize.

- **44 documents, ~15 MB**, all from official hosts (irs.gov, govinfo.gov, ecfr.gov), all HTTP 200, all hashed.
  - 10 IRS guidance PDFs (Notices 2014-21, 2023-34, 2024-56, 2024-57, 2025-7, 2026-20; Rev. Rul. 2019-24, 2023-14; Rev. Proc. 2024-28; CCA 202124008)
  - 6 IRS Publications (544, 551, 525, 550, 526, 561)
  - 7 IRS Forms/instructions (8949+i, Sch D+i, 1099-DA+i, 8283)
  - 14 IRC statute sections (govinfo USCODE-2024): §§1, 170, 1001, 1011, 1012, 1015, 1016, 1031, 1091, 1211, 1212, 1221, 1222, 1411
  - 6 Treasury Regs (eCFR 2025-12-01): §§1.1012-1, 1.6045-1, 1.1091-1, 1.1031(a)-1, 1.1015-1, 1.170A-13
  - 1 Federal Register: TD 10000 (89 FR 56480, 104 pp.)
- **Manifest / legal-defense index:** `legal/SOURCES.md` (citation · file · short hash · relevance, with Finding/Open-Q cross-refs).
- **Integrity:** `legal/SHA256SUMS` — verify with `cd legal && sha256sum -c SHA256SUMS` (expect 44 OK).
- **Grep-able text:** `legal/text/` (24 PDFs → .txt via pdftotext; statute/regs already text).
- **Provenance log + scripts:** `legal/_provenance/fetch_log.tsv`, `legal/_scripts/{fetch_sources,fetch_remainder,probe_paths}.sh`.
- Deliberately NOT archived: TD 10021 (Dec-2024 DeFi-broker rule; CRA-repealed 2025; out of scope).

---

## Background workflow (deep-research, Track A)

- Launch Task ID: `wz1qm50yu`
- Run ID: `wf_78e18831-61d`
- Script: `/home/bcg/.claude/projects/-scratch-code-bitcoin-tax/71ab70cd-f674-4e66-86de-cbc9cc49e1a8/workflows/scripts/deep-research-wf_78e18831-61d.js`
- Transcript dir: `/home/bcg/.claude/projects/-scratch-code-bitcoin-tax/71ab70cd-f674-4e66-86de-cbc9cc49e1a8/subagents/workflows/wf_78e18831-61d/`
- **Status at pause:** RUNNING (sub-agents actively writing as of ~13:51).
- **Resume if interrupted:** `Workflow({ scriptPath: "<script path above>", resumeFromRunId: "wf_78e18831-61d" })`
  (stop the old run first with TaskStop if it's still listed). Cached agents return instantly.
- **When it completes:** review the report, save it as `legal/research/REPORT_us_btc_tax_TY2025-2026.md`
  (a recon artifact, not yet reviewed), then verify its claims against the `legal/` primary sources.

---

## Next steps (in order)

1. ~~Track A: save research report.~~ ✅ DONE → `legal/research/REPORT_us_btc_tax_TY2025-2026.md`.
2. ~~Finish Track B archive (statute + regs + TD 10000 + Notice 2026-20 + CCA 202124008) + `SOURCES.md` + hashes.~~ ✅ DONE.
3. ~~Extract PDFs to grep-able text in `legal/text/`.~~ ✅ DONE (24 PDFs).
4. ~~Close the report's 5 open questions (verify against archived primary sources).~~ ✅ DONE →
   `legal/research/ADDENDUM_open_questions_verified.md` (added §1223, §61, CCA 202302012 to archive;
   SHA256SUMS now 47/47). Headlines: §1091 wash-sale does NOT apply to crypto; rate/NIIT/loss-limit
   confirmed; income timing = FMV at dominion-and-control; gift carryover/dual basis + charitable FMV-if->1yr
   + qualified appraisal >$5k; NO de minimis. (Addendum still owes the SPEC's independent-review gate.)
5. **← NEXT: leave RECON, begin Phase A — Brainstorm** for the app (no code before an agreed design).
   Recon inputs ready to feed it: the report + addendum + the 47-doc primary-source archive.

## Done this session (no action needed)
- Adopted standard workflow: `STANDARD_WORKFLOW.md` (byte-identical copy of `~/.claude/STANDARD_WORKFLOW.md`),
  `CLAUDE.md` (declares it authoritative), and a project memory pointer.
- Built `legal/` archive skeleton + 21 IRS PDFs + provenance log + scripts.
