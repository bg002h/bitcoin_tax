//! Page watermarks — plain content-stream overlays carrying their OWN embedded standard font
//! resource, so they are orthogonal to the field `/NeedAppearances` regeneration and cannot be
//! confused for real filer data.
//!
//! Two independent stamps, on **different diagonals** so a page can legibly carry both:
//!
//! | stamp | says | applied when |
//! |---|---|---|
//! | [`stamp_draft`] | `DRAFT — ESTIMATE, NOT FOR FILING` | the ledger is pseudo-reconciled (a fictional draft) |
//! | [`stamp_partial_worksheet`] | `WORKSHEET — NOT A COMPLETE FORM 1040` | the crypto-slice Form 1040, which fills **two cells** and leaves every income/deduction/tax line blank |
//!
//! **★ Why the second one exists.** `form_1040_capgains.pdf` renders as a Form 1040 — masthead, a
//! populated line 7a, and a blank line 1a. Its only caveat was a note on **stderr**, and the document
//! outlives the terminal it was printed in. A filer who mails it files a return that omits their
//! wages. The disclosure has to be ON the page, because the page is what travels.

use crate::error::FormsError;
use crate::pdf;
use lopdf::{Dictionary, Object, ObjectId, Stream};

const FONT_NAME: &[u8] = b"BtctaxWm";

/// The DRAFT watermark text. The dash is WinAnsi 0x97 (em dash) — hence `/Encoding /WinAnsiEncoding`.
fn watermark_text() -> Vec<u8> {
    let mut s = b"DRAFT ".to_vec();
    s.push(0x97); // em dash
    s.extend_from_slice(b" ESTIMATE, NOT FOR FILING");
    s
}

/// The partial-worksheet text, for a form btctax fills only part of.
fn worksheet_text() -> Vec<u8> {
    let mut s = b"WORKSHEET ".to_vec();
    s.push(0x97); // em dash
    s.extend_from_slice(b" NOT A COMPLETE FORM 1040");
    s
}

/// The text matrix for the DRAFT stamp: 45° up-and-right from the lower-left corner.
const TM_DRAFT: &[u8] = b"0.7071 0.7071 -0.7071 0.7071 90 250 Tm\n";
/// The text matrix for the worksheet stamp: 45° **down**-and-right from the upper-left corner — the
/// opposite diagonal, so a pseudo-reconciled crypto-slice 1040 carrying BOTH reads as an X rather
/// than as two overprinted strings.
const TM_WORKSHEET: &[u8] = b"0.7071 -0.7071 0.7071 0.7071 60 700 Tm\n";

/// Stamp the DRAFT watermark on every page of `pdf_bytes`.
pub fn stamp_draft(pdf_bytes: &[u8]) -> Result<Vec<u8>, FormsError> {
    stamp(pdf_bytes, &watermark_text(), TM_DRAFT)
}

/// Stamp the PARTIAL-WORKSHEET watermark on every page of `pdf_bytes`.
///
/// Composes with [`stamp_draft`] in either order (each appends its own content stream and its own
/// font entry under the same name, which resolves to the same standard-14 Helvetica).
pub fn stamp_partial_worksheet(pdf_bytes: &[u8]) -> Result<Vec<u8>, FormsError> {
    stamp(pdf_bytes, &worksheet_text(), TM_WORKSHEET)
}

/// Stamp `text` on every page of `pdf_bytes` along the diagonal given by `tm`.
fn stamp(pdf_bytes: &[u8], text: &[u8], tm: &[u8]) -> Result<Vec<u8>, FormsError> {
    let mut doc = pdf::load(pdf_bytes)?;

    // A single shared Helvetica (standard-14) font, WinAnsi-encoded.
    let mut font = Dictionary::new();
    font.set("Type", Object::Name(b"Font".to_vec()));
    font.set("Subtype", Object::Name(b"Type1".to_vec()));
    font.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    font.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let font_id = doc.add_object(Object::Dictionary(font));

    let page_ids: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for pid in page_ids {
        // Give the page its own resources dict (cloned so existing fonts survive) + our font.
        let res = resources_with_font(&doc, pid, font_id)?;
        doc.get_dictionary_mut(pid)?
            .set("Resources", Object::Dictionary(res));

        // A self-contained overlay: light gray, 30pt, rotated ~45° along `tm` so the diagonal
        // crosses the whole page.
        let mut content = b"q\n0.78 0.78 0.78 rg\nBT\n/".to_vec();
        content.extend_from_slice(FONT_NAME);
        content.extend_from_slice(b" 30 Tf\n");
        content.extend_from_slice(tm);
        content.push(b'(');
        content.extend_from_slice(&escape_pdf_string(text));
        content.extend_from_slice(b") Tj\nET\nQ\n");
        let stream_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), content)));

        append_content(&mut doc, pid, stream_id)?;
    }

    pdf::strip_nondeterminism(&mut doc);
    pdf::save(&mut doc)
}

/// Escape `(`, `)`, `\` for a PDF literal string.
fn escape_pdf_string(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    for &b in s {
        if b == b'(' || b == b')' || b == b'\\' {
            out.push(b'\\');
        }
        out.push(b);
    }
    out
}

/// Build an owned resources dictionary for a page with the watermark font added, preserving any
/// existing fonts (cloned inline so the page keeps rendering its field appearances).
fn resources_with_font(
    doc: &lopdf::Document,
    page_id: ObjectId,
    font_id: ObjectId,
) -> Result<Dictionary, FormsError> {
    let mut res = match doc.get_dictionary(page_id)?.get(b"Resources") {
        Ok(Object::Reference(rid)) => doc.get_dictionary(*rid)?.clone(),
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    let mut fonts = match res.get(b"Font") {
        Ok(Object::Reference(rid)) => doc.get_dictionary(*rid)?.clone(),
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    fonts.set(FONT_NAME.to_vec(), Object::Reference(font_id));
    res.set("Font", Object::Dictionary(fonts));
    Ok(res)
}

/// Append an overlay content stream so it draws ON TOP of the existing page content.
fn append_content(
    doc: &mut lopdf::Document,
    page_id: ObjectId,
    stream_id: ObjectId,
) -> Result<(), FormsError> {
    let existing = doc.get_dictionary(page_id)?.get(b"Contents").ok().cloned();
    let new_contents = match existing {
        Some(Object::Array(mut a)) => {
            a.push(Object::Reference(stream_id));
            Object::Array(a)
        }
        Some(Object::Reference(r)) => {
            Object::Array(vec![Object::Reference(r), Object::Reference(stream_id)])
        }
        _ => Object::Array(vec![Object::Reference(stream_id)]),
    };
    doc.get_dictionary_mut(page_id)?
        .set("Contents", new_contents);
    Ok(())
}
