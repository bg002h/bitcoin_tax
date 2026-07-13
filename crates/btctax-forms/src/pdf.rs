//! The low-level lopdf fill primitive: parse a bundled IRS PDF, walk its AcroForm field tree,
//! drop the `/XFA` layer, set `/V` (+ checkbox `/AS`), pin determinism, and serialize.
//!
//! These forms are **static XFA hybrids**: the `/AcroForm` carries both a live `/XFA` XML layer
//! (which Acrobat/Reader PREFER — a `/V`-only fill opens BLANK) AND a complete classic AcroForm.
//! Removing `/XFA` makes the classic `/V`/`/AS` values authoritative and render everywhere.

use crate::error::FormsError;
use lopdf::{Document, Object, ObjectId, StringFormat};
use std::collections::HashMap;

/// The bundled TY2025 Form 8949 (official IRS fillable PDF, US-gov public domain).
pub const F8949_PDF_2025: &[u8] = include_bytes!("../forms/2025/f8949.pdf");
/// The bundled TY2025 Schedule D (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_D_PDF_2025: &[u8] = include_bytes!("../forms/2025/schedule_d.pdf");
/// The bundled TY2025 Schedule SE (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_SE_PDF_2025: &[u8] = include_bytes!("../forms/2025/schedule_se.pdf");
/// The bundled Form 8283, Rev. 12-2025 (official IRS fillable PDF, US-gov public domain).
pub const F8283_PDF_2025: &[u8] = include_bytes!("../forms/2025/f8283.pdf");
/// The bundled TY2025 Form 1040 (official IRS fillable PDF, US-gov public domain).
pub const F1040_PDF_2025: &[u8] = include_bytes!("../forms/2025/f1040.pdf");

/// The bundled TY2024 Form 8949 (official IRS fillable PDF, US-gov public domain).
pub const F8949_PDF_2024: &[u8] = include_bytes!("../forms/2024/f8949.pdf");
/// The bundled TY2024 Schedule D (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_D_PDF_2024: &[u8] = include_bytes!("../forms/2024/schedule_d.pdf");
/// The bundled TY2024 Schedule SE (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_SE_PDF_2024: &[u8] = include_bytes!("../forms/2024/schedule_se.pdf");
/// The bundled Form 8283, Rev. 12-2023 (TY2024; official IRS fillable PDF, US-gov public domain).
pub const F8283_PDF_2024: &[u8] = include_bytes!("../forms/2024/f8283.pdf");
/// The bundled TY2024 Form 1040 (official IRS fillable PDF, US-gov public domain).
pub const F1040_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040.pdf");
/// The bundled TY2024 Form 8959, Additional Medicare Tax (official IRS fillable PDF, public domain).
pub const F8959_PDF_2024: &[u8] = include_bytes!("../forms/2024/f8959.pdf");
/// The bundled TY2024 Form 8960, Net Investment Income Tax (official IRS fillable PDF, public domain).
pub const F8960_PDF_2024: &[u8] = include_bytes!("../forms/2024/f8960.pdf");
/// The bundled TY2024 Form 8995, QBI deduction — simplified (official IRS fillable PDF, public domain).
pub const F8995_PDF_2024: &[u8] = include_bytes!("../forms/2024/f8995.pdf");
/// The bundled TY2024 Schedule 2, Additional Taxes (official IRS fillable PDF, public domain).
pub const SCHEDULE_2_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040s2.pdf");
/// The bundled TY2024 Schedule 3, Additional Credits and Payments (official IRS fillable PDF, public domain).
pub const SCHEDULE_3_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040s3.pdf");
/// The bundled TY2024 Schedule A, Itemized Deductions (official IRS fillable PDF, public domain).
pub const SCHEDULE_A_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040sa.pdf");
/// The bundled TY2024 Schedule 1, Additional Income and Adjustments (official IRS fillable PDF, public domain).
pub const SCHEDULE_1_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040s1.pdf");
/// The bundled TY2024 Schedule C, Profit or Loss From Business (official IRS fillable PDF, public domain).
pub const SCHEDULE_C_PDF_2024: &[u8] = include_bytes!("../forms/2024/f1040sc.pdf");

/// The bundled TY2017 Form 8949 (official IRS fillable PDF, US-gov public domain).
pub const F8949_PDF_2017: &[u8] = include_bytes!("../forms/2017/f8949.pdf");
/// The bundled TY2017 Schedule D (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_D_PDF_2017: &[u8] = include_bytes!("../forms/2017/schedule_d.pdf");
/// The bundled TY2017 Schedule SE (official IRS fillable PDF, US-gov public domain).
pub const SCHEDULE_SE_PDF_2017: &[u8] = include_bytes!("../forms/2017/schedule_se.pdf");
/// The bundled Form 8283, Rev. 12-2014 (TY2017; official IRS fillable PDF, US-gov public domain).
pub const F8283_PDF_2017: &[u8] = include_bytes!("../forms/2017/f8283.pdf");
/// The bundled TY2017 Form 1040 (official IRS fillable PDF, US-gov public domain).
pub const F1040_PDF_2017: &[u8] = include_bytes!("../forms/2017/f1040.pdf");

/// The bundled Form 8949 PDF bytes for a supported tax year (the asset bound to the year's map).
pub fn f8949_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2017 => Ok(F8949_PDF_2017),
        2024 => Ok(F8949_PDF_2024),
        2025 => Ok(F8949_PDF_2025),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule D PDF bytes for a supported tax year.
pub fn schedule_d_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2017 => Ok(SCHEDULE_D_PDF_2017),
        2024 => Ok(SCHEDULE_D_PDF_2024),
        2025 => Ok(SCHEDULE_D_PDF_2025),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule SE PDF bytes for a supported tax year.
pub fn schedule_se_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2017 => Ok(SCHEDULE_SE_PDF_2017),
        2024 => Ok(SCHEDULE_SE_PDF_2024),
        2025 => Ok(SCHEDULE_SE_PDF_2025),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Form 8959 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn f8959_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(F8959_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Form 8960 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn f8960_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(F8960_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Form 8995 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn f8995_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(F8995_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule 2 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn schedule_2_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(SCHEDULE_2_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule 3 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn schedule_3_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(SCHEDULE_3_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule C PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn schedule_c_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(SCHEDULE_C_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule 1 PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn schedule_1_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(SCHEDULE_1_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Schedule A PDF bytes for a supported tax year. Full-return v1 is TY2024-only.
pub fn schedule_a_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2024 => Ok(SCHEDULE_A_PDF_2024),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Form 8283 PDF bytes for a supported tax year (bound by filing-year → revision).
pub fn f8283_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2017 => Ok(F8283_PDF_2017),
        2024 => Ok(F8283_PDF_2024),
        2025 => Ok(F8283_PDF_2025),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// The bundled Form 1040 PDF bytes for a supported tax year.
pub fn f1040_pdf(year: i32) -> Result<&'static [u8], FormsError> {
    match year {
        2017 => Ok(F1040_PDF_2017),
        2024 => Ok(F1040_PDF_2024),
        2025 => Ok(F1040_PDF_2025),
        _ => Err(FormsError::UnsupportedYear(year)),
    }
}

/// One terminal (leaf) AcroForm field: its object id, fully-qualified name, widget rectangle, and
/// whether it is a checkbox (`/FT /Btn`).
#[derive(Debug, Clone)]
pub struct Field {
    /// lopdf object id of the field dictionary.
    pub id: ObjectId,
    /// Fully-qualified, bracketed name (`topmostSubform[0].Page1[0]…f1_03[0]`).
    pub fqn: String,
    /// Widget rectangle `[x0, y0, x1, y1]` in PDF user space, if present.
    pub rect: Option<[f32; 4]>,
    /// `true` iff `/FT` is `/Btn` (a checkbox/radio).
    pub is_button: bool,
}

impl Field {
    /// Horizontal center of the widget rectangle.
    pub fn cx(&self) -> Option<f32> {
        self.rect.map(|r| (r[0] + r[2]) / 2.0)
    }
    /// Vertical center of the widget rectangle.
    pub fn cy(&self) -> Option<f32> {
        self.rect.map(|r| (r[1] + r[3]) / 2.0)
    }
}

/// What to write into a field.
#[derive(Debug, Clone)]
pub enum FieldValue {
    /// A text value (`/Tx`).
    Text(String),
    /// Turn a checkbox on to the given on-state name (without the leading `/`).
    Check {
        /// The on-state PDF name, e.g. `"6"` for Box I.
        on: String,
    },
}

/// Parse a bundled PDF into a mutable document.
pub fn load(bytes: &[u8]) -> Result<Document, FormsError> {
    Ok(Document::load_mem(bytes)?)
}

fn number(o: &Object) -> Option<f32> {
    match o {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

fn rect_of(dict: &lopdf::Dictionary) -> Option<[f32; 4]> {
    let arr = dict.get(b"Rect").ok()?.as_array().ok()?;
    if arr.len() != 4 {
        return None;
    }
    Some([
        number(&arr[0])?,
        number(&arr[1])?,
        number(&arr[2])?,
        number(&arr[3])?,
    ])
}

/// The AcroForm dictionary's object id (it must be an indirect reference).
fn acroform_id(doc: &Document) -> Result<ObjectId, FormsError> {
    match doc.catalog()?.get(b"AcroForm") {
        Ok(Object::Reference(id)) => Ok(*id),
        Ok(_) => Err(FormsError::Structure(
            "AcroForm is not an indirect reference".into(),
        )),
        Err(_) => Err(FormsError::Structure("catalog has no AcroForm".into())),
    }
}

/// Remove `/XFA` from the AcroForm and set `/NeedAppearances true` (viewers regenerate the visible
/// appearance from `/V`). Must run before saving.
pub fn drop_xfa_and_set_needappearances(doc: &mut Document) -> Result<(), FormsError> {
    let id = acroform_id(doc)?;
    let acro = doc.get_dictionary_mut(id)?;
    acro.remove(b"XFA");
    acro.set("NeedAppearances", Object::Boolean(true));
    Ok(())
}

/// Walk the AcroForm `/Fields` tree and collect every terminal (leaf) field.
pub fn collect_fields(doc: &Document) -> Result<Vec<Field>, FormsError> {
    let acro = doc.get_dictionary(acroform_id(doc)?)?;
    let mut out = Vec::new();
    let fields = acro
        .get(b"Fields")
        .and_then(|o| o.as_array())
        .map_err(|_| FormsError::Structure("AcroForm has no /Fields array".into()))?;
    for f in fields {
        if let Ok(id) = f.as_reference() {
            walk(doc, id, "", None, &mut out)?;
        }
    }
    Ok(out)
}

/// Decode a PDF text string: UTF-16BE if it carries the `FEFF` BOM (Adobe LiveCycle exports field
/// names this way), else PDFDocEncoding (treated as Latin-1, which is exact for the ASCII names).
pub(crate) fn decode_pdf_text(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let units: Vec<u16> = b[2..]
            .chunks(2)
            .map(|c| ((c[0] as u16) << 8) | *c.get(1).unwrap_or(&0) as u16)
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

fn field_component_name(dict: &lopdf::Dictionary) -> Option<String> {
    dict.get(b"T")
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(decode_pdf_text)
}

fn walk(
    doc: &Document,
    id: ObjectId,
    parent_fqn: &str,
    inherited_ft: Option<String>,
    out: &mut Vec<Field>,
) -> Result<(), FormsError> {
    let dict = match doc.get_dictionary(id) {
        Ok(d) => d,
        Err(_) => return Ok(()), // dangling ref — skip
    };
    let name = field_component_name(dict);
    let fqn = match &name {
        Some(t) if parent_fqn.is_empty() => t.clone(),
        Some(t) => format!("{parent_fqn}.{t}"),
        None => parent_fqn.to_string(),
    };
    let ft = dict
        .get(b"FT")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .or(inherited_ft);

    // A branch node carries /Kids of further named fields; a leaf is a terminal field.
    let kids: Option<Vec<ObjectId>> = dict
        .get(b"Kids")
        .ok()
        .and_then(|o| o.as_array().ok())
        .map(|arr| arr.iter().filter_map(|k| k.as_reference().ok()).collect());
    match kids {
        Some(kids) if !kids.is_empty() => {
            for k in kids {
                walk(doc, k, &fqn, ft.clone(), out)?;
            }
        }
        _ => {
            out.push(Field {
                id,
                fqn,
                rect: rect_of(dict),
                is_button: ft.as_deref() == Some("Btn"),
            });
        }
    }
    Ok(())
}

/// Index the collected leaf fields by fully-qualified name.
pub fn index(fields: &[Field]) -> HashMap<String, Field> {
    fields.iter().map(|f| (f.fqn.clone(), f.clone())).collect()
}

/// Apply a batch of writes. Errors (fails closed) if any field name is absent from the PDF.
pub fn apply_writes(
    doc: &mut Document,
    index: &HashMap<String, Field>,
    writes: &[(String, FieldValue)],
) -> Result<(), FormsError> {
    for (fqn, value) in writes {
        let field = index
            .get(fqn)
            .ok_or_else(|| FormsError::MapFieldMissing(fqn.clone()))?;
        let dict = doc.get_dictionary_mut(field.id)?;
        match value {
            FieldValue::Text(s) => {
                dict.set(
                    "V",
                    Object::String(s.clone().into_bytes(), StringFormat::Literal),
                );
            }
            FieldValue::Check { on } => {
                dict.set("V", Object::Name(on.clone().into_bytes()));
                dict.set("AS", Object::Name(on.clone().into_bytes()));
            }
        }
    }
    Ok(())
}

/// Strip clock/RNG-derived bytes so `(data, form) → byte-identical` output: drop the document
/// `/Info` timestamps and the trailer `/ID`. No other source of nondeterminism exists (lopdf writes
/// objects in stable id order; no float structure; miniz_oxide deflate is deterministic).
pub fn strip_nondeterminism(doc: &mut Document) {
    if let Ok(info) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) {
        if let Ok(d) = doc.get_dictionary_mut(info) {
            d.remove(b"CreationDate");
            d.remove(b"ModDate");
        }
    }
    doc.trailer.remove(b"ID");
}

/// Serialize the document to bytes.
pub fn save(doc: &mut Document) -> Result<Vec<u8>, FormsError> {
    let mut buf = Vec::new();
    doc.save_to(&mut buf)?;
    Ok(buf)
}

/// Read back a leaf field's `/V` as a string (text value) — used by tests and the no-unmapped scan.
pub fn text_value(doc: &Document, id: ObjectId) -> Option<String> {
    let v = doc.get_dictionary(id).ok()?.get(b"V").ok()?;
    match v {
        Object::String(b, _) => Some(decode_pdf_text(b)),
        _ => None,
    }
}

/// Read back a checkbox's `/AS` on-state (None if `/Off` or absent).
pub fn checkbox_on(doc: &Document, id: ObjectId) -> Option<String> {
    let as_ = doc.get_dictionary(id).ok()?.get(b"AS").ok()?;
    match as_ {
        Object::Name(b) if b != b"Off" => Some(String::from_utf8_lossy(b).into_owned()),
        _ => None,
    }
}

/// The possible ON-state name(s) of a button widget — the `/AP` `/N` appearance keys other than
/// `/Off` (without the leading `/`). Read straight from the bundled PDF, so the SP2 same-y `/Btn`
/// pair oracle (the 1040 Digital-Asset Yes/No question) is map-INDEPENDENT. Empty when the widget
/// carries no `/AP`/`/N`.
pub fn button_on_states(doc: &Document, id: ObjectId) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(dict) = doc.get_dictionary(id) else {
        return out;
    };
    let ap = match dict.get(b"AP") {
        Ok(Object::Reference(r)) => doc.get_dictionary(*r).ok(),
        Ok(Object::Dictionary(d)) => Some(d),
        _ => None,
    };
    let n = ap.and_then(|ap| match ap.get(b"N") {
        Ok(Object::Reference(r)) => doc.get_dictionary(*r).ok(),
        Ok(Object::Dictionary(d)) => Some(d),
        _ => None,
    });
    if let Some(n) = n {
        for (k, _) in n.iter() {
            if k != b"Off" {
                out.push(String::from_utf8_lossy(k).into_owned());
            }
        }
    }
    out.sort();
    out
}
