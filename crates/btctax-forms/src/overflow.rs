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

/// Merge already-filled, already-verified single-copy Form 8949 PDFs into one document. Copy 0 is
/// the base; copies 1.. have their root field `/T` renamed (uniquifying every field's fully-qualified
/// name) and their pages + form fields appended.
pub fn merge_copies(copies: &[Vec<u8>]) -> Result<Vec<u8>, FormsError> {
    let mut out = pdf::load(&copies[0])?;
    let pages_root = out.catalog()?.get(b"Pages")?.as_reference()?;
    let out_acro = out.catalog()?.get(b"AcroForm")?.as_reference()?;

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
        // Register this copy's form as another top-level AcroForm field.
        out.get_dictionary_mut(out_acro)?
            .get_mut(b"Fields")?
            .as_array_mut()?
            .push(Object::Reference(root_id));
    }

    // Splice the appended pages into the page tree and fix /Count.
    {
        let pd = out.get_dictionary_mut(pages_root)?;
        let kids = pd.get_mut(b"Kids")?.as_array_mut()?;
        for pid in &appended_pages {
            kids.push(Object::Reference(*pid));
        }
        let count = kids.len() as i64;
        pd.set("Count", Object::Integer(count));
    }

    pdf::strip_nondeterminism(&mut out);
    pdf::save(&mut out)
}
