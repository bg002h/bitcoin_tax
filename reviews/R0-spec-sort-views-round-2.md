# R0 — SPEC_sort_views.md — round 2 (independent architect)

**Artifact:** `design/SPEC_sort_views.md` (round-1 folded).
**Baseline:** branch `feat/sort-views` @ `67b62de`; `main` @ `1837917`. Repo `/scratch/code/bitcoin_tax`.
**Reviewer role:** independent architect (author ≠ reviewer). Read-only; no implementation.
**Bar:** 0 Critical / 0 Important. Prior round: `reviews/R0-spec-sort-views-round-1.md` (0C/4I/4M/2N).

## Verdict: **0 Critical / 1 Important / 0 Minor / 3 Nit — CHANGES REQUIRED (not GREEN).**

The four round-1 Importants (I1–I4) and all four Minors (M-1…M-4) and both Nits (N-1/N-2) are **folded
correctly and verified against source** — see the per-fold ledger below. The rewrite introduced no new
contradiction, and the editor genuinely reuses the viewer's `render` fns (one signature change serves both).

**But** the round-1 fold answered only half of what M-3 stood for. M-3 was "own the existing tests that break."
The spec now correctly owns the **2** arrow=year tests — and stops there, adding the line *"The plan owns these
edits, not just new KATs."* That claim is materially incomplete: the specified `s`→`S` / `l`→`L` editor rebind
**breaks 23 additional currently-passing browse-level tests** (15 that press `s` to open select-lots, 8 that
press `l` to open link-transfer), none of which the plan accounts for. That is the one blocking gap (I5).

---

## Per-fold verification ledger (round-1 findings)

### I1 — safety reframe → **RESOLVED ✔**
The counterfactual "stable-key selection" claim and its KAT are **gone**. SPEC §"[R0-I1] Safety" (lines 54-61)
now states the true basis, and every citation checks out:
- `*_state: TableState` at **app.rs:139-141** (viewer) and **editor.rs:96-98** (editor) — confirmed both are
  bare `TableState` scroll fields.
- Read **only** by `active_state` (**main.rs:8771-8775**, feeding `scroll_up/down`, `page_up/down`,
  `go_top/bottom` at 8813-8875) and cleared by `reset_selections` (**main.rs:8885-8888**). A grep of
  `holdings_state|disposals_state|income_state` in `main.rs` returns only those two sites — no edit-flow reads
  it.
- Edit flows build their own picker: `open_select_lots_flow` → `TargetList::new(items)` at
  **main.rs:3980-3981** inside `SelectLotsFlowState`. Confirmed independent of tab order.
- The new KATs `sorting_does_not_mutate_events_or_state` + `edit_flow_targets_are_independent_of_display_order`
  (lines 88-90) test the mechanism that actually exists. The scroll-highlight-across-resort behavior is now
  specified explicitly (line 60-61: index-anchored, "stays at its row index (cosmetic)"). No residual
  contradiction.

### I2 — sort typed data, not `Vec<Row>` → **RESOLVED ✔** (one type-name nit, N-1 below)
SPEC §"[R0-I2] Architecture" (lines 63-72) now sorts a **typed borrowed working set** then formats. All source
collections + keyed fields exist:
- Holdings `Vec<&Lot>` — `snap.state.lots: Vec<Lot>` (state.rs:251); `Lot` carries `wallet`/`acquired_at`/
  `remaining_sat`/`usd_basis`/`basis_source`/`basis_pending` (state.rs:107-125).
- Disposals `Vec<(&Disposal, &DisposalLeg, usize)>` — `snap.state.disposals: Vec<Disposal>` (state.rs:253);
  `Disposal.disposed_at`/`.legs` (state.rs:160-168); `DisposalLeg.acquired_at`/`sat`/`proceeds`/`basis`/`gain`/
  `term`/`wallet` (state.rs:138-158). The leg-index `usize` gives the total order (see M-2).
- Income `Vec<&IncomeRecord>` — `snap.state.income_recognized: Vec<IncomeRecord>` (state.rs:255);
  `recognized_at`/`sat`/`usd_fmv`/`kind`/`business` (state.rs:211-218).

The shared-types (`Dir`/`ViewSort`/`cursor`/`stable_sort_by`) + per-view-comparator split is sound and
implementable: the comparator lives with the view that owns its row type; the stable helper applies dir +
tie-break uniformly.

### I3 — real columns → **RESOLVED ✔** (no column mismatch remaining)
Every column list now **exactly** matches the render code:
- **Holdings** spec `Wallet · Acquired · BTC · USD Basis · Source · Pending` = render cells
  (holdings.rs:51-58) / header (holdings.rs:86-93). ✔
- **Disposals** (per-leg) spec `Disposed · Acquired · BTC · Proceeds · Basis · Gain · Term · Wallet` = render
  cells (disposals.rs:58-67, loop `for leg in &disposal.legs` at :51) / header (disposals.rs:94-103). ✔
- **Income** spec `Recognized · Kind · Business · BTC · USD FMV` = render cells (income.rs:41-47) / header
  (income.rs:70-76). ✔
- **Income has NO wallet** — confirmed at the type level: `IncomeRecord` (state.rs:211-218) has no wallet
  field, and the rendered Income row derives none. The "do not invent one" instruction is correct. ✔

### I4 — docs are hand-authored → **RESOLVED ✔** (one dropped coupling note, N-2 below)
- TUI man pages are hand-authored, not `make docs`-generated: `xtask/src/docs.rs:98`
  ("Does NOT include the hand-authored TUI pages (`btctax-tui.1` / `btctax-tui-edit.1`)") and the structural-
  guard docstring at `docs.rs:361` ("the hand-authored TUI pages document their tab set + keys"). ✔
- Hand-edit lines confirmed: `btctax-tui.1:67-69` (`.B \(<- / \(->` → "Previous / next tax year (resets the
  row selection)."); `btctax-tui-edit.1:35-36` (`.B \(<- / \(->` → "Change tax year."); `btctax-tui-edit.1:63-64`
  (`.B s` → "select-lots"; the `l`→link-transfer entry follows a few lines below). ✔
- Viewer footer `draw.rs:150` ("←/→: change year") vs editor footer `draw_edit.rs:188` ("←/→: change year") +
  editor `?` overlay `draw_edit.rs:1916` ("←/→ change year"), `:1921` ("s select-lots"), `:1922`
  ("l link-transfer"). ✔
- **Viewer has NO `?` overlay** — confirmed: `draw.rs` has only the *export-confirmation* modal overlay (:157),
  no help overlay; grep for `Char('?')`/`draw_help`/`help_overlay` in the viewer crate returns nothing. ✔

### M-1 — default Disposed date → **RESOLVED ✔**
Line 39-40 names `Disposals→Disposed [R0-M-1] (the disposal date, not Acquired)`. Matches `disposed_at`
(disposals.rs:38/47; state.rs:163) and the existing per-flow date ordering. The two-date ambiguity is closed.

### M-2 — per-leg total order → **RESOLVED ✔**
Line 44-46: tie-break `(sort key, then disposed_at, EventId, leg index)` applied via a STABLE sort — a genuine
total order, because the working set carries the leg index (`Vec<(…, usize)>`). Legs sharing `(disposed_at,
EventId)` are disambiguated by leg index. Correct.

### M-3 — existing arrow=year tests → **PARTIALLY RESOLVED → escalates to I5**
The **arrow** half is correct and complete: the only two browse-level arrow tests are
`left_right_changes_selected_year` (**lib.rs:776**, asserts Left/Right change `selected_year` at 781-797) and
`left_right_on_browse_changes_selected_year` (**main.rs:9493**, asserts at 9498-9500) — both exist and assert
arrow=year, and both are named in the spec (line 91-92). I checked every other `press(KeyCode::Left|Right)`
site (viewer: only those 3 lines in the one test; editor: 19577/19585/19666/21163/21168) and confirmed the
editor ones are all **in-flow** (resolve-conflict / bulk-resolve `Choose` toggles) — separate handler context
(main.rs:5536+/7138/8271), untouched. So no *arrow* test is missed.

The gap is that the spec generalized M-3 into *"The plan owns these edits, not just new KATs"* while the
`s`/`l` rebind breaks a far larger set — see **I5**.

### M-4 — render signature + both call sites → **RESOLVED ✔**
Line 73-74 names both call sites, both verified: viewer dispatch `draw.rs:137-139` (→ `holdings::draw` /
`disposals::draw` / `income::draw` wrappers) and editor `draw_edit.rs:143-163` (→
`btctax_tui::tabs::holdings::render` / `disposals::render` / `income::render` **directly**). The editor reuses
the viewer's App-free `render` fns, so extending the `render`/`draw` signature with `ViewSort`+`cursor` and
threading `App`/`Editor` state serves **both** apps from one change. ✔

### N-1 (footer excluded) / N-2 (Holdings not year-scoped) → **RESOLVED ✔**
- Footer is a structurally separate `Table::footer(total_row)` — total_row built at holdings.rs:76 /
  disposals.rs:82 / income.rs:61, applied via `.footer(...)` at holdings.rs:112 / disposals.rs:131 /
  income.rs:98 — never a member of `rows`. Line 49-50 states it's excluded. ✔
- Holdings ignores `year` (holdings.rs:24 doc: "shows all lots regardless of year"; `_year` unused at :29), so
  `[`/`]` is a no-op there. Line 51-52 notes it. ✔

---

## Important

### I5 — the `s`→`S` / `l`→`L` editor rebind breaks 23 existing browse tests the plan does not own; the spec's "the plan owns these edits" claim is materially incomplete
- **Where:** SPEC line 91-92 ("**[R0-M-3]** UPDATE the existing arrow-steps-year tests: viewer `lib.rs:776`,
  editor `main.rs:~9494` … **The plan owns these edits, not just new KATs**") and Plan **T2** (line 107-108,
  which lists the rebind + new KATs `editor_S_opens_select_lots` / `editor_L_opens_link_transfer` /
  `editor_s_now_sorts` + `update main.rs:~9494`, but no migration of the existing `s`/`l` driver tests).
- **Evidence:** the spec rebinds top-level editor `s` (currently select-lots, main.rs:421) → `S` and `l`
  (currently link-transfer, main.rs:423) → `L`, freeing `s` for sort and `l` for the column cursor. In the
  editor test suite:
  - **15** tests press top-level `s` to drive select-lots: main.rs **14735, 14962, 15212, 15305, 15339,
    15424, 15479, 17516, 17613, 17767, 17907, 18030, 18095, 18216, 18402** — with assertions like
    `main.rs:14738` *"select_lots_flow must open on 's'"*, `:15215` *"E2E-SL: flow must open on 's'"*,
    `:18030` `.expect("WS: flow must open")`.
  - **8** tests press top-level `l` to drive link-transfer: main.rs **18328, 18449, 18521, 18632, 18706,
    18805, 18879, 19478** — e.g. `main.rs:18522` `assert!(app.link_transfer_flow.is_some(), "C2-LT: flow opens on 'l'")`.
  - Both flow handlers (`handle_select_lots_flow_key` main.rs:3197, `handle_link_transfer_flow_key`
    main.rs:4247) bind **neither** `s` nor `l` internally, so **all 23** presses are top-level browse openers,
    not in-flow keys. After the rebind, `s` sorts and `l` moves the cursor → every one of these 23 tests
    changes behavior (the `is_some()`/`expect` ones fail outright; the three `is_none()` ones — 15305, 17613,
    18216 — pass vacuously for the wrong reason and no longer test their intent).
- **Why it matters:** M-3's whole purpose was to make the plan *own* the existing tests that break, so the
  phased/TDD implementation isn't ambushed by a red wall. The spec discharged that for the 2 arrow tests and
  then asserted the plan owns "these edits" — but the *rebind*, which is the larger behavioral change, breaks
  **~12× more** tests than are named. An implementer following the plan literally would rebind, get 23 reds
  with no plan line for them, and be tempted into an ad-hoc fix (or to doubt the rebind). This is exactly the
  understated-blast-radius failure the review loop exists to catch, made worse by an explicit (now-false)
  ownership claim.
- **Fix (small):** in M-3 / T2, add ownership of the rebind-driver migration: *"migrate every existing
  top-level `s`→select-lots test (15 sites, main.rs 14735…18402) and `l`→link-transfer test (8 sites,
  main.rs 18328…19478) to press `S`/`L` respectively; verify none are in-flow presses (both flow handlers bind
  neither key)."* Cite a couple of representative sites. No design change — just complete the enumeration so the
  plan matches reality.

---

## Nit

### N-1 — Income working-set type is named `IncomeRecognized`, but the real type is `IncomeRecord`
SPEC line 67 (`Vec<&IncomeRecognized>`) and line 36 ("`IncomeRecognized` has no wallet field") use a type name
that does not exist. The struct is **`IncomeRecord`** (state.rs:211-218); the *field* on `LedgerState` is
`income_recognized` (state.rs:255). `Vec<&IncomeRecognized>` would not compile as written. Self-correcting (the
implementer hits it immediately) and the substance (no wallet) is right, but per the workflow's "verify
citations against source," rename to `IncomeRecord`. (Carried over verbatim from round-1 I3, so not a new
error — just not yet fixed.)

### N-2 — the `docs.rs:409-415` sync-test coupling that round-1 I4 asked to note was dropped
Round-1's I4 fix said to note the `manpages_have_required_sections` sync test (`xtask/src/docs.rs:410-414`),
which asserts `btctax-tui-edit.1` contains `.B ?`, `.B V`, `.B O`. The round-2 spec doesn't mention it. I
verified the planned hand-edits (`s`→`S`, `l`→`L`, year `←/→`→`[`/`]`) do **not** touch the `?`/`V`/`O`
entries, so the test does **not** break — hence a nit, not a finding. Worth a one-line note in §I4/T3 so the
implementer keeps the man page ↔ overlay in sync when adding the `S`/`L`/sort key rows.

### N-3 — "`h`/`l` appear only inside flow contexts" is imprecise (`l` is also bound top-level at 421→423)
SPEC line 23-24 says "`h`/`l` appear only inside flow contexts (main.rs:7138/8271)." True for `h` (unbound at
top level → free for cursor-left). But `l` **is** bound top-level at main.rs:423 — that is precisely the
link-transfer binding being rebound to `L`. The conclusion (conflict-free after the rebind) holds; only the
phrasing understates that `l` must be *freed* by the rebind, not that it's already free. Reword to "`h` is
unbound at top level; `l`/`s` are freed by the rebind" for accuracy.

---

## Confirmed sound (spot-checked this round)
- **Editor reuses the viewer's `render`** (draw_edit.rs:143-163 → `btctax_tui::tabs::{holdings,disposals,
  income}::render`), so **one** signature change serves both apps — the plan's shared-render assumption holds. ✔
- **Rebind targets free:** grep for `Char('S')`/`Char('L')`/`Char('[')`/`Char(']')` in the editor `main.rs`
  returns nothing; the viewer `lib.rs` binds none of `s l h S L [ ]`. ✔
- **Flow-level `Left`/`Right` untouched:** all non-browse arrow tests (main.rs 19577/19585/19666/21163/21168)
  are in-flow toggles handled at main.rs:5536+/7138/8271 — a separate match context. ✔
- **Year keys** viewer lib.rs:215/219, editor main.rs:407/411; **rebind sources** editor main.rs:421 (`s`),
  423 (`l`) — all as cited. ✔

## Re-review note
One blocking Important (I5) remains; the fix is a plan/enumeration completion, not a design change. Fold it,
persist the fold, and re-review (including after the last fold) to confirm 0C/0I before implementation. Not
R0-GREEN.
