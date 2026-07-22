# Defensive Filing Wizard — brainstormed design (Approach-B sub-project 2)

**Status:** brainstormed design, PRE-SPEC. Being sent for an independent Fable-architect critique
before it is formalized into a SPEC (the same design-provenance pattern sub-project-1 used).
**Lineage:** Approach-B sub-project 2 = the guided **wizard** the conservative-filing G-3 actuator
promised (`design/conservative-filing-approach-b/SPEC.md` §1 + `design/conservative-filing/DESIGN.md` §7).
Sub-project 1 (the PromoteTranche basis-floor engine + official Form 8275 PDF) shipped as **v0.9.0**.
Sub-project 3 (VARIOUS multi-date rows / 8275 pagination) is a separate later spec.
**Branch:** `feat/defensive-filing-wizard` (off `main` @ v0.9.0, `35b351b`).

## 1. Purpose

A guided TUI flow that walks a filer through the complete "defensive filing" journey — **find** the
BTC they have no acquisition records for → **declare** it as a tranche → **assess** whether promoting
is worth it → **promote** to a filed >$0 basis floor → produce the **forms + Form 8275 disclosure**.
It is a *composition* of already-shipped, already-reviewed primitives: it adds **no new tax logic**,
only guidance, sequencing, and decision-support display.

## 2. Decisions made in brainstorming (user-chosen)

- **Surface = the TUI** — specifically **`btctax-tui-edit`**, the editor that already runs interactive,
  payload-confirmed, multi-step mutation *flows* (`classify_inbound_flow`, `reclassify_outflow_flow`,
  `set_fmv_flow`, each a `*_flow` struct with a `.step`, rendered via `draw_edit.rs`, writing via
  `edit/persist.rs` behind an explicit confirmation). The wizard is a richer, multi-*step* sibling.
- **Scope = the FULL journey** (not a promote-only core): Find → Declare → Assess → Promote → Forms.
- **Discovery (Find) = "signal + guardrail confirm"** — propose ONLY lots the engine itself cannot
  price ($0-basis / basis-pending / no documented `BasisSource`), and require a per-lot **typed**
  guardrail that restates the consequence ("you are asserting NO acquisition records exist for these
  N BTC; they will file at an ESTIMATED basis backed by Form 8275, a filed position you must defend").
  Never a silent checkbox; never proposes a lot that already has a real basis.
- **Window (Declare) = era presets + manual override, LIVE readout** — an era preset seeds a concrete
  window; the filer can override start/end manually; the wizard shows the resulting **window-min floor
  ($/BTC), holding-period date (= window-end), and estimated saving** live as the dates change (a wider
  window lowers the floor — the conservative direction — visibly).

## 3. The 5-step flow

Each step writes REAL append-only events behind confirmation; the flow *is* its own persistence — a
filer who stops after Declare simply has declared tranches in the vault and re-enters the wizard later.

1. **Find** — list only engine-can't-price lots; each → a tranche only after the typed `no records`
   guardrail. Reuses the state's lot list + basis-source signal.
2. **Declare** — era preset → window; manual override; live floor/holding/saving; writes
   `DeclareTranche` via the shipped `declare_tranche` verb.
3. **Assess** — show `overpayment_delta` / `clamped_promote_year_saving` per declared tranche, ranked
   ("filing $0 overpays by ~$X (year Y)"), so the filer sees which promotions are worth it (the P6 /
   G-3 lever). Read-only.
4. **Promote** — two-sided consent as a **TypedWord gate** (mirrors the CLI `PROMOTE_ACK_PHRASE`
   contract) + record-time provenance attestation (BG-D5 refuses gift/inherited/mined/earned/airdrop/
   fork); writes `PromoteTranche` via the shipped `promote_tranche` verb.
5. **Forms** — runs the shipped **gated** export (`promote_export_gate` refuse-before-bytes) →
   `form_8275.pdf` + `f8949.pdf`; refuses if any promoted leg's disclosure is incomplete.

## 4. Design principles (load-bearing)

- **Pure-core logic + thin-TUI driver.** The wizard's OWN logic — candidate discovery, era→window
  mapping, step sequencing/gating, the live floor+saving computation — lives in a pure, KAT-able
  `btctax-core` module (mutation-proven), exactly like the existing signals. `btctax-tui-edit` is only
  the renderer + key-dispatch. No tax logic in the TUI.
- **Reuse, don't reinvent.** Every mutation goes through the shipped, reviewed verbs
  (`declare_tranche`, `promote_tranche`) and every disclosure through the shipped gated export. The
  wizard cannot file a position the CLI couldn't; the two surfaces stay consistent by construction.
- **The answered-ness invariant** ([[answeredness-invariant]]). No step silently answers for the filer:
  the no-records assertion, the acquisition window, the promote consent, and the provenance are each an
  explicit filer act. A wizard that pre-fills a consequential answer is the exact hazard the codebase
  guards.

## 5. Defaults (call out if wrong)

- **Resumability = free via append-only events.** Re-entering the wizard re-reads state; already-
  declared / already-promoted tranches show as done. No separate draft/WIP state to persist.
- **Launch point** = a new action/key on the editor's main screen ("Defensive filing"), alongside the
  existing flows.

## 6. Questions for the architect review

1. Is the **full-journey** scope right as ONE sub-project, or should it phase (e.g. Declare+Promote
   first, Find discovery second)? Where is the scope/complexity risk?
2. Is **resumability-via-events** actually sound for a stateful 5-step flow, or does a partial journey
   (declared-but-not-promoted, or promoted-but-not-exported) leave a hazardous intermediate the wizard
   must handle explicitly on re-entry?
3. The **Find discovery**: what precisely does "engine can't price" surface, and can it propose a lot
   that must NOT be promoted-as-purchase (e.g. a $0 gift/inherited/income lot)? The BG-D5 provenance
   gate refuses those at promote-time — but should discovery pre-exclude them so the wizard never
   proposes an un-promotable candidate?
4. Is the **pure-core / thin-TUI** split clean for a stateful multi-step flow, given the existing
   `*_flow` structs live in `btctax-tui-edit` (not core)? Where should the wizard state machine live?
5. Any **coexistence hazard** with the shipped surfaces (the CLI promote flow, pseudo-mode, the
   full-return DRAFT gate) that the wizard's composition could violate?
