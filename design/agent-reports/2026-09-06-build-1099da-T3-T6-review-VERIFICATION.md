# VERIFICATION ledger — the T3–T6 build review (`2026-09-06-build-1099da-T3-T6-review.md`, 0C/3I/4M/2N)

Controller's machine-check of every checkable claim, run at `2ef2c2ce` before anything was folded.
Commands are the ones run; "HOLD" means the tree says what the report says.

| # | finding | claim | check | result |
|---|---|---|---|---|
| 1 | I-1 | `route_8949_boxes` is never called in `btctax-tui` or `btctax-tui-edit` | `grep -rn route_8949_boxes crates/btctax-tui/src crates/btctax-tui-edit/src \| wc -l` → `0` | HOLD |
| 2 | I-1 | the Forms tab renders `form_8949(&snap.state, year)` unrouted | `tabs/forms.rs:96` reads exactly that | HOLD |
| 3 | I-3 | `seed_broker_rows` has one production call site, inside `open_tax_inputs_form` | grep: `main.rs:928` is the only non-test call | HOLD |
| 4 | I-3 | the `Loaded::Fresh` arm sets `working: None`, so the seed is skipped | `main.rs:846-850` (the Fresh literal ends with `working: None` above) | HOLD |
| 5 | I-2 | spec R1 says the surfaces MUST enumerate each key's rows | `SPEC_1099da_broker_reporting.md:143` — "…the tool MUST show it: `report` and the TUI prompt enumerate each key's rows…" | HOLD |
| 6 | I-2 | `report`'s disposal listing prints no wallet | `render.rs:455-480` contains no `wallet` | HOLD |
| 7 | M-2 | `stored_unread` is computed only as "a stored key not in the census" | `render.rs:1887` | HOLD |
| 8 | M-1 | the live advisory says "filed under the box your answer chose (G/H … J/K …)" regardless of the answer | `admin.rs:501-505` | HOLD |
| 9 | N-1 | a `{e:?}` Debug render reaches a user message | `render.rs:1222` | HOLD |
| 10 | M-3 | no test writes distinct non-zero values into 1b/2/8b/9 and reads the cells back | the four test files naming `line1b`/`Row1b` are existence/field-set checks (kats.rs helper, sp4, f6251_fill, full_return_forms) — none asserts a value per cell | HOLD (by inspection) |

Not machine-checkable here: M-4's claim that the anchors "cannot fire on any live path" (a reachability
argument over the commit gate — accepted as the reviewer's reading, to be settled by the fold's kill),
N-2's judgement about which revision's text to quote.

Disposition: I-1, I-2, I-3 block; folded next with kills, by one opus implementer under a brief. M-1,
M-2, M-3, N-1, N-2 folded in the same pass where cheap; M-4 recorded as forward-looking unless the
row-targeting change is small. Nothing folded yet at this commit.
