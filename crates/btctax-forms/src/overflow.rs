//! Pagination for a form whose rows exceed one copy's grid. Each chunk is filled on a fresh copy of
//! the template **on its ORIGINAL field names** (and geometry-verified there — that is what
//! [`crate::fill8949`] returns), THEN the verified copies are merged into one document with each
//! copy's ROOT field renamed so the copies do NOT share a `/V` (the ISO 32000 same-name trap).
//! Per-copy totals ride along on each copy; Schedule D aggregates the grand totals separately.
//!
//! ★★ **FR-218 — a Form 8949 copy files only the pages it has rows for.** [`retain_pages`] reduces a
//! filled copy to those pages (the page tree *and* the AcroForm subtrees whose widgets live on the
//! others), and [`merge_pages`] then assembles the surviving pages in an explicit [`PagePick`] order.
//! [`independent_part_plan`] builds that order for a form whose two parts paginate independently.

use crate::error::FormsError;
use crate::pdf;
use lopdf::{Object, ObjectId, StringFormat};
use std::collections::HashSet;

fn acroform_fields_root(doc: &lopdf::Document) -> Result<ObjectId, FormsError> {
    let acro = doc.catalog()?.get(b"AcroForm")?.as_reference()?;
    let fields = doc.get_dictionary(acro)?.get(b"Fields")?.as_array()?;
    fields
        .first()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| FormsError::Structure("AcroForm /Fields is empty".into()))
}

/// One page of the merged document: the filled copy it comes from, and the 0-based index of the page
/// **within that copy's own page list** (which [`retain_pages`] may already have shortened).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePick {
    pub copy: usize,
    pub page: usize,
}

/// Merge already-filled, already-verified single-copy PDFs into one document, copy by copy. Copy 0 is
/// the base; copies 1.. have their root field `/T` renamed (uniquifying every field's fully-qualified
/// name) and their pages + form fields appended.
pub fn merge_copies(copies: &[Vec<u8>]) -> Result<Vec<u8>, FormsError> {
    merge(copies, None)
}

/// [`merge_copies`], with the page order given explicitly as a [`PagePick`] plan.
///
/// ★★ **The ONLY thing the plan changes is the order of the page-tree `/Kids` array.** The same page
/// objects, carrying the same widget annotations and the same `/V` values, in a different sequence —
/// nothing is re-filled, re-chunked or re-totalled, and every copy is still geometry-verified before
/// it gets here. That is enforced rather than promised: the plan must be a **bijection onto every page
/// of every copy** ([`plan_covers_every_page_once`]), so it can neither drop a page nor emit one
/// twice. Row conservation is held independently by
/// `btctax-forms/tests/overflow.rs::every_entered_8949_row_is_emitted_exactly_once_across_all_pages`
/// (a dropped row reds it). A permutation is the whole fix precisely *because* an 8949 row that moved,
/// vanished or doubled would be a wrong return, while the page order is a reading convenience.
pub fn merge_pages(copies: &[Vec<u8>], plan: &[PagePick]) -> Result<Vec<u8>, FormsError> {
    merge(copies, Some(plan))
}

fn merge(copies: &[Vec<u8>], plan: Option<&[PagePick]>) -> Result<Vec<u8>, FormsError> {
    let mut out = pdf::load(&copies[0])?;
    let pages_root = out.catalog()?.get(b"Pages")?.as_reference()?;
    let out_acro = out.catalog()?.get(b"AcroForm")?.as_reference()?;

    // Copy 0's pages, in document order — the first entry of the per-copy page table below.
    let mut per_copy_pages: Vec<Vec<ObjectId>> = vec![out.get_pages().into_values().collect()];
    for (k, bytes) in copies.iter().enumerate().skip(1) {
        let mut frag = pdf::load(bytes)?;
        frag.renumber_objects_with(out.max_id + 1);
        out.max_id = frag.max_id;

        let root_id = acroform_fields_root(&frag)?;
        let page_ids: Vec<ObjectId> = frag.get_pages().into_values().collect();

        // Absorb the fragment's (renumbered) objects.
        for (id, obj) in std::mem::take(&mut frag.objects) {
            out.objects.insert(id, obj);
        }
        // Uniquify this copy's field names by renaming ONLY the root component — every descendant's
        // fully-qualified name inherits the new prefix, so no leaf shares a /V with another copy.
        out.get_dictionary_mut(root_id)?.set(
            "T",
            Object::String(format!("btctaxcopy{k}").into_bytes(), StringFormat::Literal),
        );
        // Re-parent the copied pages under the base document's page tree.
        for pid in &page_ids {
            out.get_dictionary_mut(*pid)?
                .set("Parent", Object::Reference(pages_root));
        }
        per_copy_pages.push(page_ids.clone());
        // Register this copy's form as another top-level AcroForm field.
        out.get_dictionary_mut(out_acro)?
            .get_mut(b"Fields")?
            .as_array_mut()?
            .push(Object::Reference(root_id));
    }

    // The page order: the caller's plan, or copy by copy.
    let picks: Vec<PagePick> = match plan {
        Some(p) => p.to_vec(),
        None => per_copy_pages
            .iter()
            .enumerate()
            .flat_map(|(copy, ps)| (0..ps.len()).map(move |page| PagePick { copy, page }))
            .collect(),
    };
    plan_covers_every_page_once(&picks, &per_copy_pages)?;

    // The page tree must be FLAT — `/Kids` is exactly the pages, no intermediate `/Pages` node — or a
    // rewrite of the array is not a permutation of the document. Measured flat on every bundled Form
    // 8949 and Form 8283; checked here so a future template that nests fails CLOSED instead of
    // emitting a silently mis-ordered (or page-shy) filed form.
    let existing: Vec<ObjectId> = out
        .get_dictionary(pages_root)?
        .get(b"Kids")?
        .as_array()?
        .iter()
        .filter_map(|o| o.as_reference().ok())
        .collect();
    flat_page_tree(&existing, &per_copy_pages[0])?;

    let kids: Vec<Object> = picks
        .iter()
        .map(|k| Object::Reference(per_copy_pages[k.copy][k.page]))
        .collect();
    let count = kids.len() as i64;
    let pd = out.get_dictionary_mut(pages_root)?;
    pd.set("Kids", Object::Array(kids));
    pd.set("Count", Object::Integer(count));

    pdf::strip_nondeterminism(&mut out);
    pdf::save(&mut out)
}

/// Refuse unless the plan names **every page of every copy exactly once**.
///
/// ★ This is the guard that makes a merge a permutation. It replaced a weaker "every copy has the same
/// page count" rectangle check, which FR-218 retired: once the parts paginate independently the copies
/// are legitimately ragged (a long-term-only filer's copies carry one page each), so the invariant
/// that actually matters is not *rectangular* but *total* — a filled, geometry-verified page the plan
/// forgets is a page of the filer's return that never reaches the PDF.
fn plan_covers_every_page_once(
    picks: &[PagePick],
    per_copy_pages: &[Vec<ObjectId>],
) -> Result<(), FormsError> {
    let shape = || -> String {
        format!(
            "copies have {:?} page(s)",
            per_copy_pages.iter().map(Vec::len).collect::<Vec<_>>()
        )
    };
    let mut seen: Vec<Vec<bool>> = per_copy_pages
        .iter()
        .map(|p| vec![false; p.len()])
        .collect();
    for k in picks {
        let slot = seen
            .get_mut(k.copy)
            .and_then(|c| c.get_mut(k.page))
            .ok_or_else(|| {
                FormsError::Structure(format!(
                    "page plan names copy {} page {}, which does not exist ({})",
                    k.copy,
                    k.page,
                    shape()
                ))
            })?;
        if *slot {
            return Err(FormsError::Structure(format!(
                "page plan names copy {} page {} twice — a filed page would be emitted twice ({})",
                k.copy,
                k.page,
                shape()
            )));
        }
        *slot = true;
    }
    let missed: Vec<(usize, usize)> = seen
        .iter()
        .enumerate()
        .flat_map(|(c, pages)| {
            pages
                .iter()
                .enumerate()
                .filter(|(_, s)| !**s)
                .map(move |(p, _)| (c, p))
        })
        .collect();
    if !missed.is_empty() {
        return Err(FormsError::Structure(format!(
            "page plan drops {} filled page(s) {missed:?} — every page of every copy must be filed \
             exactly once ({})",
            missed.len(),
            shape()
        )));
    }
    Ok(())
}

/// Refuse unless the base document's page-tree `/Kids` is EXACTLY the flat page list this rewrite was
/// computed from.
///
/// The rewrite replaces that array, so anything else in it — an intermediate `/Pages` node, a page the
/// caller does not know about — would be dropped from the document. Every bundled Form 8949 and Form
/// 8283 measures flat; this is what makes a future template that does not fail CLOSED rather than emit
/// a filed form with pages missing.
fn flat_page_tree(existing: &[ObjectId], expected: &[ObjectId]) -> Result<(), FormsError> {
    if existing != expected {
        return Err(FormsError::Structure(format!(
            "page-tree /Kids is not the flat page list this rewrite permutes: {} kids, \
             {} pages expected",
            existing.len(),
            expected.len()
        )));
    }
    Ok(())
}

/// ★★★ **FR-218 — the page plan for a form whose parts paginate INDEPENDENTLY.**
///
/// `groups` is one `(template page index, page count)` entry per part — e.g. `[(0, 1), (1, 2)]` for a
/// filer with one Part I page and two Part II pages. Copy `k` is filled with each part's page `k`, and
/// a part that has run out of pages contributes **no page at all** to that copy ([`retain_pages`]
/// removes it), so copy `k`'s own page list is short by one and the pick's page index has to be
/// counted rather than assumed.
///
/// The returned plan groups the parts — every Part I page, then every Part II page (FR-113) — by
/// ordering the groups on their **template** page index rather than on a hardcoded "short first".
///
/// The authority for emitting one part and not the other is `i8949` itself
/// (`design/forms/extract/i8949--2024.txt:422-424`): *"You don't need to complete and file an entire
/// copy of Form 8949 (Parts I and II) if you can check a single box to describe all your
/// transactions. In that case, complete and file **either Part I or II** and check the box that
/// describes the transactions."*
pub fn independent_part_plan(groups: &[(usize, usize)]) -> Vec<PagePick> {
    let mut g: Vec<(usize, usize)> = groups.iter().copied().filter(|(_, n)| *n > 0).collect();
    g.sort_by_key(|(page, _)| *page);
    let mut plan = Vec::with_capacity(g.iter().map(|(_, n)| *n).sum());
    for (page, count) in &g {
        for k in 0..*count {
            // Within copy k this part's page sits after every OTHER part's page that both precedes it
            // on the template AND is actually present in copy k.
            let before = g.iter().filter(|(p, n)| p < page && k < *n).count();
            plan.push(PagePick {
                copy: k,
                page: before,
            });
        }
    }
    plan
}

/// ★★★ **FR-218 — reduce a filled copy to the pages it actually files.**
///
/// `keep` is the strictly increasing set of 0-based page indices (document order) to retain. The pages
/// not named are removed from the page tree **and** the AcroForm field subtrees whose widget
/// annotations live on them are removed from the form, so the emitted file carries no field — and in
/// particular no "Name(s) shown on return" or SSN — belonging to a page nobody files. Unreferenced
/// objects are then pruned.
///
/// Keeping every page is a no-op, so the common both-parts case is byte-identical to the unreduced
/// fill.
///
/// A field subtree that straddles a kept and a dropped page **refuses**: removing it would delete a
/// filed page's cells, and keeping it would leave a dropped page's widget in the form. Measured on the
/// bundled TY2024 and TY2025 Form 8949 templates, each root kid (`Page1[0]` / `Page2[0]`) holds
/// exactly one page's widgets — 122/122 and 101/101 — so the straddle branch is a fail-closed guard
/// against a future revision, not a live case.
pub(crate) fn retain_pages(doc: &mut lopdf::Document, keep: &[usize]) -> Result<(), FormsError> {
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    if keep.is_empty() {
        return Err(FormsError::Structure(
            "a filed form must keep at least one page".into(),
        ));
    }
    if keep.windows(2).any(|w| w[0] >= w[1]) || keep.iter().any(|&i| i >= pages.len()) {
        return Err(FormsError::Structure(format!(
            "pages to keep must be a strictly increasing subset of the template's {} page(s); \
             got {keep:?}",
            pages.len()
        )));
    }
    if keep.len() == pages.len() {
        return Ok(()); // nothing dropped — leave the document (and its bytes) alone
    }

    // The widget annotations that live on the pages being dropped.
    let mut dropped_widgets: HashSet<ObjectId> = HashSet::new();
    for (i, pid) in pages.iter().enumerate() {
        if keep.contains(&i) {
            continue;
        }
        for id in page_annots(doc, *pid) {
            dropped_widgets.insert(id);
        }
    }

    // Which AcroForm subtrees go with them.
    let acro = doc.catalog()?.get(b"AcroForm")?.as_reference()?;
    let roots: Vec<ObjectId> = doc
        .get_dictionary(acro)?
        .get(b"Fields")?
        .as_array()?
        .iter()
        .filter_map(|o| o.as_reference().ok())
        .collect();
    let mut drop_roots: Vec<ObjectId> = Vec::new();
    let mut drop_kids: Vec<(ObjectId, ObjectId)> = Vec::new();
    for root in &roots {
        let kids = direct_kids(doc, *root);
        if kids.is_empty() {
            if all_dropped(doc, *root, &dropped_widgets)? {
                drop_roots.push(*root);
            }
            continue;
        }
        let mut kept_any = false;
        for kid in kids {
            if all_dropped(doc, kid, &dropped_widgets)? {
                drop_kids.push((*root, kid));
            } else {
                kept_any = true;
            }
        }
        if !kept_any {
            drop_roots.push(*root);
        }
    }
    for (root, kid) in drop_kids {
        let arr = doc
            .get_dictionary_mut(root)?
            .get_mut(b"Kids")?
            .as_array_mut()?;
        arr.retain(|o| o.as_reference().ok() != Some(kid));
    }
    if !drop_roots.is_empty() {
        let arr = doc
            .get_dictionary_mut(acro)?
            .get_mut(b"Fields")?
            .as_array_mut()?;
        arr.retain(|o| {
            !o.as_reference()
                .ok()
                .is_some_and(|id| drop_roots.contains(&id))
        });
    }

    // The page tree: exactly the kept pages, in order.
    let pages_root = doc.catalog()?.get(b"Pages")?.as_reference()?;
    let existing: Vec<ObjectId> = doc
        .get_dictionary(pages_root)?
        .get(b"Kids")?
        .as_array()?
        .iter()
        .filter_map(|o| o.as_reference().ok())
        .collect();
    flat_page_tree(&existing, &pages)?;
    let kids: Vec<Object> = keep.iter().map(|&i| Object::Reference(pages[i])).collect();
    let count = kids.len() as i64;
    let pd = doc.get_dictionary_mut(pages_root)?;
    pd.set("Kids", Object::Array(kids));
    pd.set("Count", Object::Integer(count));

    doc.prune_objects();
    Ok(())
}

/// A page's `/Annots` object ids (the array may be an indirect reference).
fn page_annots(doc: &lopdf::Document, page: ObjectId) -> Vec<ObjectId> {
    doc.get_dictionary(page)
        .ok()
        .and_then(|d| d.get(b"Annots").ok())
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default()
}

/// The direct `/Kids` of a field-tree node.
fn direct_kids(doc: &lopdf::Document, id: ObjectId) -> Vec<ObjectId> {
    doc.get_dictionary(id)
        .ok()
        .and_then(|d| d.get(b"Kids").ok())
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_array().ok())
        .map(|a| a.iter().filter_map(|o| o.as_reference().ok()).collect())
        .unwrap_or_default()
}

/// Whether every terminal widget under `id` sits on a dropped page. A node whose widgets straddle a
/// kept and a dropped page is an error, not a verdict.
fn all_dropped(
    doc: &lopdf::Document,
    id: ObjectId,
    dropped: &HashSet<ObjectId>,
) -> Result<bool, FormsError> {
    let mut leaves = Vec::new();
    field_leaves(doc, id, &mut leaves);
    let n = leaves.iter().filter(|l| dropped.contains(l)).count();
    if n == 0 {
        return Ok(false);
    }
    if n != leaves.len() {
        return Err(FormsError::Structure(format!(
            "AcroForm field subtree {id:?} straddles a kept and a dropped page ({n} of {} widgets \
             are on dropped pages) — reducing this form would either delete a filed page's cells or \
             leave an unfiled page's widget in the form",
            leaves.len()
        )));
    }
    Ok(true)
}

/// Every terminal node under `id` in an AcroForm field tree.
fn field_leaves(doc: &lopdf::Document, id: ObjectId, out: &mut Vec<ObjectId>) {
    let kids = direct_kids(doc, id);
    if kids.is_empty() {
        out.push(id);
    } else {
        for k in kids {
            field_leaves(doc, k, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> ObjectId {
        (n, 0)
    }

    /// ★ B1 — the bijection refusal, seen red on its own removal. Each case below is a plan that would
    /// silently change the filed page set: delete the `*slot` duplicate branch and case 2 emits a page
    /// twice; delete the `missed` branch and case 3 drops a filled page from the return.
    #[test]
    fn a_page_plan_that_is_not_a_bijection_refuses() {
        let two_copies = vec![vec![id(1), id(2)], vec![id(3), id(4)]];
        let all = [
            PagePick { copy: 0, page: 0 },
            PagePick { copy: 1, page: 0 },
            PagePick { copy: 0, page: 1 },
            PagePick { copy: 1, page: 1 },
        ];
        assert!(
            plan_covers_every_page_once(&all, &two_copies).is_ok(),
            "the transpose is a bijection"
        );
        // ★ A duplicate that covers every slot ANYWAY — five picks over four pages. This is the case
        //   only the duplicate branch can catch: a plan that merely repeats a page while also missing
        //   one is caught by the `missed` branch instead, so it would NOT kill this guard.
        let twice_and_complete = [
            PagePick { copy: 0, page: 0 },
            PagePick { copy: 0, page: 0 },
            PagePick { copy: 0, page: 1 },
            PagePick { copy: 1, page: 0 },
            PagePick { copy: 1, page: 1 },
        ];
        assert!(
            plan_covers_every_page_once(&twice_and_complete, &two_copies).is_err(),
            "a page named twice must refuse — the page printed twice IS the FR-218 symptom the owner \
             held on paper"
        );
        assert!(
            plan_covers_every_page_once(&all[..3], &two_copies).is_err(),
            "a plan that forgets a filled page must refuse"
        );
        assert!(
            plan_covers_every_page_once(&[PagePick { copy: 0, page: 7 }], &[vec![id(1)]]).is_err(),
            "a page that does not exist must refuse"
        );
        // ★ RAGGED IS NOW LEGAL — this is exactly what FR-218's fix produces, and the retired
        //   rectangle check refused it.
        assert!(
            plan_covers_every_page_once(
                &[
                    PagePick { copy: 0, page: 0 },
                    PagePick { copy: 0, page: 1 },
                    PagePick { copy: 1, page: 0 },
                ],
                &[vec![id(1), id(2)], vec![id(3)]]
            )
            .is_ok(),
            "one Part I page + two Part II pages is a 2-page copy beside a 1-page copy"
        );
    }

    /// ★ B1 — the flat-page-tree refusal, seen red on its own removal: return `Ok(())`
    /// unconditionally and the third case below passes, which is the document that would lose a page
    /// (or a whole nested subtree) when `/Kids` is rewritten.
    #[test]
    fn a_page_tree_that_is_not_the_flat_page_list_refuses() {
        assert!(
            flat_page_tree(&[id(1), id(2)], &[id(1), id(2)]).is_ok(),
            "the measured shape: /Kids IS the page list"
        );
        assert!(
            flat_page_tree(&[id(9), id(1), id(2)], &[id(1), id(2)]).is_err(),
            "an extra kid (an intermediate /Pages node, say) would be dropped by the rewrite"
        );
        assert!(
            flat_page_tree(&[id(2), id(1)], &[id(1), id(2)]).is_err(),
            "the same pages in another order is not the list this rewrite was computed from"
        );
    }

    /// ★★★ **FR-218 — the plan a filer's page counts produce.** Part I is template page 0, Part II
    /// page 1. The owner's shape is the fourth case: 1 Part I page and 2 Part II pages is **3** pages,
    /// and copy 1 has no Part I page at all, so its Part II page is that copy's page **0**.
    #[test]
    fn the_independent_part_plan_emits_one_page_per_part_page() {
        let pick = |copy, page| PagePick { copy, page };
        // Both parts, one page each — one 2-page copy.
        assert_eq!(
            independent_part_plan(&[(0, 1), (1, 1)]),
            vec![pick(0, 0), pick(0, 1)]
        );
        // ★ Long-term only: ZERO Part I pages. Two copies, one page each.
        assert_eq!(
            independent_part_plan(&[(0, 0), (1, 2)]),
            vec![pick(0, 0), pick(1, 0)],
            "a long-term-only filer files no Part I page (i8949: \"either Part I or II\")"
        );
        // ★ Short-term only: ZERO Part II pages.
        assert_eq!(
            independent_part_plan(&[(0, 2), (1, 0)]),
            vec![pick(0, 0), pick(1, 0)],
            "a short-term-only filer files no Part II page"
        );
        // ★ The owner's 28-row shape: 1 Part I page + 2 Part II pages = 3 pages, grouped.
        assert_eq!(
            independent_part_plan(&[(0, 1), (1, 2)]),
            vec![pick(0, 0), pick(0, 1), pick(1, 0)],
            "Part I page, then both Part II pages — and copy 1 carries only its Part II page"
        );
        // Mixed and equal: the transpose the rectangle check used to be the only shape allowed.
        assert_eq!(
            independent_part_plan(&[(0, 2), (1, 2)]),
            vec![pick(0, 0), pick(1, 0), pick(0, 1), pick(1, 1)]
        );
        // 3 Part I pages + 2 Part II pages: copy 2 has no Part II page, so nothing is picked for it.
        assert_eq!(
            independent_part_plan(&[(0, 3), (1, 2)]),
            vec![pick(0, 0), pick(1, 0), pick(2, 0), pick(0, 1), pick(1, 1)]
        );
    }
}
