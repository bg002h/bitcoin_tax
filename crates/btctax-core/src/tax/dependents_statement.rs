//! §G-28/B2 — the IRS-mandated continuation statement for a return with **more than four dependents**.
//!
//! Form 1040 page 1 prints four dependent rows. btctax used to REFUSE a fifth, which was the wrong
//! remedy: the form supplies its own. i1040gi (2024), "Dependents, Qualifying Child for Child Tax
//! Credit, and Credit for Other Dependents", Step 1:
//!
//! > *"If you have more than four dependents, check the box under Dependents on page 1 of Form 1040 or
//! > 1040-SR and include a statement showing the information required in columns (1) through (4)."*
//!
//! ★★★ **The statement is a TRANSCRIPTION of the grid, not a prose summary of it.** All four columns,
//! in the form's own numbering and wording, with column (4) rendered as the two empty check positions
//! the form prints. A draft that replaced column (4) with the sentence *"NOT CLAIMED for any
//! dependent"* was rejected in review: it converts a lawful blank into an affirmative assertion on a
//! page signed under 26 USC §6065, which is the "an entry is testimony" failure in the direction the
//! doctrine forbids. The filer IS told about the forgone credit — by `Advisory::CtcOdcOmitted`, which
//! is the channel for that. The filed page is not.

use crate::tax::packet::{DependentRow, ReturnHeader};

/// The number of dependent rows Form 1040 page 1 physically prints.
///
/// Form 1040 (2024) prints four `Table_Dependents[0].RowN[0]` groups. ★ Declared HERE because **core**
/// decides whether a continuation statement is required, while the MAP independently declares the same
/// number as `header.dependent_rows.len()`. `fill_form_1040_full_with_map` REFUSES when the two
/// disagree — a fill-time guard rather than a test, because only TY2024 has a `[header]` block today
/// and a future year's map is unwritten work. Two sources for one number, held together by a guard
/// that fails closed, is the honest arrangement when neither side can be deleted.
pub const DEPENDENTS_GRID_ROWS: usize = 4;

impl ReturnHeader {
    /// The page-1 grid rows and the continuation-statement rows, in **capture order**.
    ///
    /// ★ `split_at` is the point: the two halves are a partition of `self.dependents` by construction —
    /// concatenating them reproduces the input, in order, with nothing duplicated and nothing dropped.
    /// A hand-rolled `take`/`skip` pair would make that a property to test rather than a fact.
    ///
    /// ★★ CAPTURE ORDER is the contract, and nothing sorts. A sort would let a re-run print a different
    /// four than a previously mailed statement. v1 claims no CTC/ODC, so no dependent is "worth more"
    /// on page 1 and there is nothing to optimise for.
    pub fn dependents_split(&self) -> (&[DependentRow], &[DependentRow]) {
        self.dependents
            .split_at(self.dependents.len().min(DEPENDENTS_GRID_ROWS))
    }

    /// Form 1040 page 1: the box beside the Dependents block.
    ///
    /// ★★ ONE predicate, TWO consumers — the checkbox write and the statement emission. A checked box
    /// with no attached statement is an incomplete return; an attached statement with an unchecked box
    /// is a return that contradicts its own attachment. Neither is expressible while both read this.
    pub fn more_than_four_dependents(&self) -> bool {
        !self.dependents_split().1.is_empty()
    }
}

/// The continuation statement's contents. `rows` are the OVERFLOW rows only — the statement
/// supplements page 1 rather than restating it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependentsStatement {
    pub year: i32,
    /// The name line as it appears on the return (the joint line on MFJ).
    pub name_line: String,
    /// The taxpayer's SSN, so a detached page can be matched to its return.
    ///
    /// ★ A `String`, and the caller renders it. `Ssn`'s `Debug` is deliberately MASKED — an SSN in a
    /// log is a PII incident — so the type resists printing by design. A filed page is the one place
    /// it must print in full, and that has to be an explicit, greppable `hyphenated()` at the call
    /// site rather than something a `Display` impl would do everywhere by accident.
    pub taxpayer_ssn: String,
    /// Every dependent claimed on the return.
    pub total: usize,
    /// How many the page-1 grid carries — `min(total, DEPENDENTS_GRID_ROWS)`.
    pub on_form: usize,
    pub rows: Vec<DependentRow>,
}

/// `None` when the grid holds them all — the common case, and the correct one.
pub fn dependents_statement(header: &ReturnHeader, year: i32) -> Option<DependentsStatement> {
    let (grid, overflow) = header.dependents_split();
    if overflow.is_empty() {
        return None;
    }
    Some(DependentsStatement {
        year,
        name_line: header.name_line.clone(),
        // ★ `hyphenated()` at the call site, deliberately explicit — see the field's doc.
        taxpayer_ssn: header.taxpayer.ssn.hyphenated(),
        total: header.dependents.len(),
        on_form: grid.len(),
        rows: overflow.to_vec(),
    })
}

impl DependentsStatement {
    /// The page a filer attaches.
    ///
    /// ★ Column headings are the FORM's, verbatim from `design/forms/extract/f1040--2024.txt:40-41`:
    /// *"(1) First name / Last name"*, *"(2) Social security number"*, *"(3) Relationship to you"*,
    /// *"(4) Check the box if qualifies for (see instructions): Child tax credit | Credit for other
    /// dependents"*. Column (4) prints its two check positions **empty**, exactly as page 1 does.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "Form 1040 ({}) — CONTINUATION STATEMENT: DEPENDENTS\n\
             Attach to Form 1040. The box under Dependents on page 1 is checked.\n\n\
             Name(s) shown on return: {}\n\
             Your social security number: {}\n\n\
             This return claims {} dependents. Dependents 1-{} are listed on page 1 of Form 1040;\n\
             dependents {}-{} are listed below. Together they are the complete list.\n\n",
            self.year,
            self.name_line,
            self.taxpayer_ssn,
            self.total,
            self.on_form,
            self.on_form + 1,
            self.total,
        ));
        out.push_str(
            "                                                                     (4) Check the box if qualifies for\n\
             \x20     (1) First name    Last name          (2) Social security   (3) Relationship        Child tax    Credit for other\n\
             \x20                                              number               to you                credit        dependents\n\
             \x20    ---------------------------------------------------------------------------------------------------------------\n",
        );
        for (i, d) in self.rows.iter().enumerate() {
            out.push_str(&format!(
                " {:>2}  {:<34} {:<21} {:<21} [  ]          [  ]\n",
                self.on_form + i + 1,
                d.name,
                d.ssn.hyphenated(),
                d.relationship,
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(n: usize) -> ReturnHeader {
        let mut h = crate::tax::testonly::kitchen_sink_header();
        h.dependents = (0..n)
            .map(|i| DependentRow {
                name: format!("Child {i}"),
                ssn: crate::tax::packet::Ssn::canonical(&format!("111-22-{:04}", i)).unwrap(),
                relationship: "Daughter".into(),
            })
            .collect();
        h
    }

    /// ★★★ THE PARTITION. grid ++ statement == the captured list, IN ORDER, disjoint — for every count
    /// across the boundary. `split_at` makes it true by construction; this guards a future refactor.
    #[test]
    fn the_split_is_an_exact_ordered_partition() {
        for n in 0..=12 {
            let h = hdr(n);
            let (grid, overflow) = h.dependents_split();
            let rejoined: Vec<_> = grid.iter().chain(overflow).cloned().collect();
            assert_eq!(rejoined, h.dependents, "n={n}: the two halves must rejoin");
            assert!(grid.len() <= DEPENDENTS_GRID_ROWS, "n={n}");
            assert_eq!(grid.len(), n.min(DEPENDENTS_GRID_ROWS), "n={n}");
            // The box and the statement are ONE decision.
            assert_eq!(
                h.more_than_four_dependents(),
                !overflow.is_empty(),
                "n={n}: the checkbox predicate and the statement must agree"
            );
            assert_eq!(
                h.more_than_four_dependents(),
                n > DEPENDENTS_GRID_ROWS,
                "n={n}"
            );
        }
    }

    /// The statement exists exactly when the grid overflows, and carries only the overflow.
    #[test]
    fn the_statement_supplements_page_one_rather_than_restating_it() {
        for n in 0..=DEPENDENTS_GRID_ROWS {
            assert!(
                dependents_statement(&hdr(n), 2024).is_none(),
                "n={n}: the grid holds them all — no statement"
            );
        }
        let st = dependents_statement(&hdr(9), 2024).expect("9 > 4 ⇒ a statement");
        assert_eq!((st.total, st.on_form, st.rows.len()), (9, 4, 5));
        assert_eq!(st.rows[0].name, "Child 4", "the FIFTH dependent starts it");
    }

    /// ★★★ COLUMN (4) IS BLANK, AND NO SENTENCE ASSERTS ANYTHING ABOUT IT.
    ///
    /// A draft replaced the column with *"NOT CLAIMED for any dependent on this return"*. On a page
    /// signed under §6065 that converts a lawful blank into an affirmative assertion — "an entry is
    /// testimony" in the direction the doctrine forbids. The forgone credit is disclosed by
    /// `Advisory::CtcOdcOmitted`; the filed page is not that channel.
    #[test]
    fn the_rendered_statement_transcribes_all_four_columns_and_claims_nothing() {
        let h = hdr(6);
        let st = dependents_statement(&h, 2024).unwrap();
        let r = st.render();
        for heading in [
            "(1) First name",
            "Last name",
            "(2) Social security",
            "(3) Relationship",
            "(4) Check the box if qualifies for",
            "Child tax",
            "Credit for other",
        ] {
            assert!(
                r.contains(heading),
                "missing the form's own {heading:?}:\n{r}"
            );
        }
        assert!(
            r.contains("[  ]"),
            "column (4) prints its check positions EMPTY"
        );
        for forbidden in [
            "NOT CLAIMED",
            "not claimed",
            "does not qualify",
            "no credit",
        ] {
            assert!(
                !r.contains(forbidden),
                "the statement must ASSERT nothing about column (4); found {forbidden:?}"
            );
        }
        // It must identify itself and its return, or a detached page is unattributable.
        assert!(
            r.contains(&h.taxpayer.ssn.hyphenated())
                && r.contains(&h.name_line)
                && r.contains("1040"),
            "a detached page must be attributable to its return"
        );
        // Only the overflow rows appear.
        assert!(r.contains("Child 4") && r.contains("Child 5"));
        assert!(
            !r.contains("Child 0"),
            "page 1 already carries the first four"
        );
    }
}
