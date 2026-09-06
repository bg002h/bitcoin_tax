# VERIFICATION ledger — spec-4868-1040v r1 review (2026-09-06-spec-4868-1040v-review.md)

Controller: Claude Fable 5.1, 2026-09-06, tree at `49a94058`.

| finding | claim | check | verdict |
|---|---|---|---|
| C-1 | `Payments.extension_payment: Usd` exists, is collected, feeds `total_payments` and prints on Schedule 3 line 10 | `return_inputs.rs:708` `pub extension_payment: Usd, // → Sch 3 L10`; `return_1040.rs:2338` `+ ri.payments.extension_payment`; `printed.rs:1543` `let line10 = round_dollar(ar.printed_inputs.extension_payment)` | **TRUE** — R3's premise was false |
| C-2 | every emitter gates on `pseudo_active()` with `require_attestation` + watermark; `cli.rs:199-200` states it as a guarantee | `admin.rs:625-627` `let watermarked = state.pseudo_active(); … require_attestation(attest)?`; `cli.rs:199-200` verbatim | **TRUE** |
| I-1 | the 4868 conditions lines 2/3 on a JOINT return; the 1040 filler writes the spouse unconditionally | `f4868--2025.txt:187-190` "If you plan to file a joint return, enter on line 2 … the other SSN to be shown on the joint return"; `form1040_full.rs:442` `if let Some(sp) = &header.spouse` | **TRUE** |
| I-2 | `Form1040Lines` has no Schedule 3 line 10; line 15 = line 10 + line 11 | `printed.rs:613-620` `line31`, `line33` only; `:1545` `let line15 = line10 + line11` | **TRUE** |
| I-3 | three gates red: sequence-less ⇔ f1040; `instr_pages` pinned to one row; instructions stem must be in the manifest | `map_rows.rs:298-302` assert; `:337` `assert_eq!(pages, vec![(2025, "f1040s1a", [101, 110])])`; `cite_check.rs:1378` `every_archived_rows_instructions_stem_is_a_manifest_entry` | **TRUE** |
| I-4 | `label_join("f1040v--2025")` errs; 2025 `max_unwitnessed: 0` | seen earlier this session ("no numbered label column found"); `YEAR_FLOORS` | **TRUE** |
| I-5 | `YearRecord.return_due` exists; 2017 is 2018-04-17; the form says June 15 for out-of-country | `year_record.rs:73`; `forms/2017/YEAR.toml:8`; `f4868--2025.txt:113` "file your return or this form by June 15, 2026" | **TRUE** |
| I-6 | the payment is collected on the input form, so the refusal over-fires | C-1's evidence (collected on the TUI and by `income import`) | **TRUE** |
| I-7 | the slice-year refusal precedent and its sentence | `admin.rs:610` "Silence is the one answer a tax tool may not give here." | **TRUE** |
| I-8 | only TY2024 params are bundled | `tax_tables.rs:102` `by_year.insert(2024, ty2024_full_return())` only | **TRUE** |
| M-1 | four quotes not byte-exact; `cite-check` covers two documents | lens 1 details; `cite-check: OK — 51 quotations` over the Schedule 1-A spec + plan | **TRUE** |
| M-3 | SSN rendering is per-cell from `/MaxLen` | `map.rs:21-23` | **TRUE** |
| M-9 | `f4868--2025`: 7 labels, 17 boxes | `xtask label-census f4868--2025` | **TRUE** |
| M-10 | no top-level `tax` group | `cli.rs` command list (TaxProfile, ExportIrsPdf, … — no `Tax`) | **TRUE** |
| M-12 | `authority` may only read `not-yet-archived`, pinned to six rows | `map_rows.rs:270-280` | **TRUE** |
| M-13 | the 2024 f1040 map censuses the three foreign boxes `unmodeled`; the 2025 map has no census | `forms/2024/f1040.map.toml:202-204`; `grep -c census forms/2025/f1040.map.toml` → 0 | **TRUE** |
| N-3 | f4868 PDF is 529,415 bytes | its provenance note | **TRUE** |

**Verdict: 17/17 claims TRUE, 0 refuted.** The box-mapping audit (32/32 correct) is the reviewer's
own geometry check and is not re-run.
