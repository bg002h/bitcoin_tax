//! ⑤ step 1 — **the committed GEOMETRY observation** that both label witnesses read.
//!
//! ★★ **Why a fixture and not the PDF.** The form PDFs are gitignored (see `design/forms/README.md`),
//! so CI has none. Tests must therefore read a *committed* observation — the same reason the text
//! layer lives in `design/forms/extract/`.
//!
//! ★★★ **What it may and may not contain, and this is load-bearing.** This fixture holds the RAW
//! OBSERVATION — every word with its coordinates, every AcroForm box with its coordinates — and
//! **never the reader's conclusions.** `design/forms/LABEL_READER.md` names the trap directly:
//!
//! > pin an observation **of the form** (which reds when the form changes), never the **reader's own
//! > output** (which would assert only that the reader still does what it did).
//!
//! So the chain is non-circular by construction:
//!
//! | artifact | what it is | what it is checked against |
//! |---|---|---|
//! | this fixture | an observation of the form | the PDF's sha256, pinned in its header |
//! | the ledger | the adjudicated label truth | the two witnesses, re-derived every run |
//! | the witnesses | derivations under test | the ledger |
//!
//! ★ **Two coordinate systems, kept raw rather than pre-reconciled.** `pdftotext -bbox` is top-down
//! (y grows downward from the page top); AcroForm boxes are PDF-native bottom-up. Both are stored as
//! measured, with the page height, so the flip is an explicit step someone can check — not a silent
//! adjustment baked into the data.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One word from the text layer, in `pdftotext -bbox` coordinates (TOP-DOWN).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Word {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
    #[serde(rename = "t")]
    pub text: String,
}

/// One AcroForm field, in PDF-native coordinates (BOTTOM-UP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Box_ {
    pub page: u32,
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub n: u32,
    pub width: f64,
    pub height: f64,
}

/// The committed observation of one form-year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geometry {
    /// `f1040s1a--2025`.
    pub form: String,
    /// sha256 of the PDF this was read from. ★ A changed hash means the IRS REVISED the form —
    /// review it, never regenerate silently.
    pub pdf_sha256: String,
    pub pages: Vec<Page>,
    pub words: Vec<Word>,
    pub boxes: Vec<Box_>,
}

impl Geometry {
    pub fn page(&self, n: u32) -> Option<&Page> {
        self.pages.iter().find(|p| p.n == n)
    }

    /// A box's y in TOP-DOWN coordinates, so it can be compared with [`Word`] positions.
    ///
    /// ★ The flip is here, once, named — rather than pre-applied in the fixture where nobody could
    /// see it. `top_down = page_height - pdf_y`.
    pub fn box_top_down_y(&self, b: &Box_) -> Option<(f64, f64)> {
        let h = self.page(b.page)?.height;
        Some((h - b.y2, h - b.y)) // y2 is the higher PDF y, i.e. the SMALLER top-down y
    }
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask has a grandparent")
        .to_path_buf()
}

pub fn geometry_path(root: &Path, stem: &str) -> PathBuf {
    root.join(format!("design/forms/geometry/{stem}.json"))
}

pub fn load(root: &Path, stem: &str) -> Result<Geometry, String> {
    let p = geometry_path(root, stem);
    let text = std::fs::read_to_string(&p)
        .map_err(|e| format!("geometry fixture missing: {} ({e})", p.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{} is not valid geometry: {e}", p.display()))
}

// ─────────────────────────────── extraction (needs the PDF) ───────────────────────────────

/// Parse `pdftotext -bbox` XHTML. Deliberately hand-parsed: the grammar we consume is three tag
/// shapes, and adding an XML dependency to read them would be more surface than the job needs.
fn parse_bbox(xml: &str) -> (Vec<Page>, Vec<Word>) {
    let (mut pages, mut words) = (Vec::new(), Vec::new());
    let mut page_n = 0u32;
    for line in xml.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("<page width=\"") {
            page_n += 1;
            let w = rest
                .split('"')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let h = rest
                .split("height=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            pages.push(Page {
                n: page_n,
                width: w,
                height: h,
            });
            continue;
        }
        if !t.starts_with("<word ") {
            continue;
        }
        let attr = |k: &str| -> Option<f64> {
            t.split(&format!("{k}=\""))
                .nth(1)?
                .split('"')
                .next()?
                .parse()
                .ok()
        };
        let text = t
            .split('>')
            .nth(1)
            .and_then(|s| s.split('<').next())
            .unwrap_or("");
        if text.is_empty() {
            continue;
        }
        if let (Some(x), Some(y), Some(x2), Some(y2)) =
            (attr("xMin"), attr("yMin"), attr("xMax"), attr("yMax"))
        {
            words.push(Word {
                page: page_n,
                x,
                y,
                x2,
                y2,
                text: html_unescape(text),
            });
        }
    }
    (pages, words)
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

/// `cargo run -p xtask -- extract-geometry <stem>` — e.g. `f1040s1a--2025`.
///
/// ★ Requires the PDF locally (gitignored, re-fetchable from the URL in its `.pdf.txt` note). The
/// committed JSON is what tests read, so neither CI nor a fresh clone needs `pdftotext` or network.
pub fn extract(stem: &str) -> Result<(), String> {
    let root = repo_root();
    let year = stem.rsplit("--").next().unwrap_or("2025");
    let pdf = root.join(format!("design/forms/{year}/{stem}.pdf"));
    if !pdf.is_file() {
        return Err(format!(
            "{} not present. It is gitignored; re-fetch it from the URL in {}.txt",
            pdf.display(),
            pdf.display()
        ));
    }

    let out = std::process::Command::new("pdftotext")
        .args(["-bbox", pdf.to_str().unwrap_or_default(), "-"])
        .output()
        .map_err(|e| format!("pdftotext failed (is poppler installed?): {e}"))?;
    if !out.status.success() {
        return Err(format!("pdftotext exited {:?}", out.status.code()));
    }
    let (pages, words) = parse_bbox(&String::from_utf8_lossy(&out.stdout));
    if words.is_empty() {
        return Err(
            "pdftotext -bbox produced no words — refusing to write an empty observation".into(),
        );
    }

    // ★ Reuse the SHIPPED AcroForm reader (`btctax-forms::testonly`), the same one the emitter
    // fills through and `dump-fields` prints. A second parser's view of the boxes would be a second
    // truth, and the whole point of this design is that the witnesses observe the same form.
    let bytes = std::fs::read(&pdf).map_err(|e| format!("read {}: {e}", pdf.display()))?;
    let doc = btctax_forms::testonly::load(&bytes)
        .map_err(|e| format!("parse {}: {e}", pdf.display()))?;
    let fields = btctax_forms::testonly::collect_fields(&doc)
        .map_err(|e| format!("reading AcroForm fields: {e}"))?;

    // The page is not carried on `Field`; the IRS templates always nest widgets under a `PageN[0]`
    // subform, so the FQN is the page. Same derivation `dump-fields` uses — one rule, not two.
    let page_of = |fqn: &str| -> u32 {
        fqn.split('.')
            .find_map(|seg| seg.strip_prefix("Page")?.split('[').next()?.parse().ok())
            .unwrap_or(0)
    };

    // ★ A field with no widget rect has no position, so it cannot participate in a geometric join.
    // It is DROPPED here and counted, never silently absorbed — an uncounted drop is how a witness
    // goes quietly blind.
    let mut no_rect = 0usize;
    let boxes: Vec<Box_> = fields
        .iter()
        .filter_map(|f| {
            let Some(r) = f.rect else {
                no_rect += 1;
                return None;
            };
            Some(Box_ {
                page: page_of(&f.fqn),
                x: r[0] as f64,
                y: r[1] as f64,
                x2: r[2] as f64,
                y2: r[3] as f64,
                name: f.fqn.clone(),
            })
        })
        .collect();
    if no_rect > 0 {
        println!(
            "extract-geometry: NOTE — {no_rect} field(s) have no widget rect and were dropped"
        );
    }

    let (sha256, _) = crate::authority_manifest::sha256_of(&pdf)
        .map_err(|e| format!("hashing {}: {e}", pdf.display()))?;

    let g = Geometry {
        form: stem.to_string(),
        pdf_sha256: sha256,
        pages,
        words,
        boxes,
    };
    let path = geometry_path(&root, stem);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("mkdir {}: {e}", d.display()))?;
    }
    std::fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string(&g).map_err(|e| format!("serialising: {e}"))?
        ),
    )
    .map_err(|e| format!("writing {}: {e}", path.display()))?;

    println!(
        "extract-geometry: {} — {} words, {} boxes, {} pages (sha256:{}…)",
        path.display(),
        g.words.len(),
        g.boxes.len(),
        g.pages.len(),
        &g.pdf_sha256[..8]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ The bbox parser on a hand-built sample of the real grammar. Without this, a parser that
    /// silently returned nothing would make every downstream witness "find no labels" and the census
    /// would pass by having nothing to check — the exact vacuous-pass trap.
    #[test]
    fn the_bbox_parser_reads_words_and_pages() {
        let xml = r#"<html>
<body>
<doc>
  <page width="612.000000" height="792.000000">
    <word xMin="45.396000" yMin="120.649000" xMax="50.400000" yMax="131.386000">1</word>
    <word xMin="50.000000" yMin="144.000000" xMax="55.000000" yMax="155.000000">b</word>
    <word xMin="36.000000" yMin="33.000000" xMax="80.000000" yMax="45.000000">SCHEDULE</word>
  </page>
</doc>
</body>
</html>"#;
        let (pages, words) = parse_bbox(xml);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].height, 792.0);
        assert_eq!(words.len(), 3, "all three words must parse");
        assert_eq!(words[0].text, "1");
        assert_eq!(words[0].x, 45.396);
        assert_eq!(words[1].text, "b");
        assert_eq!(words[2].page, 1);
    }

    /// ★★ The y-flip, which is the join between the two witnesses. Getting it backwards would
    /// silently pair every box with the wrong row — a defect that produces a *plausible* ledger, the
    /// worst kind.
    #[test]
    fn the_y_flip_converts_pdf_coordinates_to_top_down() {
        let g = Geometry {
            form: "t".into(),
            pdf_sha256: String::new(),
            pages: vec![Page {
                n: 1,
                width: 612.0,
                height: 792.0,
            }],
            words: vec![],
            boxes: vec![Box_ {
                page: 1,
                x: 504.0,
                y: 684.0,
                x2: 576.0,
                y2: 698.0,
                name: "f1_01".into(),
            }],
        };
        let (top, bottom) = g.box_top_down_y(&g.boxes[0]).expect("page exists");
        // A box near the TOP of the page in PDF coords (y=684..698 of 792) must come out with a
        // SMALL top-down y.
        assert!((top - 94.0).abs() < 0.01, "top was {top}");
        assert!((bottom - 108.0).abs() < 0.01, "bottom was {bottom}");
        assert!(top < bottom, "top-down y must increase downward");
    }
}
