# Phase 2 — J9 (select-lots) review r1 — NOT GREEN (2 Important)

_Fable, independent. Commit a779f67._

VERDICT: NOT GREEN — 0 Critical / 2 Important (I1 prose falsehood, I2 premise-vs-frame contradiction).

Verified green first: determinism (all 3 emits byte-stable, other 8 untouched, 4 gates pass); tax facts (lot-a $25k/BTC; 0.50→$12,500 basis; $47,500 proceeds → $35,000 gain, LONG 2023→2025; "contemporaneous" = made-date ≤ sale); coherence (editor S→Enter form vs seed_j9_selected select_lots(sale,[lot-a#0:50000000]) same action; parse_lot_pick confirms 0.50 of lot-a). No tax-min overclaim.

I1 — manifest.txt:4 "sells 0.50 — less than either holding" FALSE (0.50 > lot-b's 0.40). The CLI J9 got it right ("less than her holdings"). Fix "less than her combined holdings" + echoes (testonly.rs:361, testonly.rs:91, examples.rs:1248).
I2 — j9/01-select-lots.txt:9-11 shows a ONE-row form (lot-a only, "Remaining 50000000", Pick Sat 0). The select-lots LotsForm is fed from post-default-projection snap.state.lots; the default HIFO consumed lot-b + 0.10 of lot-a and dropped exhausted lots, so exactly one submission validates (lot-a:50000000). The manifest's "which lots the sale draws from is a real choice she can make and record" / "she names which lots cover it" is contradicted by its own frame — no alternative is representable in the depicted TUI (the CLI can record lot-b picks; the editor form cannot), and "Remaining 50000000" is post-default, not the at-sale availability (60000000). Fix: (a) reshape the fixture so the default leaves both lots offerable; or (b) keep the fixture, capture pick-typed + confirm frames, add honest prose about the single-row display, and file an app-side follow-up.
Minor M1: "Basis/Sat" header renders the lot's remaining TOTAL USD basis (12500.00) — reads as per-sat. App-side follow-up.
Nit N1: capture assert checks only select_lots_flow.is_some() (true at List step) while the message claims the form is open. Assert matches!(flow.step, SelectLotsStep::LotsForm{..}).
