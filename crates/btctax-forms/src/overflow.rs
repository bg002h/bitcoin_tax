//! Pagination for > 11 rows per part. Each 11-row chunk is filled on a fresh copy of the template
//! **on its ORIGINAL field names** (and geometry-verified there — that is what [`crate::fill8949`]
//! returns), THEN the verified copies are merged into one document with each copy's ROOT field
//! renamed so the copies do NOT share a `/V` (the ISO 32000 same-name trap). Per-copy totals ride
//! along on each copy; Schedule D aggregates the grand totals separately.

use crate::error::FormsError;
use crate::pdf;
use lopdf::{Object, ObjectId, StringFormat};

fn acroform_fields_root(doc: &lopdf::Document) -> Result<ObjectId, FormsError> {
    let acro = doc.catalog()?.get(b"AcroForm")?.as_reference()?;
    let fields = doc.get_dictionary(acro)?.get(b"Fields")?.as_array()?;
    fields
        .first()
        .and_then(|o| o.as_reference().ok())
        .ok_or_else(|| FormsError::Structure("AcroForm /Fields is empty".into()))
}

/// How the merged document orders the pages of the copies it was built from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOrder {
    /// Copy by copy: copy 0's pages, then copy 1's, … — every copy stays a contiguous physical form.
    ByCopy,
    /// ★ FR-113 — by POSITION within the copy: every copy's page 1, then every copy's page 2, … On
    /// Form 8949 that groups Part I before Part II, which is how a human assembles the paper stack.
    /// Requires every copy to have the same page count (a pure permutation, nothing added or lost).
    ByPosition,
}

/// Merge already-filled, already-verified single-copy Form 8949 PDFs into one document, copy by copy
/// ([`PageOrder::ByCopy`]). Copy 0 is the base; copies 1.. have their root field `/T` renamed
/// (uniquifying every field's fully-qualified name) and their pages + form fields appended.
pub fn merge_copies(copies: &[Vec<u8>]) -> Result<Vec<u8>, FormsError> {
    merge_copies_ordered(copies, PageOrder::ByCopy)
}

/// [`merge_copies`], with the page order chosen — see [`PageOrder`].
///
/// ★★ **FR-113: the ONLY thing `ByPosition` changes is the order of the page-tree `/Kids` array.**
/// The same page objects, carrying the same widget annotations and the same `/V` values, in a
/// different sequence — nothing is re-filled, re-chunked or re-totalled, and every copy is still
/// geometry-verified before it gets here. Row conservation is therefore untouched by construction,
/// and is held independently by
/// `btctax-forms/tests/overflow.rs::every_entered_8949_row_is_emitted_exactly_once_across_all_pages`
/// (a dropped row reds it). A permutation is the whole fix precisely *because* an 8949 row that
/// moved, vanished or doubled would be a wrong return, while the page order is a reading convenience.
pub fn merge_copies_ordered(copies: &[Vec<u8>], order: PageOrder) -> Result<Vec<u8>, FormsError> {
    let mut out = pdf::load(&copies[0])?;
    let pages_root = out.catalog()?.get(b"Pages")?.as_reference()?;
    let out_acro = out.catalog()?.get(b"AcroForm")?.as_reference()?;

    // Copy 0's pages, in document order — the first entry of the per-copy page table below.
    let mut per_copy_pages: Vec<Vec<ObjectId>> = vec![out.get_pages().into_values().collect()];
    let mut appended_pages: Vec<ObjectId> = Vec::new();
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
            appended_pages.push(*pid);
        }
        per_copy_pages.push(page_ids.clone());
        // Register this copy's form as another top-level AcroForm field.
        out.get_dictionary_mut(out_acro)?
            .get_mut(b"Fields")?
            .as_array_mut()?
            .push(Object::Reference(root_id));
    }

    // Splice the appended pages into the page tree and fix /Count.
    match order {
        PageOrder::ByCopy => {
            let pd = out.get_dictionary_mut(pages_root)?;
            let kids = pd.get_mut(b"Kids")?.as_array_mut()?;
            for pid in &appended_pages {
                kids.push(Object::Reference(*pid));
            }
            let count = kids.len() as i64;
            pd.set("Count", Object::Integer(count));
        }
        // ★ FR-113 — the transpose: page 1 of every copy, then page 2 of every copy, … Written as a
        //   rebuild of `/Kids` rather than a push, so the two branches are visibly the same page set
        //   in two orders.
        PageOrder::ByPosition => {
            let per_copy = pages_per_copy(&per_copy_pages)?;
            // The page tree must be FLAT — `/Kids` is exactly the pages, no intermediate `/Pages`
            // node — or a permutation of the array is not a permutation of the document. Measured
            // flat on every bundled Form 8949; checked here so a future template that nests fails
            // CLOSED instead of emitting a silently mis-ordered filed form.
            let existing: Vec<ObjectId> = out
                .get_dictionary(pages_root)?
                .get(b"Kids")?
                .as_array()?
                .iter()
                .filter_map(|o| o.as_reference().ok())
                .collect();
            flat_page_tree(&existing, &per_copy_pages[0])?;
            let mut kids = Vec::with_capacity(per_copy * per_copy_pages.len());
            for pos in 0..per_copy {
                for pages in &per_copy_pages {
                    kids.push(Object::Reference(pages[pos]));
                }
            }
            let count = kids.len() as i64;
            let pd = out.get_dictionary_mut(pages_root)?;
            pd.set("Kids", Object::Array(kids));
            pd.set("Count", Object::Integer(count));
        }
    }

    pdf::strip_nondeterminism(&mut out);
    pdf::save(&mut out)
}

/// The page count every copy shares, for [`PageOrder::ByPosition`]. A ragged set refuses: the
/// transpose is only a permutation when the rows are the same length, and a copy with a different
/// page count means the caller merged two different forms.
fn pages_per_copy(per_copy_pages: &[Vec<ObjectId>]) -> Result<usize, FormsError> {
    let per_copy = per_copy_pages.first().map_or(0, Vec::len);
    if per_copy == 0 || per_copy_pages.iter().any(|p| p.len() != per_copy) {
        return Err(FormsError::Structure(format!(
            "grouping by page position needs every copy to have the same, non-zero page count; got {:?}",
            per_copy_pages.iter().map(Vec::len).collect::<Vec<_>>()
        )));
    }
    Ok(per_copy)
}

/// Refuse unless the base document's page-tree `/Kids` is EXACTLY copy 0's flat page list.
///
/// [`PageOrder::ByPosition`] rewrites that array, so anything else in it — an intermediate `/Pages`
/// node, a page the merge does not know about — would be dropped from the document by the rewrite.
/// Every bundled Form 8949 measures flat; this is what makes a future template that does not fail
/// CLOSED rather than emit a filed form with pages missing.
fn flat_page_tree(existing: &[ObjectId], copy0_pages: &[ObjectId]) -> Result<(), FormsError> {
    if existing != copy0_pages {
        return Err(FormsError::Structure(format!(
            "page-tree /Kids is not the flat page list this merge permutes: {} kids, \
             {} pages in copy 0",
            existing.len(),
            copy0_pages.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ B1 — the ragged-copy refusal, seen red on its own removal: delete the `any(|p| p.len() !=
    /// per_copy)` conjunct and the two-then-three case below returns `Ok(2)`, which would transpose
    /// only the first two pages of the second copy and silently drop its third.
    #[test]
    fn a_ragged_copy_set_refuses_instead_of_transposing() {
        let id = |n| (n, 0);
        assert_eq!(
            pages_per_copy(&[vec![id(1), id(2)], vec![id(3), id(4)]]).unwrap(),
            2,
            "two 2-page copies transpose"
        );
        assert!(
            pages_per_copy(&[vec![id(1), id(2)], vec![id(3), id(4), id(5)]]).is_err(),
            "a 2-page copy beside a 3-page one is not a rectangle and must refuse"
        );
        assert!(
            pages_per_copy(&[vec![], vec![]]).is_err(),
            "zero pages per copy is not a page order"
        );
    }

    /// ★ B1 — the flat-page-tree refusal, seen red on its own removal: return `Ok(())`
    /// unconditionally and the third case below passes, which is the document that would lose a page
    /// (or a whole nested subtree) when `/Kids` is rewritten.
    #[test]
    fn a_page_tree_that_is_not_the_flat_page_list_refuses() {
        let id = |n| (n, 0);
        assert!(
            flat_page_tree(&[id(1), id(2)], &[id(1), id(2)]).is_ok(),
            "the measured shape: /Kids IS copy 0's page list"
        );
        assert!(
            flat_page_tree(&[id(9), id(1), id(2)], &[id(1), id(2)]).is_err(),
            "an extra kid (an intermediate /Pages node, say) would be dropped by the rewrite"
        );
        assert!(
            flat_page_tree(&[id(2), id(1)], &[id(1), id(2)]).is_err(),
            "the same pages in another order is not the list this transpose was computed from"
        );
    }
}
