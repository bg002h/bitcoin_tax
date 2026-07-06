//! Error type for the PDF form-fill engine.

/// Anything that can go wrong filling an official IRS PDF.
///
/// The **geometric read-back** failures (`Geometry`, `UnmappedField`) are the tax-safety net: a fill
/// that lands a value in the wrong cell — or writes any field the map did not authorize — FAILS
/// CLOSED (no PDF bytes are returned), so a mis-mapped form is never handed to a filer.
#[derive(Debug, thiserror::Error)]
pub enum FormsError {
    /// The requested tax year has no bundled form set / map (SP1 ships TY2025 only).
    #[error("unsupported tax year {0}: this build bundles IRS forms for 2025 only")]
    UnsupportedYear(i32),

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

    /// The bundled PDF's structure was not what the engine expects (missing AcroForm, bad Rect, …).
    #[error("bundled PDF structure error: {0}")]
    Structure(String),

    /// Underlying lopdf parse/serialize error.
    #[error("pdf error: {0}")]
    Pdf(#[from] lopdf::Error),

    /// A committed TOML map failed to parse.
    #[error("map parse error: {0}")]
    Map(#[from] toml::de::Error),

    /// I/O error serializing the PDF.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
