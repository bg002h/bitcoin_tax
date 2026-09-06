# VERIFICATION ledger — spec-4868-1040v r5 verification (2026-09-06-spec-4868-1040v-review-r5.md)

Controller: Claude Fable 5.1, 2026-09-06.

| item | claim | check | verdict |
|---|---|---|---|
| M | `name_line` is a Rust field (`ReturnHeader.name_line`), not a key in any committed map | `grep -rn name_line crates/btctax-forms/forms/` → none; `packet.rs:338` `pub name_line: String` | **TRUE** |
| N1 | four of the six declared grids have a geometry fixture (the two TY2017 grids have none) | `design/forms/geometry/`: f8949--2024, f8949--2025, f8283--2025 present; no 2017 fixtures | **TRUE** (three fixtures: 2024/f8949, 2025/f8949, 2025/f8283 — and 2024/f8275 has none either; stated as "the declared grids that HAVE a fixture") |
| N2 | a dangling "so" | by reading | **TRUE** |
| gate | R4-I1/I2, M1/M2, N1–N4 RESOLVED; `c1_1` confirmed by dump-fields | the reviewer's run | **accepted** |

**Verdict: 0C/0I — the spec is GREEN at r6; Minors/Nits fixed inline, no re-dispatch.**
