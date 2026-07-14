# P6 GATE REVIEW — r2-fold verification (`8a56158`) — Fable r3

Persisted verbatim, per STANDARD_WORKFLOW §2.

**VERDICT: 0 Critical / 0 Important / 2 Minor / 1 Nit — the P6 GATE IS CLOSED. The phase may ship.**

---

**Reviewer:** Fable (independent; same reviewer as r1/r2, adversarially verifying the folds of my own findings).
**Scope:** the fold `8a56158` (= the entire `7312f4e..HEAD` delta, diff-stat-confirmed identical to the commit), the whole phase `b40bdec..HEAD`, SPEC §3.1 as extended, FOLLOWUPS.

**Gates re-verified by me, not taken on faith:** `cargo test --workspace --locked` = **1685 passed / 0 failed** across 83 suites (+2 vs r2 = the two new KATs; the disjointness KAT was a rewrite, not a net-add — arithmetic reconciles). `cargo clippy --workspace --all-targets --locked -- -D warnings` = exit 0. `cargo fmt --all -- --check` clean. `cargo run -p xtask -- docs` regenerates every man page with a **clean tree**. FROZEN files (`tax/{types,compute,se}.rs`) = **0 bytes** since `059ec2a` (empty diff).

## Per-finding verification

### NEW-I1 (QDCGT third operand) — **FIXED**

`Form1040Income.qdcgt_net_capital_gain = sch_d.line15.min(sch_d.line16).max(Usd::ZERO)` (`printed.rs:546`), threaded to `qdcgt_line16`'s third operand (`printed.rs:618`). I walked **every** §7.2 routing path against the worksheet's own line-3 rule ("smaller of Sch D 15/16; if either is blank or a loss, enter -0-"):

- **BothGains:** min of two positive printed lines — including the ST-loss/larger-LT-gain shape where L16 < L15 and the min correctly takes L16. Correct.
- **ShortGainLongLoss** (L15 ≤ 0 < L16): min ≤ 0 → clamps to 0; the ST gain stays ordinary, dividends keep the rate via `line3a`. Correct.
- **NetLoss** (L16 < 0): min < 0 → 0; printed L15 already carries L7 = −L21. Correct.
- **Zero:** 0. Correct.
- **No Schedule D at all:** `schedule_d_lines(ar, None)` returns all-zero lines (verified `packet.rs:458–463` — the income block always receives the struct), so the operand degenerates to 0. The worksheet's "not filing Schedule D → line 3 = 1040 line 7" branch cannot diverge here: `must_file()` covers every input of L7/L15/L16, so a nonzero printed L7 ⟹ Schedule D files.
- **The case I most expected to be wrong — qualified-dividends-only (3a > 0, no D):** traced concretely through `qdcgt_line16` (`method.rs:74–91`): `pref_full = qual_div + 0 = 3a > 0` → the 0/15/20% split still applies. **Preferential rate is NOT lost**, and the pre-fold operand (`round_dollar(ar.net_ltcg)` = 0 for this shape) was identical — no regression.

No production use of `ar.net_ltcg` survives in `printed.rs` (grep: doc comments + the KAT only); the exact engine's `qdcgt_line16` call (`return_1040.rs:1154`) correctly keeps exact operands for the *computed* liability. SPEC §3.1 is extended exactly as prescribed ("PRINTED operands THROUGHOUT — L1 = printed 1040 L15, L2 = printed 3a, L3 = min(printed Sch D L15, L16), floored at 0").

**One residue → NM7 (Minor).** The KAT (`form_1040_line16_takes_the_qdcgt_operand_off_the_printed_schedule_d`, `printed.rs:1653–1678`) pins only `income.qdcgt_net_capital_gain` — it never calls `form_1040_lines`. Reverting **line 618 alone** back to `round_dollar(ar.net_ltcg)` keeps the whole suite green (the I2 KAT has no preferential income; `form_1040_printed_lines_cross_foot` has dividends but a zero, non-divergent operand). The shipped code is correct — I verified the wiring by reading — but the pass-through leg is unpinned, and the test's *name* promises line 16 while its body checks the operand. Same class as NM3 (assembly-wiring blindspot), graded consistently: Minor, not blocking.

### NEW-I2 (negative-AGI clamp) — **FIXED**

`printed.rs:1101`: `line3 = round_dollar(MEDICAL_FLOOR_RATE * line2).max(Usd::ZERO)`. Verified the full body of `schedule_a_lines`: `line11_1040` feeds **only** L2 → L3 (clamped) → L4 (already floored) — no other line consumes the negative L2 (SALT/interest/charitable all come from pre-limited parts). The printed Schedule A can no longer deduct more medical than was paid (`line4 ≤ line1` whenever `line3 ≥ 0`). The ordinary path is untouched — `max(0)` is a no-op on a positive product. The new KAT (`schedule_a_medical_floor_never_goes_negative_on_a_negative_agi`, `printed.rs:1613–1644`) feeds the **production operand shape** — a literal `dec!(-50000)` printed L11, exactly as `packet.rs:464` passes it — and pins L2 = −50,000, L3 = 0, L4 = 10,000 = the expense paid. The old fixture-clamped test (`printed.rs:2173–2196`) still exists and still feeds the fixture's clamped 0; it is now redundant-but-harmless (its assertions are correct for its operand), a cosmetic residue only.

### NEW-I3 (vacuous disjointness KAT) — **FIXED, verified by my own fault injection**

The rewritten KAT (`the_two_pipelines_cannot_overwrite_each_others_files`, `export_irs_pdf.rs:554–617`) snapshots every packet file's **bytes** (full bytes, stronger than the comment's "hashes"), self-guards against the r1 vacuity mode (asserts >1 file and a `schedule_d`-named member, so a crypto-less fixture would fail loudly), runs the 2017 slice into the same directory, and asserts byte-equality. I did **not** take the implementer's fault-injection claim on faith: in a scratchpad clone I reverted the sequence-prefix (`admin.rs:501–505` → bare `{name}.pdf`) and ran the test — it **FAILED** with `★ the slice OVERWROTE the packet's f8949.pdf`; restored, it **PASSED**. The slice writes `f8949.pdf`/`schedule_d.pdf` unconditionally even for an event-less 2017 (`wants(&[], _) = true`), so the collision pressure is real, not fixture luck. `LIMITATIONS.md:69–72` is re-trued to exactly what the test proves ("asserts every packet file is still byte-for-byte what the packet wrote").

**One Nit:** the KAT's comment and the commit message both say the packet contains "Schedule D, 8949, **Schedule SE**" — the fixture (Acquire+Dispose, no SE income) produces no Schedule SE; the actual collision census is 8949 + Schedule D. The test's own assertions don't depend on the overstatement.

### NM4 (Minors/Nits filed) — **FIXED**

`design/full-return/FOLLOWUPS.md:305–355`: all of r1 M1–M6, M8, N1–N5, plus r2 NM1–NM3, NM5, NM6 and the partial-packet observation are filed with owning phases (P6.7 cleanup, N1 → P7, M6 → post-v1). M7/NM4 are discharged by the filing act itself.

## Regression sweep of the fold

- `Form1040Income` grew a field: it is constructed **only** by `form_1040_income_lines` (grep-verified — no literal constructions to miss it); consumers destructure, so a stale copy is compile-impossible.
- The Schedule A chain: the clamp is the only behavioral delta; positive-AGI KATs (`printed.rs:1405/1427/2040`) unaffected and green.
- The CLI test fixture change (2025 → 2017 slice year in the KAT): the 2017 slice path was already covered by `ty2017_real_ledger_fills_box_c_f_and_line13_no_da`; no coverage lost.
- Exact-cents chain: untouched, FROZEN 0 bytes.

## Whole phase, final pass

The r3 delta over the phase is exactly this fold; r1 and r2 each swept `b40bdec..HEAD` in full and every Important from those sweeps is now verified fixed. I re-checked the two spots the fold's neighborhood touches for wrong-number-on-a-filed-return risk (QDCGT routing exhaustively above; the Sch A → 1040 L12 chain above) and found nothing new. No fail-opens: the dispatch, the screens-before-bytes ordering, the all-or-nothing fill, and the identity fail-closed paths are as verified in r1/r2.

## NEW findings

- **NM7 (Minor)** — the QDCGT KAT does not pin the pass-through at `printed.rs:618`; a partial revert (third operand back to `round_dollar(ar.net_ltcg)`) keeps the suite green, and the test's name promises line 16 while its body checks the operand. Extend it to call `form_1040_lines` with a bin-straddling fixture and assert `l.line16` equals the worksheet on printed operands and differs on exact ones. → P6.7, alongside NM3.
- **NM8 (Minor)** — FOLLOWUPS' pre-existing "**Open — owned by P6**" section (`FOLLOWUPS.md:357–396`) was never reconciled at the closing gate: `p5-c1` (refusal replaced by the dispatch, P6.5), `p6-printed-line-chain` (every chain exists), `p5-report-vs-pdf` (report prints printed figures + LIMITATIONS says so), and `p5-n5` (`wrap_bulleted` shipped, with a test citing the slug) are all **done but still listed open**; `p5-m1` is at least largely discharged. A reconciliation grep today shows five overdue P6-owned items that are actually finished. Mark them closed (and state p5-m1's residual scope, if any). → P6.7.
- **Nit** — the disjointness KAT's comment and the fold's commit message overstate the fixture packet's colliding-form census (it has no Schedule SE); also "hashes every packet file's BYTES" describes a byte-snapshot (immaterial — stronger).

These three must be **filed in FOLLOWUPS with owning phases** like their siblings (the standing rule); none holds the gate.

---

**VERDICT: 0 Critical / 0 Important / 2 Minor / 1 Nit**

All three r2 Importants are genuinely fixed — NEW-I1's operand is correct in every routing path including the qualified-dividends-only degeneration, NEW-I2's clamp restores §213(a) with the production operand pinned, and NEW-I3's KAT now demonstrably fails on the reverted code (I reproduced the failure myself, both directions). The full validation surface is green and re-verified first-hand.

**The P6 gate is CLOSED. The phase may ship** — subject only to filing NM7/NM8/the Nit into FOLLOWUPS (owning phase P6.7), which is recording, not gating.
