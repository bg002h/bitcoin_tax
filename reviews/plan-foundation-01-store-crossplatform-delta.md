# Review — Cross-platform (NFR8) + crypto-rust delta (store spec/plan)

- **Reviewer:** independent Rust reviewer, scoped to the post-green NFR8 + crypto-rust delta. Verified vs sequoia 1.x flags, fs2 0.4 source, windows-sys 0.59, Rust Windows error-kind mapping, POSIX/Win32 rename.
- **Date:** 2026-06-28
- **Verdict (as reviewed):** 1 Critical, 0 Important, 1 Minor, 3 Nit. **Resolution:** the Critical was a plan-doc/code mismatch — the *implemented* Cargo.toml (commit d4019bd) already carries `allow-experimental-crypto` and builds+passes; the plan document omitted it. Plan synced. Net after fold: 0 Critical / 0 Important.
- Persisted per STANDARD_WORKFLOW §2.

## Critical (resolved by doc-sync)
**C-1 — plan Cargo.toml missing `allow-experimental-crypto`.** The Task-0 spike (FOLLOWUPS, 2026-06-28) confirmed sequoia's build script gates the RustCrypto backend behind `allow-experimental-crypto` and won't compile without it. The implemented crate has it (builds + smoke 2/2); the **plan document** lacked it → "as-written" wouldn't compile. **Folded:** added `allow-experimental-crypto` to plan Task 0 Cargo.toml + comments. (Code was already correct.)

## Important — None.

## Minor
**M-1 — fs2 0.4 dormant + WouldBlock mapping is Rust-version-dependent.** On Windows, `fs2` propagates `ERROR_LOCK_VIOLATION(33)`; Rust ≥1.64 maps it to `WouldBlock` (MSRV 1.74 satisfies). **Folded:** comment in plan lock.rs citing the dependency + raw_os_error(33) fallback; FOLLOWUPS notes `fd-lock` (maintained) as the swap candidate; Windows CI ticket already open in FOLLOWUPS.

## Nit (folded)
- **N-1** VirtualLock `*const→*mut` cast comment added (LPVOID, not written-through; BOOL!=0 = success).
- **N-2** S2K spike panic message reworded (Argon2 isn't "weak").
- **N-3** Cargo comment precision: `allow-variable-time-crypto` is for RSA *interoperability* (rsa crate always compiled under crypto-rust), not the primary Cv25519 path.

## Cross-platform soundness (confirmed)
`fs2::try_lock_exclusive` → flock(EX|NB)/LockFileEx(EXCLUSIVE|FAIL_IMMEDIATELY) on Unix/Windows ✓; WouldBlock-on-contention ✓ (M-1 caveat); `VirtualLock`/`VirtualUnlock` correct (feature `Win32_System_Memory`; BOOL check; drop-after-zeroize order) ✓; `std::fs::rename` POSIX-atomic / Windows MoveFileEx-replace + `.bak` safety net — honest ✓; macOS = Unix (flock/mlock/rename apply), no gaps ✓.

## crypto-rust (confirmed)
`allow-variable-time-crypto` correct for RSA interop ✓; `allow-experimental-crypto` required (C-1, folded); variable-time trade-off honest + acceptable for local at-rest single-user ✓; `S2K::Iterated{SHA256, 0x3E00000}` backend-independent, unchanged under crypto-rust ✓. No regression vs green spec/plan.
