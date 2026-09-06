//! Error type for the PDF form-fill engine.

/// Anything that can go wrong filling an official IRS PDF.
///
/// The **geometric read-back** failures (`Geometry`, `UnmappedField`) are the tax-safety net: a fill
/// that lands a value in the wrong cell — or writes any field the map did not authorize — FAILS
/// CLOSED (no PDF bytes are returned), so a mis-mapped form is never handed to a filer.
#[derive(Debug, thiserror::Error)]
pub enum FormsError {
    /// The requested tax year has no bundled form set / map. The years named in the message are
    /// DERIVED from the glob (`bundled::BUNDLED_YEARS`), never a literal — the literal this used to
    /// carry ("2017, 2024 and 2025 only") is the stale-string class design r2 retires.
    #[error(
        "unsupported tax year {0}: this build bundles IRS forms for {} only",
        crate::bundled::years_sentence()
    )]
    UnsupportedYear(i32),

    /// The year's map is bundled and listed, but its `line_set` revision has not been VERIFIED
    /// against a struct yet (design r2 §10 step 3 → step 5; most would parse — see `line_set.rs`).
    /// Distinct from `UnsupportedYear`: the file is THERE; the verification is not.
    #[error("{stem} for tax year {year} is bundled but not wired: its line-set revision {line_set:?} has not been verified against a transcription struct yet (design r2 §10 step 5)")]
    UnwiredLineSet {
        stem: &'static str,
        year: i32,
        line_set: &'static str,
    },

    /// A field named by the map does not exist in the bundled PDF's AcroForm.
    #[error("map references field {0:?} which is absent from the bundled PDF field set")]
    MapFieldMissing(String),

    /// More data rows than the form's page grid can hold on the paths that do not paginate.
    #[error("{rows} rows exceed the {capacity}-row capacity of a single {part} page")]
    Overflow {
        /// The part being filled ("Part I" / "Part II").
        part: &'static str,
        /// The number of rows requested.
        rows: usize,
        /// The per-page row capacity from the map.
        capacity: usize,
    },

    /// The geometric read-back found a written value in the WRONG column/row band — the map is
    /// mis-aligned. Fails closed.
    #[error("geometric read-back FAILED (mis-mapped cell): {0}")]
    Geometry(String),

    /// A field carries a value but the map never authorized writing it (a stray write). Fails closed.
    #[error("read-back FAILED: unmapped field {0:?} was filled")]
    UnmappedField(String),

    /// A written value is longer than the cell's `/MaxLen` — the form declares a fixed-width (usually
    /// **comb**) cell and the value does not fit. A PDF viewer would silently truncate it, or splay it
    /// across the wrong comb teeth, so the fill fails closed instead: a truncated SSN on a filed return
    /// is a wrong return.
    #[error(
        "read-back FAILED: {fqn:?} holds {len} characters but the cell's /MaxLen is {max_len}"
    )]
    CellOverflow {
        /// The over-filled field.
        fqn: String,
        /// The cell's declared capacity.
        max_len: usize,
        /// The length actually written.
        len: usize,
    },

    /// The bundled PDF's structure was not what the engine expects (missing AcroForm, bad Rect, …).
    #[error("bundled PDF structure error: {0}")]
    Structure(String),

    /// Underlying lopdf parse/serialize error.
    #[error("pdf error: {0}")]
    Pdf(#[from] lopdf::Error),

    /// A committed TOML map failed to parse.
    #[error("map parse error: {0}")]
    Map(#[from] toml::de::Error),

    /// A caller-supplied value the form line cannot carry, refused BEFORE any byte is produced.
    ///
    /// The two live cases are both filer choices on a money line whose siblings are whole dollars:
    /// Form 4868 line 7 (`--pay`) and Form 1040-V box 3 (`--pay`). The form's own rounding rule is
    /// all-or-nothing — *"You can round off cents to whole dollars on Form 4868. If you do round to
    /// whole dollars, you must round all amounts."* — so a payment carrying cents beside four
    /// whole-dollar lines is a
    /// form contradicting itself, and rounding it on the filer's behalf would put a number they did not
    /// choose on a signed application. Refuse instead.
    ///
    /// Both are ALSO refused by the command that collects them (`btctax extension`, `export-irs-pdf
    /// --pay-by-check`), with a message naming the flag. This variant is the second, structural gate:
    /// the filler is reachable from a caller that never saw the flag.
    #[error("Form {form} line {line}: {detail}")]
    InvalidValue {
        /// The form, as a filer names it (`"4868"`, `"1040-V"`).
        form: &'static str,
        /// The printed line/box number the value was destined for.
        line: &'static str,
        /// What is wrong with it, and what would be accepted.
        detail: String,
    },

    /// I/O error serializing the PDF.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
