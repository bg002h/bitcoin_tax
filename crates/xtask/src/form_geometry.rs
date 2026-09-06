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
    ///
    /// ★★★ **Enforced by [`pdf_sha_tests::every_committed_geometry_fixture_matches_the_manifest`],
    /// and until 2026-09-05 it was enforced by nothing.** This field was written at generation time
    /// and compared to `design/forms/MANIFEST.json` by no test, no command and no hook — the
    /// guarantee above was a sentence in a doc comment. Drafts make that concrete rather than
    /// theoretical: the IRS replaces a draft IN PLACE under the same URL, so a revision leaves the
    /// filename, the `.pdf.txt` note and this fixture all looking exactly as they did, and the only
    /// witness that the observation is still an observation *of the manifest's document* is this
    /// hash being read by something.
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

/// The REPO-RELATIVE path of the PDF a fixture stem was read from — and therefore the key its
/// `MANIFEST.json` entry is filed under: `f6251--2026-DRAFT` →
/// `design/forms/2026/f6251--2026-DRAFT.pdf`.
///
/// ★★ **A DRAFT stem is `<form>--<year>-DRAFT`, and the year DIRECTORY is the YEAR, not
/// `2026-DRAFT`.** The `-DRAFT` marker stays in the FILENAME on purpose — it is one of the three
/// signals [`crate::authority_manifest::Entry::is_draft`] reads, and R20 is precisely that a draft
/// under a clean stem is indistinguishable from a final. So the suffix is stripped when resolving
/// the DIRECTORY and kept everywhere else.
///
/// ★ **One rule, one place.** The year comes from [`crate::label_reader::stem_year`], the same
/// function `label-proof` resolves its PDF with, so a fixture, its PDF and its manifest entry cannot
/// be resolved three different ways. Until 2026-09-05 `extract` carried its own copy ending in
/// `.unwrap_or("2025")` — a fallback that could never fire (`rsplit` always yields at least one
/// item, so `"f1040".rsplit("--").next()` is `Some("f1040")`) while reading as a deliberate
/// fallback-to-2025 policy. That is `TY2026_PORT_REPORT.md` #4, and this was its second site.
pub fn pdf_rel_for_stem(stem: &str) -> Result<String, String> {
    let year = crate::label_reader::stem_year(stem)?;
    Ok(format!("design/forms/{year}/{stem}.pdf"))
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
    let pdf = root.join(pdf_rel_for_stem(stem)?);
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
    let (mut pages, mut words) = parse_bbox(&String::from_utf8_lossy(&out.stdout));

    // ★★★ **DROP AN IRS DRAFT COVER SHEET, and renumber — otherwise words and boxes disagree by a
    //     page and every label join on a draft is silently wrong.**
    //
    //     Every IRS draft is served with a cover sheet reading "Caution: DRAFT—NOT FOR FILING"; the
    //     form itself begins on page 2. Box pages here come from the FQN (`Page1[0]`), which still
    //     says 1, while word pages come from pdftotext's PHYSICAL page, which says 2. The join then
    //     matches page-1 boxes against page-1 words — the cover sheet — and produces labels that
    //     are not merely wrong but plausible.
    //
    //     ★ Measured: every one of the 16 archived TY2026 drafts has exactly one more page than its
    //       final (f6251 3 vs 2, f1040 3 vs 2, f8960 2 vs 1) with an IDENTICAL box count. It voided
    //       the whole "lines that moved" column of a work list built on top of it, and the
    //       calibration tests did not catch it because they compare two FINALS.
    //
    //     ★★ Detected from the page's own TEXT, not the filename. A filename is a convention; the
    //        cover sheet is the document telling us what it is.
    let cover = pages.first().is_some_and(|p| {
        let t: String = words
            .iter()
            .filter(|w| w.page == p.n)
            .map(|w| w.text.to_uppercase())
            .collect::<Vec<_>>()
            .join(" ");
        t.contains("DRAFT") && t.contains("NOT FOR FILING")
    });
    if cover {
        let first = pages[0].n;
        words.retain(|w| w.page != first);
        for w in &mut words {
            w.page -= 1;
        }
        pages.remove(0);
        for p in &mut pages {
            p.n -= 1;
        }
        eprintln!("extract-geometry: dropped the DRAFT cover sheet and renumbered pages");
    }

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

#[cfg(test)]
mod cover_sheet_tests {
    use super::*;

    /// ★★★ **No committed geometry fixture may still contain a DRAFT COVER SHEET.**
    ///
    /// The cover sheet is not part of the form. Box pages come from the AcroForm FQN (`Page1[0]`),
    /// word pages from pdftotext's PHYSICAL page — so a surviving cover sheet shifts one and not the
    /// other, and every line→label join on that form is off by a page. It does not produce garbage:
    /// it produces PLAUSIBLE labels, which is why it went unnoticed.
    ///
    /// ★ Measured cost when it did: `design/TY2026_WORK_LIST.md` reported Form 1040 moving **31**
    /// printed line bindings and Form 6251 **28**. With the cover sheet dropped the true figures are
    /// **0** and **1**. A whole column of a committed document, and its headline, were an artifact.
    /// The calibration tests missed it because they compare two FINALS, neither of which has a cover.
    #[test]
    fn no_committed_geometry_fixture_contains_a_draft_cover_sheet() {
        let dir = repo_root().join("design/forms/geometry");
        let mut checked = 0usize;
        let mut offenders: Vec<String> = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .expect("geometry fixture directory exists")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        entries.sort();
        for path in entries {
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            // ★★ A fixture that will not READ or PARSE is a FAILURE, not a skip. The first draft
            //    of this guard `continue`d on both, so a corrupt fixture passed — and the planted
            //    defect that was supposed to prove the guard worked was silently skipped instead of
            //    caught. A checker that treats "cannot inspect" as "fine" is the exact shape it
            //    exists to prevent.
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{stem}: geometry fixture unreadable: {e}"));
            let g: Geometry = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{stem}: geometry fixture does not parse: {e}"));
            checked += 1;
            let Some(first) = g.pages.first() else {
                continue;
            };
            let joined: String = g
                .words
                .iter()
                .filter(|w| w.page == first.n)
                .map(|w| w.text.to_uppercase())
                .collect::<Vec<_>>()
                .join(" ");
            if joined.contains("DRAFT") && joined.contains("NOT FOR FILING") {
                offenders.push(stem);
            }
        }
        assert!(
            checked > 30,
            "only {checked} fixtures inspected — the walk is not reaching design/forms/geometry"
        );
        assert!(
            offenders.is_empty(),
            "these fixtures still carry the draft cover sheet as page 1, so their box pages (from \
             the FQN) and word pages (physical) are off by one and every label join on them is \
             wrong — regenerate with `xtask extract-geometry <stem>`: {offenders:?}"
        );
    }
}

// ────────────── the fixture is an observation OF the manifest's document ──────────────

#[cfg(test)]
mod pdf_sha_tests {
    //! ★★★ **[`Geometry::pdf_sha256`] is a pin, and a pin nobody reads is a decoration.**
    //!
    //! Every committed fixture claims, in its own header, the sha256 of the PDF it was read from.
    //! `MANIFEST.json` independently claims the sha256 of that same PDF (as a `note` — the binaries
    //! are gitignored, so the manifest entry is all CI has). Nothing compared the two, so a fixture
    //! could go on describing a document the repo no longer points at, and every witness reading it
    //! — the label census, `label-proof`, the Schedule 1-A conformance KAT — would keep passing
    //! against last year's geometry.
    //!
    //! ★ Why it bites hardest on drafts: `f1040s1a--2026-DRAFT` and friends are *replaced in place*
    //! at the same URL, and the TY2026 Schedule 1-A draft was revised as recently as 2026-09-04. A
    //! re-fetch updates the manifest note; the fixture is regenerated only if someone remembers.
    //! This test is the "someone remembers".
    //!
    //! ★★ **No hand-list anywhere.** The fixture set is the `*.json` files under
    //! `design/forms/geometry/`; the expected hash is the manifest entry filed under the fixture's
    //! own PDF path, resolved by [`super::pdf_rel_for_stem`] — the single rule `extract` uses to
    //! WRITE the fixture. A fixture whose PDF has no manifest entry is a FINDING reported by name,
    //! never a skip.

    use super::*;
    use crate::authority_manifest::Entry;

    /// One reason a committed fixture is not a trustworthy observation of the manifest's document.
    #[derive(Debug, PartialEq, Eq)]
    enum Finding {
        /// The filename is not `<form>--<year>[-DRAFT]`, so no PDF path can be derived from it.
        UnusableStem { stem: String, why: String },
        /// The fixture's own `form` field disagrees with its filename — the two identities used to
        /// look it up are not the same identity.
        NameDisagrees { stem: String, recorded: String },
        /// No manifest entry covers the PDF this fixture claims to observe.
        NoManifestEntry { stem: String, pdf: String },
        /// Several manifest entries claim the same path, so "the" expected hash is not defined.
        Ambiguous {
            stem: String,
            pdf: String,
            shas: Vec<String>,
        },
        /// The pin and the manifest disagree: the form was revised, or the fixture was hand-edited.
        Drifted {
            stem: String,
            pdf: String,
            fixture: String,
            manifest: String,
        },
    }

    impl Finding {
        fn describe(&self) -> String {
            match self {
                Finding::UnusableStem { stem, why } => {
                    format!("{stem}: cannot resolve the PDF it observes — {why}")
                }
                Finding::NameDisagrees { stem, recorded } => format!(
                    "{stem}: the fixture's own `form` field says `{recorded}` — the filename and \
                     the recorded identity are not the same document"
                ),
                Finding::NoManifestEntry { stem, pdf } => format!(
                    "{stem}: no MANIFEST.json entry for `{pdf}` — this fixture observes a document \
                     the repo does not track, so nothing can say whether it is current"
                ),
                Finding::Ambiguous { stem, pdf, shas } => format!(
                    "{stem}: {} manifest entries claim `{pdf}` ({shas:?}) — the expected hash is \
                     not defined",
                    shas.len()
                ),
                Finding::Drifted {
                    stem,
                    pdf,
                    fixture,
                    manifest,
                } => format!(
                    "{stem}: pdf_sha256 {fixture} but MANIFEST.json says {manifest} for `{pdf}` — \
                     the IRS REVISED the form (drafts are replaced in place) or the fixture was \
                     edited. REVIEW the revision against the ledger; do not regenerate silently."
                ),
            }
        }
    }

    /// The whole check, over ONE fixture, as a pure function of the manifest and the fixture — so
    /// the planted-defect tests below can hand it a defect that must never exist on disk.
    fn findings_for(entries: &[Entry], stem: &str, g: &Geometry) -> Vec<Finding> {
        let mut out = Vec::new();
        if g.form != stem {
            out.push(Finding::NameDisagrees {
                stem: stem.to_string(),
                recorded: g.form.clone(),
            });
        }
        let pdf = match pdf_rel_for_stem(stem) {
            Ok(p) => p,
            Err(why) => {
                out.push(Finding::UnusableStem {
                    stem: stem.to_string(),
                    why,
                });
                return out;
            }
        };
        let hits: Vec<&Entry> = entries.iter().filter(|e| e.path == pdf).collect();
        match hits.as_slice() {
            [] => out.push(Finding::NoManifestEntry {
                stem: stem.to_string(),
                pdf,
            }),
            [e] => {
                if e.sha256 != g.pdf_sha256 {
                    out.push(Finding::Drifted {
                        stem: stem.to_string(),
                        pdf,
                        fixture: g.pdf_sha256.clone(),
                        manifest: e.sha256.clone(),
                    });
                }
            }
            many => out.push(Finding::Ambiguous {
                stem: stem.to_string(),
                pdf,
                shas: many.iter().map(|e| e.sha256.clone()).collect(),
            }),
        }
        out
    }

    /// ★★★ **THE GUARANTEE: every committed geometry fixture's `pdf_sha256` equals the
    /// `MANIFEST.json` sha256 of the PDF it names.**
    ///
    /// Measured 2026-09-05 before this test existed: 47 fixtures, 47 matching, 0 missing entries,
    /// and no instrument that said so.
    #[test]
    fn every_committed_geometry_fixture_matches_the_manifest() {
        let root = repo_root();
        let entries = crate::authority_manifest::load(&root).expect("MANIFEST.json loads");
        assert!(
            !entries.is_empty(),
            "MANIFEST.json parsed to zero entries — every fixture would then be reported as \
             unbacked, or worse, nothing would be compared at all"
        );

        let dir = root.join("design/forms/geometry");
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| {
                panic!(
                    "{}: geometry fixture directory unreadable: {e}",
                    dir.display()
                )
            })
            .map(|e| e.expect("directory entry is readable").path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        paths.sort();
        // ★ The walk is derived from the filesystem, so its own vacuity is the failure mode to
        //   guard: a wrong directory panics above, a wrong extension filter lands here.
        assert!(
            !paths.is_empty(),
            "no *.json fixtures under {} — the walk is not reaching the fixtures",
            dir.display()
        );

        let mut checked = 0usize;
        let mut findings: Vec<String> = Vec::new();
        for path in &paths {
            let stem = path
                .file_stem()
                .expect("a *.json path has a file stem")
                .to_string_lossy()
                .into_owned();
            // ★★ Unreadable or unparseable is a FAILURE, never a skip — the sibling cover-sheet
            //    guard shipped with `continue` here and silently skipped the very defect planted to
            //    prove it worked.
            let text = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("{stem}: geometry fixture unreadable: {e}"));
            let g: Geometry = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{stem}: geometry fixture does not parse: {e}"));
            checked += 1;
            findings.extend(
                findings_for(&entries, &stem, &g)
                    .iter()
                    .map(Finding::describe),
            );
        }
        assert_eq!(
            checked,
            paths.len(),
            "{} fixtures found but only {checked} inspected — a fixture was skipped",
            paths.len()
        );
        assert!(
            findings.is_empty(),
            "{} of {checked} committed geometry fixtures are not observations of the document \
             MANIFEST.json points at:\n  {}",
            findings.len(),
            findings.join("\n  ")
        );
    }

    // ─────────────────────────── B1: seen RED on a planted defect ───────────────────────────

    /// Two `note` entries in the manifest's real serialised shape, so these tests exercise the same
    /// `Entry` deserialisation the live manifest goes through.
    const SAMPLE_MANIFEST: &str = r#"[
      { "path": "design/forms/2026/f6251--2026-DRAFT.pdf", "kind": "form", "storage": "note",
        "sha256": "aaaa000000000000000000000000000000000000000000000000000000000001",
        "bytes": 100, "url": "https://www.irs.gov/pub/irs-dft/f6251--dft.pdf", "extract": "" },
      { "path": "design/forms/2025/f6251--2025.pdf", "kind": "form", "storage": "note",
        "sha256": "bbbb000000000000000000000000000000000000000000000000000000000002",
        "bytes": 100, "url": "https://www.irs.gov/pub/irs-pdf/f6251.pdf", "extract": "" }
    ]"#;

    fn sample_entries() -> Vec<Entry> {
        serde_json::from_str(SAMPLE_MANIFEST).expect("the sample manifest is manifest-shaped")
    }

    fn fixture(stem: &str, sha: &str) -> Geometry {
        Geometry {
            form: stem.to_string(),
            pdf_sha256: sha.to_string(),
            pages: vec![Page {
                n: 1,
                width: 612.0,
                height: 792.0,
            }],
            words: vec![],
            boxes: vec![],
        }
    }

    /// ★★★ **The kill.** A truthful fixture must pass and a fixture whose recorded hash has drifted
    /// by ONE character must be rejected — otherwise the walk above is a decoration that reports
    /// success on a stale observation, which is the exact class it exists to close.
    #[test]
    fn a_recorded_hash_that_drifts_from_the_manifest_is_rejected() {
        let entries = sample_entries();
        let truth = "aaaa000000000000000000000000000000000000000000000000000000000001";

        // Control: the fixture that tells the truth is clean.
        assert_eq!(
            findings_for(
                &entries,
                "f6251--2026-DRAFT",
                &fixture("f6251--2026-DRAFT", truth)
            ),
            vec![],
            "a fixture whose pdf_sha256 IS the manifest sha256 must pass"
        );

        // Plant: the last character only — the shape an in-place draft revision would leave, and
        // small enough that eyeballing a truncated `sha256:aaaa0000…` would not see it.
        let planted = "aaaa000000000000000000000000000000000000000000000000000000000009";
        let found = findings_for(
            &entries,
            "f6251--2026-DRAFT",
            &fixture("f6251--2026-DRAFT", planted),
        );
        assert_eq!(
            found,
            vec![Finding::Drifted {
                stem: "f6251--2026-DRAFT".into(),
                pdf: "design/forms/2026/f6251--2026-DRAFT.pdf".into(),
                fixture: planted.into(),
                manifest: truth.into(),
            }],
            "a one-character drift must be caught, not tolerated"
        );

        // And it must be compared against the RIGHT entry: the 2025 final's hash is in the same
        // manifest, so a lookup that ignored the path would flatter a draft with a final's pin.
        let cross = findings_for(
            &entries,
            "f6251--2026-DRAFT",
            &fixture(
                "f6251--2026-DRAFT",
                "bbbb000000000000000000000000000000000000000000000000000000000002",
            ),
        );
        assert!(
            matches!(cross.as_slice(), [Finding::Drifted { .. }]),
            "a draft carrying the FINAL's hash must still be rejected, got {cross:?}"
        );
    }

    /// ★★ **Skipping is not passing.** A fixture whose PDF the manifest does not track is a finding
    /// reported by name — the alternative (no entry ⇒ nothing to compare ⇒ fine) is how a whole
    /// document leaves the tracked set without anyone noticing.
    #[test]
    fn a_fixture_with_no_manifest_entry_is_named_not_skipped() {
        let entries = sample_entries();
        let found = findings_for(
            &entries,
            "f8283--2026-DRAFT",
            &fixture("f8283--2026-DRAFT", "cccc"),
        );
        assert_eq!(
            found,
            vec![Finding::NoManifestEntry {
                stem: "f8283--2026-DRAFT".into(),
                pdf: "design/forms/2026/f8283--2026-DRAFT.pdf".into(),
            }],
            "an untracked fixture must be reported by name"
        );
        assert!(
            found[0].describe().contains("f8283--2026-DRAFT"),
            "the message must name the fixture: {}",
            found[0].describe()
        );

        // And two entries claiming one path is ALSO not a pass — "the" expected hash is undefined.
        let mut dupes = sample_entries();
        let mut clone = dupes[0].clone();
        clone.sha256 = "dddd000000000000000000000000000000000000000000000000000000000003".into();
        dupes.push(clone);
        assert!(
            matches!(
                findings_for(
                    &dupes,
                    "f6251--2026-DRAFT",
                    &fixture(
                        "f6251--2026-DRAFT",
                        "aaaa000000000000000000000000000000000000000000000000000000000001"
                    )
                )
                .as_slice(),
                [Finding::Ambiguous { .. }]
            ),
            "two manifest entries for one path must be a finding, not a first-match win"
        );
    }

    /// ★ The two identities a fixture is looked up by — its FILENAME and its own `form` field — must
    /// agree, and a filename that is not a form stem is a refusal rather than a silent pass.
    #[test]
    fn a_fixture_that_misidentifies_itself_is_a_finding() {
        let entries = sample_entries();
        assert_eq!(
            findings_for(
                &entries,
                "f6251--2026-DRAFT",
                &fixture(
                    "f6251--2025",
                    "aaaa000000000000000000000000000000000000000000000000000000000001"
                )
            ),
            vec![Finding::NameDisagrees {
                stem: "f6251--2026-DRAFT".into(),
                recorded: "f6251--2025".into(),
            }],
            "the fixture's own `form` field disagreeing with its filename must be caught"
        );
        assert!(
            matches!(
                findings_for(&entries, "notastem", &fixture("notastem", "aaaa")).as_slice(),
                [Finding::UnusableStem { .. }]
            ),
            "a filename that is not `<form>--<year>` must be a finding, not an unchecked fixture"
        );
    }
}
