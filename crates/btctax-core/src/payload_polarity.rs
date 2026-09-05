//! FR-38: the **record-time polarity gate** for every value carried by a persisted event payload.
//!
//! # Why this file exists, and why it is a file
//!
//! FR-38 was filed as a `classify-raw` finding — a `usd_cost` arriving inside a JSON payload was
//! neither sign-checked nor advised. The 2026-09-05 recon
//! (`design/agent-reports/2026-09-05-recon-fr38-validation-seam.md`) established the gap is wider
//! than the CLI: the TUI `classify-raw` form (`form.rs`) built its payload with a bare
//! `Usd::from_str` while the guarded `parse_nonneg_usd` sat in the same file; three of the four CSV
//! adapters never sign-guard USD at all (and `btctax_adapters::parse::parse_usd` *deliberately*
//! produces negatives from accounting parentheses); and `accept-conflict` has no validation code of
//! its own, because what it promotes into force was written by an earlier command.
//!
//! Adding a seventh, eighth and ninth call site is the pattern that produced FR-38. So the check
//! lives at the **persistence boundary** instead: `persistence::insert` holds the workspace's only
//! `INSERT INTO events`, and every door — present or future — necessarily passes through it. A
//! payload that has not been checked cannot become a row. This is the same seam ruling the repo
//! already made for the Form 8283 TIN/EIN shapes at the shared side-table write choke point
//! (`donation_details::set` → `validate_and_normalize`), including its as-built correction that a
//! CLI-only fix was bypassed by the TUI.
//!
//! # ★ THE `..` RULE — do not add one to this file
//!
//! Every struct pattern below names **every** field, with **no `..` rest-pattern**. That is load
//! bearing: it makes adding a `Usd` (or `Sat`) field to any payload a **compile error**, not a
//! silent omission —
//!
//! ```text
//! error[E0027]: pattern does not mention field `new_usd_field`
//! ```
//!
//! — so a new money field cannot ship until someone has classified its polarity here. Likewise the
//! outer `match` has **no `_` arm**, so a new `EventPayload` variant is an `E0004`. rustc's own
//! help text offers `..` as the escape hatch, which is why this is written down: the convention is
//! one file wide and greppable, rather than spread over the 40 production append sites.
//!
//! A field that carries no polarity is written `name: _` — mentioned, and thereby decided.
//!
//! # ★ REFUSE, NEVER REPAIR
//!
//! This module only ever **reads** and returns `Result`. It must never `.abs()`, clamp, or round.
//! `persistence::fingerprint` is computed from these same values and is a component of a conflict
//! `EventId`, so normalising here would change event identities — a migration, not a fix. Refusing
//! changes no byte on disk, no column, no digest.
//!
//! # ★ THE POLARITY IS PER FIELD, NEVER PER TYPE — `[G-I5]`
//!
//! `Usd` is a type alias for `Decimal`, and a blanket "every `Usd` >= 0" is **false**:
//! `ConsentTerm`'s four delta fields are *differences* (`conservative_promote.rs`:
//! `let delta_usd = t_without - t_with;`), so a promotion that raises tax legitimately records a
//! negative. Those four are routed through [`signed_by_design`] — an explicit whitelist with the
//! reason attached — exactly as `eventref::parse_nonneg_usd_arg` guards per flag and never inside
//! the shared parser.
//!
//! # What is deliberately NOT here
//!
//! * **The magnitude ("sats typed into a dollars field") advisory.** It needs the satoshi
//!   quantity, the *target event's* tax date, and a price provider. `append_decision` has none of
//!   the three, and a decision's `utc_timestamp` is the decision's creation time — not the date the
//!   FR-37 heuristic anchors against. It stays an advisory at the surfaces (`reconcile.rs`).
//! * **A read-side guard.** `load_all` is untouched. The invariant is forward-only, so a vault
//!   that already holds an impossible value still loads (and can be inspected and voided). A serde
//!   `deserialize_with` would instead make such a vault *unloadable*.
use crate::conventions::{Sat, Usd};
use crate::event::*;
use crate::CoreError;

/// The statutory reason a money field on an event payload cannot be negative. §1012 basis; §1016
/// floors adjustments at zero; §301(c)(2)-(3)/§733 excess-of-basis is *gain*, never negative basis.
/// Fees and proceeds are magnitudes. Zero stays legal — it is this application's conservative
/// default for an undocumented tranche and for an inbound self-transfer.
const USD_REASON: &str =
    "no legitimate negative cost basis / FMV / fee / proceeds exists (§1012; §1016 floors \
     adjustments at zero)";

/// The reason a satoshi quantity cannot be negative: it is a count of indivisible units held or
/// moved. Zero stays legal (a fee-only or dust row is not an error).
const SAT_REASON: &str = "a satoshi quantity is a count and cannot be negative";

fn usd(field: &str, v: &Usd) -> Result<(), CoreError> {
    if v.is_sign_negative() && !v.is_zero() {
        return Err(CoreError::ImpossibleValue {
            field: field.to_string(),
            value: v.to_string(),
            reason: USD_REASON,
        });
    }
    Ok(())
}

fn usd_opt(field: &str, v: &Option<Usd>) -> Result<(), CoreError> {
    match v {
        Some(x) => usd(field, x),
        None => Ok(()),
    }
}

fn sat(field: &str, v: Sat) -> Result<(), CoreError> {
    if v < 0 {
        return Err(CoreError::ImpossibleValue {
            field: field.to_string(),
            value: v.to_string(),
            reason: SAT_REASON,
        });
    }
    Ok(())
}

fn sat_opt(field: &str, v: Option<Sat>) -> Result<(), CoreError> {
    match v {
        Some(x) => sat(field, x),
        None => Ok(()),
    }
}

/// `[G-I5]` — the explicit signed-field whitelist. A `ConsentTerm` delta is `t_without - t_with`
/// (`conservative_promote.rs`); a promotion that RAISES tax yields a negative, and that negative is
/// the truthful figure shown to the filer and snapshotted into the §6664(c) good-faith record. It
/// is a no-op **on purpose**: writing the call, rather than omitting the check, is what makes the
/// whitelist visible to a reader and to `grep`.
#[inline]
fn signed_by_design(_field: &str, _v: &Usd) {}

/// `[G-I5]`, the `Option` form.
#[inline]
fn signed_by_design_opt(_field: &str, _v: &Option<Usd>) {}

/// Refuse any structurally-impossible value on `p` — a negative USD amount on a field that has no
/// negative meaning, or a negative satoshi count. Total, pure, price-free and date-free.
///
/// Called from `persistence::insert` before the `INSERT`, so it holds for every door at once. See
/// the module docs for the `..`-free / `_`-arm-free rule that makes a new money field a compile
/// error, and for why normalisation is forbidden here.
pub fn check_payload_polarity(p: &EventPayload) -> Result<(), CoreError> {
    match p {
        // ── imported payloads ────────────────────────────────────────────────────────────────
        EventPayload::Acquire(Acquire {
            sat: s,
            usd_cost,
            fee_usd,
            basis_source: _,
        }) => {
            sat("Acquire.sat", *s)?;
            usd("Acquire.usd_cost", usd_cost)?;
            usd("Acquire.fee_usd", fee_usd)
        }
        EventPayload::Income(Income {
            sat: s,
            usd_fmv,
            fmv_status: _,
            kind: _,
            business: _,
        }) => {
            sat("Income.sat", *s)?;
            usd_opt("Income.usd_fmv", usd_fmv)
        }
        EventPayload::Dispose(Dispose {
            sat: s,
            usd_proceeds,
            fee_usd,
            kind: _,
        }) => {
            sat("Dispose.sat", *s)?;
            usd("Dispose.usd_proceeds", usd_proceeds)?;
            usd("Dispose.fee_usd", fee_usd)
        }
        EventPayload::TransferOut(TransferOut {
            sat: s,
            fee_sat,
            dest_addr: _,
            txid: _,
        }) => {
            sat("TransferOut.sat", *s)?;
            sat_opt("TransferOut.fee_sat", *fee_sat)
        }
        EventPayload::TransferIn(TransferIn {
            sat: s,
            src_addr: _,
            txid: _,
        }) => sat("TransferIn.sat", *s),
        EventPayload::Unclassified(Unclassified { raw: _ }) => Ok(()),

        // ── system payload: the check must reach the payload accept-conflict later promotes ──
        EventPayload::ImportConflict(ImportConflict {
            target: _,
            new_payload,
            new_fingerprint: _,
        }) => check_payload_polarity(new_payload),

        // ── decision payloads ────────────────────────────────────────────────────────────────
        EventPayload::TransferLink(TransferLink {
            out_event: _,
            in_event_or_wallet: _,
        }) => Ok(()),
        EventPayload::ReclassifyOutflow(ReclassifyOutflow {
            transfer_out_event: _,
            as_,
            principal_proceeds_or_fmv,
            fee_usd,
            donee: _,
        }) => {
            match as_ {
                OutflowClass::Dispose { kind: _ } => {}
                OutflowClass::GiftOut => {}
                OutflowClass::Donate {
                    appraisal_required: _,
                } => {}
            }
            usd(
                "ReclassifyOutflow.principal_proceeds_or_fmv",
                principal_proceeds_or_fmv,
            )?;
            usd_opt("ReclassifyOutflow.fee_usd", fee_usd)
        }
        EventPayload::ClassifyInbound(ClassifyInbound {
            transfer_in_event: _,
            as_,
        }) => match as_ {
            InboundClass::Income {
                kind: _,
                fmv,
                business: _,
            } => usd_opt("InboundClass::Income.fmv", fmv),
            InboundClass::GiftReceived {
                donor_basis,
                donor_acquired_at: _,
                fmv_at_gift,
            } => {
                usd_opt("InboundClass::GiftReceived.donor_basis", donor_basis)?;
                usd("InboundClass::GiftReceived.fmv_at_gift", fmv_at_gift)
            }
            InboundClass::SelfTransferMine {
                basis,
                acquired_at: _,
            } => usd_opt("InboundClass::SelfTransferMine.basis", basis),
        },
        EventPayload::ManualFmv(ManualFmv { event: _, usd_fmv }) => {
            usd("ManualFmv.usd_fmv", usd_fmv)
        }
        EventPayload::SafeHarborAllocation(SafeHarborAllocation {
            lots,
            as_of_date: _,
            method: _,
            timely_allocation_attested: _,
            pre2025_method: _,
        }) => {
            for AllocLot {
                wallet: _,
                sat: s,
                usd_basis,
                acquired_at: _,
                dual_loss_basis,
                donor_acquired_at: _,
            } in lots
            {
                sat("AllocLot.sat", *s)?;
                usd("AllocLot.usd_basis", usd_basis)?;
                usd_opt("AllocLot.dual_loss_basis", dual_loss_basis)?;
            }
            Ok(())
        }
        EventPayload::SupersedeImport(SupersedeImport { conflict_event: _ }) => Ok(()),
        EventPayload::RejectImport(RejectImport { conflict_event: _ }) => Ok(()),
        EventPayload::VoidDecisionEvent(VoidDecisionEvent { target_event_id: _ }) => Ok(()),

        // the FR-38 door as filed: the supplied payload rides one box down
        EventPayload::ClassifyRaw(ClassifyRaw { target: _, as_ }) => check_payload_polarity(as_),

        EventPayload::MethodElection(MethodElection {
            effective_from: _,
            method: _,
            wallet: _,
        }) => Ok(()),
        EventPayload::LotSelection(LotSelection {
            disposal_event: _,
            lots,
            attested: _,
        }) => {
            for LotPick { lot: _, sat: s } in lots {
                sat("LotPick.sat", *s)?;
            }
            Ok(())
        }
        EventPayload::ReclassifyIncome(ReclassifyIncome {
            income_event: _,
            business: _,
            kind: _,
        }) => Ok(()),
        EventPayload::SelfTransferPassthrough(SelfTransferPassthrough {
            in_event: _,
            out_event: _,
        }) => Ok(()),
        EventPayload::DeclareTranche(DeclareTranche {
            sat: s,
            wallet: _,
            window_start: _,
            window_end: _,
        }) => sat("DeclareTranche.sat", *s),
        EventPayload::PromoteTranche(PromoteTranche {
            target: _,
            method: _,
            filed_basis,
            coverage: _,
            provenance_attested: _,
            acknowledgment:
                Acknowledgment {
                    phrase: _,
                    shown_terms,
                    provenance_text: _,
                    provenance_version: _,
                },
            part_ii_narrative: _,
        }) => {
            usd("PromoteTranche.filed_basis", filed_basis)?;
            for term in shown_terms {
                match term {
                    // [G-I5]: a computed tax delta is `t_without - t_with` — SIGNED by design.
                    ConsentTerm::ComputedTax {
                        year: _,
                        delta_usd,
                        deduction_delta_usd,
                    } => {
                        signed_by_design("ConsentTerm::ComputedTax.delta_usd", delta_usd);
                        signed_by_design_opt(
                            "ConsentTerm::ComputedTax.deduction_delta_usd",
                            deduction_delta_usd,
                        );
                    }
                    // [G-I5]: fold-pair deltas — SIGNED by design.
                    ConsentTerm::Uncomputable {
                        year: _,
                        gain_delta_usd,
                        deduction_delta_usd,
                    } => {
                        signed_by_design(
                            "ConsentTerm::Uncomputable.gain_delta_usd",
                            gain_delta_usd,
                        );
                        signed_by_design(
                            "ConsentTerm::Uncomputable.deduction_delta_usd",
                            deduction_delta_usd,
                        );
                    }
                    // NOT deltas: an undisposed quantity and a reduction magnitude.
                    ConsentTerm::Unrealized {
                        sat: s,
                        hypothetical_reduction,
                        as_of: _,
                    } => {
                        sat("ConsentTerm::Unrealized.sat", *s)?;
                        usd_opt(
                            "ConsentTerm::Unrealized.hypothetical_reduction",
                            hypothetical_reduction,
                        )?;
                    }
                    ConsentTerm::CascadeNamed { year: _ } => {}
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// `Decimal` distinguishes `-0` from `0`; a signed zero is still zero and must not be refused
    /// (`parse_usd` on an accounting `(0.00)` produces exactly this).
    #[test]
    fn negative_zero_is_not_a_negative_amount() {
        let mut z = dec!(0);
        z.set_sign_negative(true);
        assert!(z.is_sign_negative(), "the fixture must actually be -0");
        usd("f", &z).expect("-0 is zero and must record");
    }

    /// The refusal text must carry the field name, the offending value, and the reason — a bare
    /// "invalid payload" would leave the filer with nowhere to look.
    #[test]
    fn the_refusal_names_field_value_and_reason() {
        let e = usd("Acquire.usd_cost", &dec!(-1.5)).unwrap_err();
        let s = e.to_string();
        assert!(s.contains("Acquire.usd_cost"), "{s}");
        assert!(s.contains("-1.5"), "{s}");
        assert!(s.contains("§1012"), "{s}");
    }

    #[test]
    fn a_negative_sat_is_refused_and_zero_is_not() {
        assert!(sat("f", -1).is_err());
        sat("f", 0).expect("a zero-sat row is not an error");
        sat_opt("f", None).expect("an absent optional carries no value to check");
    }
}
