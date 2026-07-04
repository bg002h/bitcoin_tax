# R0 — SPEC_crate_publishing.md — round 2 (verification of the round-1 fold)

**Artifact:** `design/SPEC_crate_publishing.md`
**Baseline:** branch `feat/crate-publishing` @ `7725fa6` (fold commit "spec(crate-publishing): fold R0 round 1"); `main` == `2bd11ba` ✓ (spec's stated source baseline).
**Reviewer role:** independent architect (R0), round-2 verification. Read-only; no Cargo.toml edits, no `cargo publish`, no branch switch, token never printed.
**Toolchain in env:** `cargo 1.97.0-nightly` (workspace MSRV `1.88`) — `cargo publish --workspace` and `cargo package --workspace` are both present (`--help` confirmed).
**Round 1:** 0C / 1I / 2M / 2N (I1 dry-run methodology; M1 categories; M2 v0.1.0-burned; N1 bare-`btctax`; N2 real-publish `--workspace`). Bar: 0 Critical / 0 Important.

## Verdict: 0 Critical / 0 Important / 1 Minor / 1 Nit — **R0-GREEN**

All five round-1 findings are folded correctly and confirmed against source + upstream docs. The coordinated `--workspace` dry-run is the right last gate and is genuinely offline-verifiable on this toolchain; the category slugs are real; the go-ahead gate now carries every irreversibility fact a user needs. The publish plan will work. Two non-blocking items remain (neither gates GREEN): a Minor operator-awareness gap — the crates.io **new-crate 5-burst rate limit** means a single `cargo publish --workspace` of **6 brand-new** crates will upload 5 and then be throttled on the 6th (`btctax-tui-edit`), requiring a ~10-minute retry; and a Nit about license/reuse consent. **Recommend folding the Minor into Task 3 / Gotchas before the irreversible run**, but it does not block the SPEC gate.

---

## Fold verification (each round-1 finding)

### [I1 — the blocker] Coordinated dry-run — RESOLVED, correct.
Spec §Verification (lines 59–65) + Task 2/3 now use `cargo publish --dry-run --workspace --allow-dirty` and explicitly explain why the per-crate form fails ("verify build extracts the packaged crate with `path` STRIPPED and resolves siblings from the REGISTRY (not yet published) → `no matching package named 'btctax-core' found`"). Verified correct:
- **`--workspace` exists on this toolchain.** `cargo publish --help` shows `--workspace  Publish all packages in the workspace`; `cargo package --help` shows the same. So the fold's prescribed command runs on 1.97.
- **Mechanism confirmed (cargo ≥1.90 workspace publishing).** Upstream docs/announcements confirm the workspace publish creates a **temporary, initially-empty local registry**, stores each just-packaged member into it, and runs coordinated verification building the **full set as if published**, in **topological order** — so inter-member deps resolve **offline** with nothing on crates.io. That is exactly the spec's characterization ("packages ALL members then verifies each against the just-packaged siblings OFFLINE, in topological order, WITHOUT uploading"). The per-crate `--dry-run -p <downstream>` genuinely would fail as the spec now states.
- **`--no-verify` fallback correctly characterized.** Spec: "per-crate `--dry-run --no-verify`, which packages but skips the build-verify." Accurate — `--no-verify` ("Don't verify the contents by building them") packages + checks metadata only, trading away the build check. Correct.
- **No residual spot implies a per-crate dry-run works.** I re-scanned the whole spec. The only per-crate `-p` reference left is the **real-publish fallback** (line 81, `cargo publish -p <crate>` in topological order) — which is correct precisely because each upstream is actually on the index by then (round-1 I1 explicitly blessed the in-order real publish). No dry-run anywhere implies per-crate resolution from `path`. Clean.

### [M1] Categories per-crate — RESOLVED, correct; slugs verified REAL.
Spec §Changes 2 (lines 46–49) now sets `categories` **literally per crate** and NOT in `[workspace.package]`: libs (core/store/adapters) `["finance"]`; bins (cli/tui/tui-edit) `["command-line-utilities","finance"]`. The round-1 self-contradiction (inherit-one-value-but-vary-for-bins) is gone.
- **Slugs are real crates.io categories.** Verified against the canonical `rust-lang/crates.io` `src/boot/categories.toml`: both `[finance]` and `[command-line-utilities]` exist as top-level category slugs. Neither is wrong; upload will not be rejected for an unknown slug.
- **`keywords`/`repository`/`homepage` correctly still shared via `[workspace.package]`.** These apply identically to all 6 crates (same repo, same 5 keywords `bitcoin/tax/cryptocurrency/accounting/ledger` — exactly 5, ≤5 cap), so inheritance is right; only `categories` legitimately varies (lib vs bin), which is why per-crate is correct for it and shared for the rest. Consistent.

### [M2 / N1 / N2] Go-ahead gate — RESOLVED, complete.
§Go-ahead gate (lines 69–81) now states, plainly:
- **Names permanent** (line 71): "6 crate names will be permanently claimed (yank ≠ release; a name is never freed)." ✓
- **[M2] v0.1.0 permanently burned** (lines 72–73): "even after a yank, that exact version can NEVER be re-published; any fix must ship as 0.1.1." ✓
- **Source public regardless of repo privacy** (lines 74–75): "publishing exposes the source regardless of whether the repo is private." ✓
- **[N1] bare `btctax` name** (lines 77–78): the `btctax` binary ships from the `btctax-cli` crate; bare `btctax` is not among the six; asks whether the user wants to reserve/publish it. ✓
- **[N2] real publish via `cargo publish --workspace` from a clean committed tree, NO `--allow-dirty`** (lines 79–81), with the per-crate in-order fallback. ✓

Nothing else material is missing for informed consent (the bundled public price CSV is also flagged, line 76). See Nit N1 below for one optional addition.

---

## No-regression / residual-gap checks (all re-verified against source)

- **Publish order is still a valid toposort.** Re-grepped all internal edges: core=none; store=none; adapters→core; cli→core/store/adapters; tui→cli/store/core/adapters; tui-edit→tui/cli/core/store/adapters (`xtask→cli` but `publish=false`). Order core→store→adapters→cli→tui→tui-edit publishes every dep before its dependent. ✓
- **Path→version edge set complete.** All 13 internal edges live in `[dependencies]`; a full per-crate table scan found **no** internal `btctax-*` ref in any `[dev-dependencies]`, `[build-dependencies]`, or `[target.'cfg(...)'.dependencies]` table (store's target tables and every dev-deps table are external-only; adapters even documents removing a redundant internal dev-dep). Spec's enumeration misses nothing; nothing new has crept in since round 1. ✓
- **`description` is still the only missing REQUIRED field.** `license.workspace = true` present in all 6 (`[workspace.package].license = "MIT OR Unlicense"`, valid SPDX); name+version present (`version = "0.1.0"` literal per crate, line 3 of each — not workspace-inherited, matching the spec). `keywords/categories/repository/homepage/documentation/readme` are all optional; only `description` is hard-required. ✓
- **Clean-tree real publish (no `--allow-dirty`) with gitignored vault files — CONFIRMED SAFE.** cargo's dirty check ignores `.gitignore`d files; only **untracked-but-not-ignored** files force `--allow-dirty`. `.gitignore` covers `*.pgp/*.gpg/*.asc/vault*/*.vault/*.sqlite*/*.db/*.xlsx/*.coinbase.*.csv/ /data/ /samples/` etc.; `git status --porcelain` is **clean** (empty), all vault/PII/tooling files show as `!!` ignored. Critically, **no `include=`/`exclude=` in any Cargo.toml** — so the known false-positive (cargo #16872, where `package.include` makes cargo wrongly flag gitignored files as uncommitted) does **not** apply here. The bundled `crates/btctax-adapters/data/btc_usd_daily_close.csv` is tracked and NOT ignored (`/data/` is root-anchored; `git check-ignore` confirms not-ignored) — it ships as intended. So `cargo publish --workspace` from the committed metadata tree will not trip the dirty check without `--allow-dirty`. The spec's forbidding `--allow-dirty` on the real publish is correct. ✓
- **No sensitive tracked file packaged.** A repo-wide `git ls-files` scan for secret extensions/`vault` returns only `design/SPEC_vault_half_created_repair.md` and `reviews/R0-vault-half-created-repair-round-1.md` — both are docs matching on the word "vault," living at repo root **outside every crate dir**, so never packaged. Round-1's no-leak clearance still holds. ✓

---

## Findings (non-blocking; do not gate GREEN)

### [Mnew-1] Minor — 6 brand-new crates exceed the crates.io 5-crate new-crate burst; the real `--workspace` publish will throttle on the 6th
crates.io rate-limits **new crate name** creation with a leaky-bucket: a burst of **5 new crates** per account, then **1 new crate per 10 minutes**. This workspace publishes **6 brand-new** crates in a single `cargo publish --workspace`. In practice cargo will upload the first 5 in topological order (core, store, adapters, cli, tui) and then be **rate-limited (HTTP 429) on the 6th** (`btctax-tui-edit`) — a hard error at the very last, irreversible step. This is a known interaction (crates.io issue #1643, "Publishing workspaces with large numbers of crates hits the rate limit").

Why it's only Minor, not Important: it is **safe and fully resumable** — each of the 5 that succeeded is permanently published, and because the workspace publish already coordinated-verified all 6 against one another, the throttled 6th just needs a **re-run** (`cargo publish --workspace` again, or `cargo publish -p btctax-tui-edit`) after ~10 minutes; nothing is corrupted, misordered, or version-burned incorrectly. But the spec's Task 3 / Go-ahead currently describe `--workspace` as a smooth single run that "uploads in dependency order and waits for each crate to hit the index," with no mention of the burst cap — so an operator following it verbatim would hit an unexpected 429 mid-irreversible-action and could misread it as a real failure.

**Fix (recommend folding into Task 3 + Gotchas before the run):** note that 6 new crates > the 5-burst limit, so expect the 6th to be rate-limited (~10-minute wait), and that re-running `cargo publish --workspace` (or `-p btctax-tui-edit`) finishes it — already-verified, safe, resumable. (A request to crates.io for a rate-limit bump is the alternative, but the retry is simpler.)

### [Nnew-1] Nit — go-ahead gate could name the license-reuse consequence, not just "source becomes public"
The gate says the source becomes public, which is the main point. One optional addition for fully-informed consent before an irreversible public act: publishing under the workspace's `MIT OR Unlicense` makes the code **freely reusable/redistributable/relicensable by anyone, permanently** (Unlicense is effectively public-domain dedication). This is already implied by "source becomes public" and the license is a pre-existing committed choice, so it's a nicety, not a gap. Optional one-liner; not blocking.

---

## Bottom line
The round-1 fold is correct and complete: the coordinated `--workspace` dry-run is the right offline-verifiable last gate on this 1.97 toolchain (per-crate dry-run genuinely fails as now stated), `finance`/`command-line-utilities` are real slugs set per-crate, `keywords/repository/homepage` are correctly shared, and the go-ahead gate carries every irreversibility fact (names permanent, v0.1.0 burned, source public regardless of repo privacy, bare-`btctax`, `--workspace` from a clean tree). Publish order + the 13 path→version edges + `description`-only-missing all re-verified; the clean-tree/no-`--allow-dirty` real publish is safe because gitignored vault files are treated as clean and no `include=` triggers the false-positive. **0 Critical / 0 Important → R0-GREEN.** Fold the Minor (new-crate 5-burst rate limit → expect a throttle + retry on `btctax-tui-edit`) into Task 3/Gotchas before executing the irreversible publish so the operator isn't surprised; the Nit is optional.
