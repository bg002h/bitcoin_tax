# Crypto-shred (§G-14) + year dimension (§G-15) — Fable consult r2 (2026-07-30)

**Artifact:** `design/forms/FIELD_PROVENANCE.md` §6b and §6d; `FOLLOWUPS.md` §G-14, §G-15.
**Round 1** (`reviews/field-provenance-fable-r1.md`) was settled and explicitly out of scope.
**Reviewer:** Fable, independent, read-only, 13 tool calls.
**Status:** persisted VERBATIM before folding, per `STANDARD_WORKFLOW.md` §2.

---

## Verbatim output

Q1 SHRED: **Do not build envelope encryption — it cannot deliver what its name promises inside this vault, and a false crypto guarantee is worse than none.** Every prior vault generation (`.bak` — verified: `atomic.rs::atomic_write` copies the pre-shred ciphertext to `.bak` before rename; user backups of `vault.pgp`; `export_snapshot`'s deliberately-plaintext SQLite; filesystem snapshots) decrypts under the never-rotated cert to a full plaintext DB containing the answers as plain rows *and* any wrapped data key beside them — so per-item keys protect nothing a plain `DELETE` doesn't. The mechanism that actually meets G-14: **tombstoned deletion** — the answer field's type grows a third state (`Answered(bool)` / `Shredded{answered_on, shredded_on, prompt_hash}` / `None`), shred rewrites the row with content destroyed, `VACUUM` (or `secure_delete=ON`) before serialize so no ghost survives in free pages, then **save twice** so `.bak` also ages to a post-shred generation. "Cryptographically deletable" is then already true: every file btctax manages is whole-image ciphertext, and once no generation contains the rows, no key on earth reveals them. Granularity: per-(year, question) — free, because deletion has no keys to sprawl. The census stays accounted because a shredded answer must **never collapse to `None`**: `Shredded` is a determinate `asked()` resolution ("answered, content destroyed on date D"), and it is not a second store — round 1 forbade a *duplicate* that can drift; the tombstone is the primary record itself, redacted, with nothing left to drift against.

Q2 YEAR: Put `tax_year` **on `ReturnInputs` as the authoritative copy**: `set(conn, &ri)` keys the row from `ri.tax_year`, `get(conn, year)` refuses on mismatch, no `Default` for the field (constructor-required; the compiler drives every fixture edit), schema v3 under the existing §2.6 refuse-and-reimport policy — free while there are no users. `live` **keeps its one-argument signature** and reads `ri.tax_year` internally: P9's invariant survives exactly (one copy of each predicate, now capable of being correct), whereas `live(year, &ri)` invites every caller to pass a year that disagrees with the row's origin — a second copy in flight. Year gates are transcribed from the statute (`(2025..=2028).contains(&ri.tax_year)` for the car-loan interest window), and `HasIncomeExclusion`'s always-live workaround retires to `ri.tax_year >= 2025`. Durability: a per-entry `durability: PerYear | Durable`, **defaulting to `PerYear`** (fail toward re-asking). `PerYear` yes/no questions re-ask **blank — never display the prior**: for a one-keystroke answer the prior buys nothing but anchoring. `Durable` facts (DOB, identity dates) display the prior and require the same explicit keystroke as a fresh ask — never Enter-to-accept, never pre-filled — because there a forced retype invites typos, which for a DOB is the worse failure. Either way the confirmation is a NEW answer bearing this year's date, so provenance is never satisfied by a carry.

Q3 TENSION: **Reconcilable, and durability loses — by design.** Shred is gated on the year's forward-flowing outputs being extracted first: return emitted and archived, carryover written forward (`capital_loss_carryforward_in` lives in the *next* year's `ReturnInputs`), lot ledger untouched. After that gate, prior-year answers serve only ergonomics — `Durable` confirmations degrade to asking blank, which §6d already names as correct behavior with a stated usability cost. The answer's computed *consequences* survive as ordinary data; only the testimony's content dies, which is exactly the lifetime control the owner asked for.

- **The envelope claim was verified, not assumed:** `crypto.rs` has one cert, no rotation; `vault.rs::save()` re-encrypts the whole image; `atomic_write` retains the prior ciphertext generation; an old image + passphrase yields answers as plaintext rows regardless of any inner wrapping. Deletion plus generation-aging gives the identical real guarantee with zero key management.
- **The tombstone is where round 1's named bend lands, cheaply:** it retains prompt-hash and dates — evidence of what was *asked* — while destroying what was *answered*, so diligence-of-asking survives a shred and the "no second store" rule bends exactly that far and no farther.
- **Scope (b), plainly: an emitted return is NOT reproducible after shred** — answers feed computation (`HasIncomeExclusion` drives MAGI), and a tombstone cannot. Offer shred only for a year with an archived PDF and propagated carryover, and the confirmation must say "you will never regenerate this return."
- **Scope (c): never-auto-shred is right.** A retention timer is software destroying the filer's records on its own initiative — the same "software acts for the filer" defect class as a manufactured `0`. The shred UX must also state the honest bound: backups, exported plaintext snapshots, and filesystem snapshots are beyond its reach.
- **Scope (e), doing nothing:** every future TY2025+ question ships always-live with a bespoke neutrality proof; Schedule 1-A Part IV has none — a "no" swears to a predicate with no TY2024 legal existence, the census then cites fabricated testimony as provenance, and the interview bloats O(years × questions), compounding G-13a's round-trip problem annually.

BUILD ORDER: **G-15 first** — it is a small compiler-driven change (year on `ReturnInputs`, schema v3, per-entry durability enum), every TY2025 census `asked()` entry depends on year-correct liveness, and it must land before a third always-live workaround ships. **G-13 second**, per round 1's settled sequencing (pure accounting, unblocked, now year-correct), with the `Shredded` state included in the census's resolution vocabulary from day one so shred never has to retrofit it. **G-11 third**, consuming the census's mapped-field audit as round 1 already ordered. **G-14 last**: it is deletion + tombstone + VACUUM + double-save — no new cryptography — and its gate ("emitted, archived, carryover propagated") needs the emit path and census stable before it can be stated as a check rather than a warning. Durability-confirmation UX rides after G-15 at leisure; it is ergonomics, never a gate.

WHAT WOULD MAKE THIS WRONG: The shred verdict rests on the vault remaining a single whole-image ciphertext under one non-rotating cert with no copies btctax cannot delete — the moment answers are stored or synced as an independently-encrypted, separately-backed-up piece (cloud copy, split store, per-item blobs), envelope encryption with its own destroyable key stops being redundant and becomes the mechanism.

---

## Verification (orchestrator, before folding)

The load-bearing claims were re-run. **The decisive one holds on a verified fact**, and one minor
slip was found.

| claim | verdict | evidence |
|---|---|---|
| ★ `.bak` retains the PRIOR ciphertext generation | ✅ **verbatim** | `atomic.rs:18-22` — `let bak = paths::bak_of(target); fs::copy(target, &bak)?;` then `fs::rename(&tmp, target)` |
| `export_snapshot` writes plaintext SQLite | ✅ | `vault.rs:263` writes `out_dir.join("snapshot.sqlite")` from `snapshot()`, which returns the **unencrypted** in-memory image |
| `capital_loss_carryforward_in` flows to the next year | ✅ | `return_inputs.rs:438` — an INPUT to year N+1 carrying year N's result |
| one cert, no rotation; `save()` re-encrypts the whole image | ✅ | `vault.rs` holds a single `cert: openpgp::Cert`; no rotation path exists |
| "schema **v3**" | ⚠️ **SLIP** | `lib.rs:2` — `SCHEMA_VERSION: u32 = 1`. The next version is **v2**, not v3. `blob.rs::migrate` refuses on mismatch, so the *refuse-and-reimport* half of the claim is right. |
| `VACUUM` / `secure_delete=ON` | ⚠️ **not present** | zero occurrences in the tree — correctly offered as a recommendation, but it is new work, not an existing capability |

★★ **The decisive argument is now a verified fact, not a plausibility.** Because `atomic_write`
copies the pre-write ciphertext to `.bak`, a *wrapped per-item key* would sit in the **same plaintext
DB as the answers it purports to protect**, one generation back — decryptable by the same
never-rotated cert. Envelope encryption would therefore add key management and deliver **no** real
guarantee that `DELETE` + generation-aging does not already deliver. That is why the "don't build it"
verdict is accepted rather than merely noted.

★ The `VACUUM`/`secure_delete` gap matters for implementation: without it, deleted rows survive in
SQLite free pages and would be carried into the next encrypted image, so the shred would be a lie at
the byte level while looking correct at the row level. It is the single most mutation-testable part
of §G-14 — plant a shredded answer, serialize, and grep the image for the destroyed content.
