#!/usr/bin/env python
"""Archive IRS DRAFT forms for a tax year — fetch, VERIFY THE YEAR, extract, record.

★★★ THE YEAR IS VERIFIED, NEVER ASSUMED. The IRS serves drafts from one unversioned path
(`irs-dft/<stem>--dft.pdf`) and replaces them IN PLACE, so the file behind a URL is whatever is
current — which is not necessarily the year you wanted. Measured 2026-09-05: every headline form had
a TY2026 draft except Form 1040, whose draft URL still served a 2025 document. A pipeline that
trusted the URL would have archived a 2025 form as TY2026 authority-adjacent evidence.

★★★ AND THE FIRST VERSION OF THAT CHECK DID NOT MAKE THAT REFUSAL. It read

    years = re.findall(r"\\b20(?:1[5-9]|2[0-9])\\b", pages_1_to_3); return max(years)

— the LARGEST year mentioned anywhere on the first three pages. Every Form 1040 ever printed carries
line 36, *"Amount of line 34 you want applied to your 2026 estimated tax"*, so a TY2025 Form 1040
contains the token "2026" and `max()` returned 2026. `design/forms/2026/f1040--2026-DRAFT.pdf` was
therefore archived as TY2026 evidence while its masthead read *"1040 U.S. Individual Income Tax
Return 2025"* and all three of its footers read *"Form 1040 (2025)"*, and the docstring above went on
crediting a refusal that never happened. Two lens reports then built a TY2025-vs-TY2025 port
work-list on top of it. The instrument was green because it was blind — the house's dominant defect
shape, and the reason for the rule that no checker exists until it has been watched going RED.

So the year is now read from the two places where the form declares its OWN identity, and they must
AGREE with each other and with the year asked for:

  * the MASTHEAD — the bare year token printed in the year box at the top of the form's first page.
    Measured over the 82 PDFs committed under `design/forms/`: it is set 2.2x-3.2x the page's median
    word height (25.2pt against a 9.3pt median on Form 1040) and sits in the top 6% of the page,
    while every prose and revision-date occurrence is body-sized AND carries adjacent punctuation
    (`2025,` `2025.` `2024)`), so it never tokenises as a bare year. Size, position and bareness are
    three independent reasons the same token wins; a tie between two different years REFUSES.
  * the FOOTER STAMP — `Form 1040 (2025)`, `Schedule SE (Form 1040) 2026`,
    `Schedule 1-A (Form 1040) (2026)`. The IRS prints it on every page. All occurrences must agree.

A year in a line's prose is not a declaration and is never read. `--self-test` runs both readers over
every committed form and asserts each file's document agrees with the year in its own name; it goes
RED on the `max()` reader, because `design/forms/2025/f1040--2025.pdf` carries the same line 36.

★ Non-annual forms exist and are not a failure. Form 8275 and Form 8283 are REVISION-dated
(`(Rev. October 2024)`, `(Rev. December 2025)`) and carry no tax-year masthead at all. They are
refused with that diagnosis rather than "the URL served a different year", because a refusal that
reports the wrong reason still costs someone an afternoon.

★ Everything archived here is a DRAFT: `-DRAFT` in the stored filename and an `irs-dft` URL, the two
signals `authority_manifest::Entry::is_draft` reads. Drafts are EVIDENCE ONLY, never transcribed —
see design/ty2025/SPEC.md. What they are FOR is structure: which forms exist, how lines renumber,
what a port must change. That is how Critical R2 (Schedule 1-A line 37 -> 43) was found.

Usage:  .venv/bin/python scripts/archive_drafts.py 2026 [--dry-run]
        .venv/bin/python scripts/archive_drafts.py --relayout 2026   # notes + text layers, no network
        .venv/bin/python scripts/archive_drafts.py --self-test
"""

import hashlib
import pathlib
import re
import statistics
import subprocess
import sys
import urllib.request
from collections import Counter

UA = "btctax-archiver/0.19 (US federal tax form archival; +https://github.com/bg002h/bitcoin_tax)"
ROOT = pathlib.Path(__file__).resolve().parent.parent

# The stems btctax emits or reads, in IRS draft-URL spelling.
STEMS = [
    "f1040", "f1040s1", "f1040s1a", "f1040s2", "f1040s3",
    "f1040sa", "f1040sb", "f1040sc", "f1040sd", "f1040sse",
    "f6251", "f8275", "f8283", "f8949", "f8959", "f8960", "f8995", "f8995a",
]

# A BARE year token — no comma, no period, no closing paren. That is what the masthead year box
# contains and what nothing else on an IRS form does; see the module docstring.
BARE_YEAR = re.compile(r"^20(?:1[5-9]|2[0-9])$")

# pdftotext -bbox-layout geometry. y grows downward from the top of the page.
_WORD = re.compile(
    r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">([^<]*)</word>'
)
_PAGE = re.compile(r'<page width="([\d.]+)" height="([\d.]+)"')

# The IRS identity stamp printed at the foot of every page of a form.
#   Form 1040 (2025) · Form 8995-A (2026) · Schedule SE (Form 1040) 2026
#   Schedule 1-A (Form 1040) (2026)
# A reference to another form in prose ("Schedule K-1 (Form 1041)", "Schedule A (Form 8936), Part II")
# carries no year and so cannot match.
_STAMP = re.compile(
    r"(?:Form\s+([0-9][0-9A-Za-z\-]*)\s*\((20\d\d)\)"
    r"|Schedule\s+([0-9A-Z][0-9A-Za-z\-]*)\s+\(Form\s+[0-9][0-9A-Za-z\-]*\)\s+\(?(20\d\d)\)?)"
)

# A revision-dated (non-annual) form: "(Rev. October 2024)". These have no tax year to check.
_REV = re.compile(r"\(Rev\.\s+([A-Z][a-z]+\s+20\d\d)\)")

# The masthead year box sits at the very top of the page. Measured maximum over the 82 committed
# PDFs: yMin = 49pt on a 792pt page (6.2%). The window is deliberately far looser than the
# measurement, and falling outside it produces a NAMED REFUSAL, never a silent pass.
TOP_OF_PAGE = 0.20


# ★★★ THE TWO OUTPUTS OF ARCHIVING, AND WHY THEY MAY NEVER SHARE A FILENAME.
#
# Every archived binary produces exactly two committed artifacts, and they answer different
# questions:
#
#   the PROVENANCE NOTE   `design/forms/<year>/<name>.pdf.txt`   where did these bytes come from?
#   the TEXT LAYER        `design/forms/extract/<stem>.txt`      what does the document SAY?
#
# Until 2026-09-05 this script wrote `pdftotext -layout` output to the FIRST of those — the
# filename design/forms/README.md reserves for the note — so for TY2026 one suffix meant two
# things: a ~740-byte URL+sha256 note in 2024/2025, and a ~9,494-byte form dump in 2026. Both
# costs were real and both were silent:
#
#   * `authority_manifest::extract_for` resolves a text layer at ONE path,
#     `design/forms/extract/<stem>.txt`. Nothing was there, so all 15 TY2026 manifest entries
#     recorded `"extract": ""` — *a year whose text layer IS extracted reading as not extracted*.
#   * `authority_manifest::verify` checked only that `<binary>.txt` EXISTED, and a dump exists
#     just as well as a note does. So 15 documents had no recorded URL and no recorded hash while
#     the manifest reported them fine.
#
# The bar against a repeat is structural rather than a naming convention: `note_text` writes a
# fetch URL and a sha256, `extract_text` writes a `# GENERATED` header and the form's own words,
# and `authority_manifest::Problem::NoteIsNotAProvenanceNote` reds on any `<binary>.txt` that is
# not the former. A text layer cannot satisfy that check without ceasing to be a text layer.
EXTRACT_DIR = ROOT / "design" / "forms" / "extract"


def draft_url(stem: str) -> str:
    """The ONE unversioned IRS path a draft is served from."""
    return f"https://www.irs.gov/pub/irs-dft/{stem}--dft.pdf"


def note_path(pdf: pathlib.Path) -> pathlib.Path:
    """The provenance note for `pdf` — beside it, `<name>.pdf.txt`."""
    return pdf.with_name(pdf.name + ".txt")


def extract_path(pdf: pathlib.Path, extract_dir: pathlib.Path | None = None) -> pathlib.Path:
    """The committed text layer for `pdf`.

    ★ `<extract_dir>/<stem>.txt` is not a choice — it is the ONLY path
    `authority_manifest::extract_for` resolves for a `design/forms/<year>/<stem>.pdf` source
    (`crates/xtask/src/authority_manifest.rs`, arm (A)). Writing anywhere else populates nothing.

    ★ The stem keeps its `-DRAFT` marker, so `line_coverage_check`'s `<form>--<year>` lookup can
    never land on a draft's text layer by accident: `f6251--2026` is not `f6251--2026-DRAFT`.
    """
    return (extract_dir or EXTRACT_DIR) / (pdf.stem + ".txt")


def note_text(pdf: pathlib.Path, url: str, sha: str, nbytes: int, extract_rel: str) -> str:
    """The provenance note — line 1 is the URL, and the sha256 the bytes must reproduce."""
    return (
        f"{url}\n"
        f"\n"
        f"# {pdf.name} — IRS primary source, NOT committed (publicly available; keeps the repo small).\n"
        f"# sha256  {sha}\n"
        f"# bytes   {nbytes}\n"
        f"#\n"
        f"# Fetch:    curl -sL -o {pdf.name} {url}\n"
        f"# Verify:   sha256sum {pdf.name}   # must equal the sha256 above\n"
        f"#\n"
        f"# ★★ THIS IS A DRAFT — evidence only, NEVER transcribed as authority (design/ty2025/SPEC.md).\n"
        f"#    The IRS serves drafts from one unversioned path and REPLACES THEM IN PLACE, so a\n"
        f"#    re-fetch that hashes differently means the IRS posted a NEW draft. Review the diff\n"
        f"#    and update deliberately — never absorb it silently.\n"
        f"#\n"
        f"# The committed text layer is\n"
        f"# {extract_rel} — that is what the conformance tests read,\n"
        f"# so they run with no PDF and no network.\n"
    )


def _rel(p: pathlib.Path) -> str:
    """Repo-relative when it is in the repo; the bare name in a temp fixture."""
    return p.relative_to(ROOT).as_posix() if p.is_relative_to(ROOT) else p.name


def extract_text(pdf: pathlib.Path, sha: str, year: int, body: str) -> str:
    """The text layer, with the same `# GENERATED` header the 64 committed extracts carry."""
    rel = _rel(pdf)
    return (
        f"# GENERATED — do not hand-edit. Text layer of {rel}\n"
        f"# sha256:{sha[:16]}…  |  pdftotext -layout\n"
        f"# Regenerate: .venv/bin/python scripts/archive_drafts.py --relayout {year}\n"
        f"# ★ DRAFT — evidence only, never transcribed as authority (design/ty2025/SPEC.md).\n"
        f"#\n"
        f"{body}"
    )


def record(pdf: pathlib.Path, url: str, year: int,
           extract_dir: pathlib.Path | None = None) -> tuple[pathlib.Path, pathlib.Path]:
    """Write BOTH artifacts for an archived binary, to their two distinct homes.

    Returns `(note, extract)`. Raises if they would be the same file — the defect this replaced.
    """
    note = note_path(pdf)
    extract = extract_path(pdf, extract_dir)
    if note.resolve() == extract.resolve():
        raise AssertionError(
            f"the provenance note and the text layer resolve to ONE file ({note}); they answer "
            f"different questions and must never share a name"
        )
    data = pdf.read_bytes()
    sha = hashlib.sha256(data).hexdigest()
    extract.parent.mkdir(parents=True, exist_ok=True)
    note.write_text(note_text(pdf, url, sha, len(data), _rel(extract)))
    body = subprocess.run(
        ["pdftotext", "-layout", str(pdf), "-"],
        capture_output=True, text=True, timeout=300,
    ).stdout
    extract.write_text(extract_text(pdf, sha, year, body))
    return note, extract


def fetch(url: str) -> bytes | None:
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.read()
    except Exception:
        return None


def _text(pdf: pathlib.Path, first: int, last: int) -> str:
    try:
        return subprocess.run(
            ["pdftotext", "-layout", "-f", str(first), "-l", str(last), str(pdf), "-"],
            capture_output=True, text=True, timeout=120,
        ).stdout
    except Exception:
        return ""


def _bbox(pdf: pathlib.Path, page: int) -> str:
    try:
        return subprocess.run(
            ["pdftotext", "-bbox-layout", "-f", str(page), "-l", str(page), str(pdf), "-"],
            capture_output=True, text=True, timeout=120,
        ).stdout
    except Exception:
        return ""


def says_draft(pdf: pathlib.Path) -> bool:
    """Does the DOCUMENT say it is a draft?

    ★★ The strongest of the three signals, and the only one the IRS controls rather than us. Every
    IRS draft is served with a COVER SHEET on page 1 reading "Caution: DRAFT—NOT FOR FILING"; the
    form itself begins on page 2. A file lacking that cover is a FINAL served from the draft path,
    and archiving it under a `-DRAFT` name would put a mislabelled document in the manifest — the
    inverse of R20 and just as bad.
    """
    head = _text(pdf, 1, 1).upper()
    return "DRAFT" in head and "NOT FOR FILING" in head


def first_form_page(pdf: pathlib.Path) -> int | None:
    """The first page that is the FORM rather than a draft cover sheet.

    ★ DERIVED, not assumed to be 2. "NOT FOR FILING" appears on the cover and on no form page —
    measured on `f1040--2026-DRAFT.pdf`: page 1 yes, pages 2 and 3 no. A final served from a
    non-draft path has no cover and answers 1. If every page looks like a cover, we return None and
    the caller refuses by name.
    """
    for n in range(1, 5):
        page = _text(pdf, n, n)
        if not page.strip():
            break
        if "NOT FOR FILING" not in page.upper():
            return n
    return None


def pick_masthead(words, page_height: float) -> tuple[int | None, str]:
    """Choose the masthead year from a page's laid-out words. Pure, so it can be killed directly.

    `words` is an iterable of `(height_pt, text, y_top)`. A candidate must be all three of:

      (a) a BARE year — prose and revision dates always carry adjacent punctuation and so tokenise
          as `2025,` `2025.` `2024)`, never as `2025`;
      (b) in the top fifth of the page — that is where a masthead is;
      (c) set larger than the page's median word height — a year box is display type, not body type.

    Of the survivors the TALLEST wins; if two survivors are tied at the top and disagree about the
    year, we refuse rather than pick. Each conjunct has its own kill row in `_reader_kills`.
    """
    words = list(words)
    if not words:
        return None, "no-text-layer"
    median_h = statistics.median(h for h, _, _ in words)
    cands = [
        (h, w) for h, w, y in words
        if BARE_YEAR.match(w) and y < page_height * TOP_OF_PAGE and h > median_h
    ]
    if not cands:
        return None, f"no-masthead-year-box (median word height {median_h:.1f}pt)"
    tallest = max(h for h, _ in cands)
    winners = {w for h, w in cands if h >= tallest - 0.5}
    if len(winners) != 1:
        return None, f"ambiguous-masthead {sorted(winners)} tied at {tallest:.1f}pt"
    return int(winners.pop()), f"{tallest:.1f}pt vs {median_h:.1f}pt median"


def masthead_year(pdf: pathlib.Path, page: int) -> tuple[int | None, str]:
    """`pick_masthead` over the real geometry of `page`. Returns (year, note)."""
    out = _bbox(pdf, page)
    words = [
        (float(m.group(4)) - float(m.group(2)), m.group(5), float(m.group(2)))
        for m in _WORD.finditer(out)
    ]
    pm = _PAGE.search(out)
    return pick_masthead(words, float(pm.group(2)) if pm else 792.0)


def stamp_years(pdf: pathlib.Path, first_page: int) -> Counter:
    """Every `Form N (YYYY)` / `Schedule X (Form 1040) YYYY` identity stamp, counted."""
    return Counter(
        (m.group(1) or ("Schedule " + m.group(3)), int(m.group(2) or m.group(4)))
        for m in _STAMP.finditer(_text(pdf, first_page, 99))
    )


def revision_date(pdf: pathlib.Path, page: int) -> str | None:
    """`(Rev. October 2024)` if the form is revision-dated rather than annual."""
    m = _REV.search(_text(pdf, page, page))
    return m.group(1) if m else None


def adjudicate(masthead, masthead_note, stamps, want, *, require_stamp=True):
    """The DECISION, separated from the readers so it can be killed without a PDF.

    Returns `(accepted, code, detail)`. The CODE is what the kill table asserts on: a row that
    checked only accepted/rejected could pass on a branch it was not testing, and one of them
    silently did — see `_decision_kills`. Every rejection names its own cause; there is no path on
    which an unreadable or ambiguous document is treated as fine. `require_stamp=False` is for
    auditing instruction booklets, whose footers do not always carry the stamp; the archiver never
    uses it.
    """
    stamp_years_seen = sorted({y for (_, y) in stamps})
    if masthead is None:
        return False, "no-masthead", f"no tax year in the masthead — {masthead_note}"
    if len(stamp_years_seen) > 1:
        return False, "stamps-disagree", (
            f"the footer stamps disagree with each other: {stamp_years_seen}")
    if not stamp_years_seen:
        if require_stamp:
            return False, "no-stamp", "no `Form N (YYYY)` footer stamp anywhere in the document"
    else:
        stamped = stamp_years_seen[0]
        if stamped != masthead:
            return False, "masthead-vs-footer", (
                f"the masthead says {masthead} but the footer stamp says {stamped} — "
                f"the document contradicts itself")
    if want is not None and masthead != want:
        return False, "wrong-year", f"the document declares {masthead}, not {want}"
    return True, "ok", (
        f"declares {masthead} (masthead {masthead_note}; stamps {sorted(stamps.items())})")


def declared_year(pdf: pathlib.Path, want: int | None, *, require_stamp=True):
    """Read the document's own declared tax year and adjudicate it against `want`."""
    page = first_form_page(pdf)
    if page is None:
        return False, None, "every page carries the DRAFT cover marker — no form page found"
    mast, note = masthead_year(pdf, page)
    stamps = stamp_years(pdf, page)
    if mast is None and not stamps:
        rev = revision_date(pdf, page)
        if rev:
            return False, None, (f"revision-dated, not annual: (Rev. {rev}) — this form has no tax "
                                 f"year to check")
    ok, _code, why = adjudicate(mast, note, stamps, want, require_stamp=require_stamp)
    return ok, mast, why


# --------------------------------------------------------------------------------------------
# --self-test — the checker is not trusted until it has been watched going RED (CLAUDE.md B1).
# --------------------------------------------------------------------------------------------

def classify_form_pdf(p: pathlib.Path):
    """Classify ONE archived form PDF as `declares` / `excused` / `PROBLEM`, with a reason.

    ★★ SKIPPING IS NOT PASSING. There are exactly three outcomes and no silent fourth: a form
    either declares the tax year its filename claims, or is demonstrably revision-dated (a
    `(Rev. Month Year)` marker AND no year box — the excuse must BE the mechanism, not a shrug),
    or it is a PROBLEM. A document whose text layer cannot be read is a PROBLEM, never an excuse —
    "I could not check this" and "this is fine" must never produce the same output.

    One implementation, two call sites: the corpus audit runs it over the committed archive, and
    `_negative_fixture_kill` runs it over deliberately broken files in a temp directory. That is
    what makes the excuse path killable — the committed archive is clean, so no mutation of this
    function can be witnessed against it alone.
    """
    m = re.search(r"--(\d{4})", p.name)
    if not m:
        return "PROBLEM", "filename declares no year"
    want = int(m.group(1))
    ok, _mast, why = declared_year(p, want)
    if ok:
        return "declares", why
    page = first_form_page(p) or 1
    if masthead_year(p, page)[0] is None and revision_date(p, page):
        return "excused", f"revision-dated: (Rev. {revision_date(p, page)})"
    return "PROBLEM", f"filename says {want} but {why}"


def _negative_fixture_kill() -> list[str]:
    """Drive `classify_form_pdf` with files that are deliberately wrong.

    Nothing here touches `design/forms/` — the fixtures are copies in a temp directory under
    different names, because manufacturing a mislabelled document inside the authority archive is
    the very thing this script exists to prevent.
    """
    import shutil
    import tempfile
    annual = ROOT / "design/forms/2025/f1040--2025.pdf"
    periodic = ROOT / "design/forms/2025/f8283--2025.pdf"
    for src in (annual, periodic):
        if not src.is_file():
            return [f"negative fixtures: {src.name} is absent, so the kill cannot run — FAIL"]
    fails = []
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        # (1) A real annual form offered under a year it does not declare must be a PROBLEM.
        mislabelled = d / "f1040--2099.pdf"
        shutil.copyfile(annual, mislabelled)
        v, why = classify_form_pdf(mislabelled)
        if v != "PROBLEM":
            fails.append(f"negative fixture: a TY2025 Form 1040 named `--2099` classified as "
                         f"'{v}' ({why}) — it must be a PROBLEM")
        # (2) A genuinely revision-dated form is excused — the excuse must still work.
        rev = d / "f8283--2099.pdf"
        shutil.copyfile(periodic, rev)
        v, why = classify_form_pdf(rev)
        if v != "excused":
            fails.append(f"negative fixture: revision-dated Form 8283 classified as '{v}' ({why}) "
                         f"— the `(Rev. ...)` excuse stopped working")
        # (3) A file whose text layer cannot be read is a PROBLEM, NOT an excuse. This is the row
        #     that dies when the excuse is widened from "revision-dated" to "no masthead found".
        unreadable = d / "f9999--2026.pdf"
        unreadable.write_bytes(b"%PDF-1.4\n% not a real pdf\n")
        v, why = classify_form_pdf(unreadable)
        if v != "PROBLEM":
            fails.append(f"negative fixture: an unreadable PDF classified as '{v}' ({why}) — "
                         f"'I could not check this' was treated as 'this is fine'")
    return fails


def _note_extract_collision_kill() -> list[str]:
    """★★★ THE B1 KILL for F5 — a provenance note and a text layer may never be one file.

    `record` is driven over a REAL archived draft in a temp directory, and both outputs are then
    checked for the properties that make them different KINDS of artifact:

      the note      first line is the fetch URL, carries a 64-hex sha256, and is SMALL — it
                    vouches for bytes it does not contain.
      the text layer `# GENERATED` header, lands under `<extract_dir>/<stem>.txt` (the ONE path
                    `authority_manifest::extract_for` resolves), and is the form's own words.

    ★ THE PLANTS THIS REDS ON, each the actual regression it guards:
      * point `record`'s pdftotext output back at `note_path(pdf)` (the code this replaced):
        rows 1-3 red — the note no longer starts with a URL, carries no sha, and is 9 KB.
      * make `extract_path` return `pdf.with_name(pdf.name + ".txt")`: row 0 reds — `record`
        itself raises before writing, because the two resolve to one file.
      * move the extract anywhere but `<extract_dir>/<stem>.txt`: row 5 reds, and with it the
        manifest's `extract` field goes empty again, which is the defect that started this.

    Nothing here writes into `design/forms/` — the archive is read, never manufactured into.
    """
    import shutil
    import tempfile

    drafts = sorted(ROOT.glob("design/forms/[0-9][0-9][0-9][0-9]/*-DRAFT.pdf"))
    if not drafts:
        return ["note/extract collision kill: no archived draft PDF — the check cannot run, so it FAILS"]
    src = drafts[0]
    year = int(re.search(r"--(\d{4})-DRAFT", src.name).group(1))
    stem = src.name.split("--", 1)[0]
    fails = []
    with tempfile.TemporaryDirectory() as d:
        d = pathlib.Path(d)
        (d / str(year)).mkdir()
        pdf = d / str(year) / src.name
        shutil.copyfile(src, pdf)
        edir = d / "extract"
        url = draft_url(stem)
        try:
            note, extract = record(pdf, url, year, extract_dir=edir)
        except AssertionError as e:
            return [f"note/extract collision: record refused to write — {e}"]

        # 0. the two artifacts are two files.
        if note.resolve() == extract.resolve():
            fails.append("collision kill: the note and the text layer are the SAME FILE")
        # 1-3. the note is a note: URL first, a sha256 in it, and small enough not to be a dump.
        note_txt = note.read_text()
        if not note_txt.startswith(url):
            fails.append(f"collision kill: the note does not start with the fetch URL — it starts "
                         f"{note_txt.splitlines()[0][:60]!r}")
        if not re.search(r"\b[0-9a-f]{64}\b", note_txt):
            fails.append("collision kill: the note records no sha256, so it vouches for nothing")
        if len(note_txt) > 4000:
            fails.append(f"collision kill: the note is {len(note_txt):,} bytes — that is a document "
                         f"dump, not a provenance note")
        if "NOT FOR FILING" in note_txt.upper():
            fails.append("collision kill: the note contains the DRAFT COVER SHEET — pdftotext "
                         "output was written to the note path")
        # 4. the text layer is a text layer, not a note.
        ex_txt = extract.read_text()
        if not ex_txt.startswith("# GENERATED"):
            fails.append(f"collision kill: the text layer lacks the GENERATED header — it starts "
                         f"{ex_txt.splitlines()[0][:60]!r}")
        if ex_txt.splitlines()[0].startswith("http") or len(ex_txt) < 3000:
            fails.append(f"collision kill: the text layer is {len(ex_txt):,} bytes and does not look "
                         f"like a form dump")
        # 5. and it is where the MANIFEST looks: <extract_dir>/<stem>.txt, stem keeping -DRAFT.
        want = edir / (pdf.stem + ".txt")
        if extract != want:
            fails.append(f"collision kill: the text layer went to {extract}, but "
                         f"authority_manifest::extract_for resolves only {want}")

    # And the same contract over the REAL archive, enumerated from the filesystem: every archived
    # draft's text layer must exist at the path the manifest resolves. A draft with no text layer
    # here is the F5 defect itself, still present.
    for pdf in drafts:
        want = extract_path(pdf)
        if not want.is_file():
            fails.append(f"{pdf.relative_to(ROOT)} has no text layer at "
                         f"{want.relative_to(ROOT)} — the manifest will record `extract: \"\"`")
        note = note_path(pdf)
        if not note.is_file():
            fails.append(f"{pdf.relative_to(ROOT)} has no provenance note at {note.relative_to(ROOT)}")
        elif not note.read_text().startswith("http"):
            fails.append(f"{note.relative_to(ROOT)} is not a provenance note — its first line is "
                         f"not a URL")
    return fails


def _corpus_audit() -> list[str]:
    """Every committed FORM must declare the year its own filename claims.

    The file set is enumerated FROM THE FILESYSTEM (`design/forms/*/f*.pdf`), never from a list, so
    a form added tomorrow is audited without editing this script. A file that declares no tax year
    passes only if it is demonstrably revision-dated; "could not read it" is a FAILURE, not a skip.
    """
    fails, excused, checked = [], [], []
    pdfs = sorted(ROOT.glob("design/forms/[0-9][0-9][0-9][0-9]/f*.pdf"))
    if len(pdfs) < 30:
        fails.append(f"corpus audit found only {len(pdfs)} form PDFs — the glob is not finding them")
    for p in pdfs:
        verdict, detail = classify_form_pdf(p)
        if verdict == "declares":
            checked.append(p.name)
        elif verdict == "excused":
            excused.append(p.name)  # non-annual form — excused BY NAME, never silently
        else:
            fails.append(f"{p.relative_to(ROOT)}: {detail}")
    if len(checked) + len(excused) + len(fails) != len(pdfs):
        fails.append(f"{len(pdfs)} form PDFs globbed but {len(checked)} checked + {len(excused)} "
                     f"excused + {len(fails)} failed does not account for all of them")
    return fails, excused, checked


def _decision_kills() -> list[str]:
    """The adjudicator's own kill table. Each row is a defect this check exists to catch."""
    S = Counter({("1040", 2026): 3})
    cases = [
        # ★★ Each row asserts the REASON CODE, not just accept/reject. Two rows here were measured
        # passing on the wrong branch when their mechanism was deleted:
        #   * this first row originally used masthead=2025/want=2026, so removing the
        #     masthead-vs-footer check left `wrong-year` to refuse it and the row stayed GREEN;
        #   * the disagreeing-stamps row refused via an arbitrary `set` pick, so it stayed GREEN or
        #     went RED depending on hash order.
        # Both are the B1 disease inside the kill table itself. Asserting the code fixes both.
        ("masthead matches the year asked for but the footer stamp contradicts it",
         (2026, "25.2pt", Counter({("1040", 2025): 3}), 2026), (False, "masthead-vs-footer")),
        ("masthead disagrees with footer stamp",
         (2025, "25.2pt", Counter({("1040", 2026): 3}), 2026), (False, "masthead-vs-footer")),
        ("masthead right, stamp right, year right",
         (2026, "25.2pt", S, 2026), (True, "ok")),
        ("masthead and stamp agree but on the wrong year",
         (2025, "25.2pt", Counter({("1040", 2025): 3}), 2026), (False, "wrong-year")),
        ("no masthead year at all",
         (None, "no-masthead-year-box", S, 2026), (False, "no-masthead")),
        ("ambiguous masthead (two years tied)",
         (None, "ambiguous-masthead ['2025', '2026']", S, 2026), (False, "no-masthead")),
        ("footer stamps disagree with each other",
         (2026, "25.2pt", Counter({("1040", 2026): 2, ("1040", 2025): 1}), 2026),
         (False, "stamps-disagree")),
        ("no footer stamp at all",
         (2026, "25.2pt", Counter(), 2026), (False, "no-stamp")),
    ]
    fails = []
    for name, args, expect in cases:
        try:
            ok, code, why = adjudicate(*args)
        except Exception as e:  # a kill row that EXPLODES is red, never green
            fails.append(f"decision kill '{name}': raised {e!r}")
            continue
        if (ok, code) != expect:
            fails.append(f"decision kill '{name}': expected {expect}, got {(ok, code)} ({why})")
    return fails


def _reader_kills() -> list[str]:
    """`pick_masthead`'s kill table — one row per conjunct, so no guard is unwitnessed.

    Heights and positions are the MEASURED ones: a Form 1040 masthead is 25.2pt against a 9.3pt
    median at y=23, its body text is 8.2pt, and a `(Rev. October 2024)` token sits at 8.2pt/y=67.
    """
    body93 = [(9.3, "word", 300.0)] * 40
    body105 = [(10.5, "word", 300.0)] * 40
    cases = [
        ("a real masthead is read",
         body93 + [(25.2, "2026", 45.0)], 2026),
        ("BARE-year guard: display-size prose keeps its punctuation and is not a masthead",
         body93 + [(25.2, "2025,", 45.0)], None),
        ("TOP-OF-PAGE guard: a large bare year down the page is not a masthead",
         body93 + [(25.2, "2019", 600.0)], None),
        ("SIZE guard: a body-sized bare year at the top is not a year box",
         body105 + [(8.2, "2024", 67.0)], None),
        ("the tallest candidate wins over a smaller one",
         body93 + [(25.2, "2026", 45.0), (10.0, "2025", 100.0)], 2026),
        ("two different years tied at the top size REFUSE rather than pick",
         body93 + [(25.2, "2026", 45.0), (25.2, "2025", 45.0)], None),
        ("a page with no text at all refuses",
         [], None),
    ]
    fails = []
    for name, words, expect in cases:
        try:
            got, note = pick_masthead(words, 792.0)
        except Exception as e:  # e.g. dropping the BARE-year anchor makes int("2025,") explode
            fails.append(f"reader kill '{name}': raised {e!r}")
            continue
        if got != expect:
            fails.append(f"reader kill '{name}': expected {expect}, got {got} ({note})")
    return fails


def _end_to_end_kill() -> list[str]:
    """The whole stack — page selection, masthead, footer stamp, adjudication — over a REAL PDF.

    A committed form must be ACCEPTED for the year it declares and REFUSED for the next one. The
    second half is the kill: it is the archiver's actual failure mode (a document offered as a year
    it does not declare), run against bytes on disk rather than a synthetic tuple.
    """
    pdfs = sorted(ROOT.glob("design/forms/[0-9][0-9][0-9][0-9]/f1040s?--*.pdf"))
    if not pdfs:
        return ["end-to-end kill: no witness PDF found — the check cannot run, so it FAILS"]
    p = pdfs[0]
    want = int(re.search(r"--(\d{4})", p.name).group(1))
    fails = []
    ok, _, why = declared_year(p, want)
    if not ok:
        fails.append(f"end-to-end: {p.name} should be accepted for {want} but was refused ({why})")
    ok, _, why = declared_year(p, want + 1)
    if ok:
        fails.append(f"end-to-end KILL FAILED: {p.name} was accepted as {want + 1} ({why})")
    return fails


def self_test() -> int:
    print("  reader kills ...")
    fails = _reader_kills()
    print(f"    {'FAIL' if fails else 'ok'} — 7 rows")
    print("  decision kills ...")
    d = _decision_kills()
    print(f"    {'FAIL' if d else 'ok'} — 8 rows")
    fails += d
    print("  end-to-end kill on a real document ...")
    e2e = _end_to_end_kill()
    print(f"    {'FAIL' if e2e else 'ok'} — 2 rows")
    fails += e2e
    print("  negative fixtures (temp dir, never the archive) ...")
    nf = _negative_fixture_kill()
    print(f"    {'FAIL' if nf else 'ok'} — 3 rows")
    fails += nf
    print("  note/extract collision kill (F5) ...")
    ne = _note_extract_collision_kill()
    print(f"    {'FAIL' if ne else 'ok'} — 6 rows in a temp dir + every archived draft")
    fails += ne
    print("  corpus audit (design/forms/*/f*.pdf) ...")
    corpus, excused, checked = _corpus_audit()
    print(f"    {'FAIL' if corpus else 'ok'} — {len(checked)} declared their own year, "
          f"{len(excused)} excused as revision-dated: {', '.join(sorted(excused)) or 'none'}")
    fails += corpus
    if fails:
        print("\n  SELF-TEST FAILED:")
        for f in fails:
            print(f"    {f}")
        return 1
    print("\n  self-test PASSED")
    return 0


def relayout(year: int) -> int:
    """Rewrite the note + text layer for drafts ALREADY archived under `design/forms/<year>/`.

    No network. This is what repairs an archive written by the old code path, and it is the
    `# Regenerate:` command the extract headers name.

    ★ Only `*--<year>-DRAFT.pdf` is touched, so the TY2024/TY2025 finals — whose notes are correct
      and hand-verified by round-trip — are structurally out of reach.
    ★ A draft whose binary is absent (gitignored, not fetched) is REPORTED BY NAME and fails the
      run. It cannot be re-hashed, so "skip it" would quietly leave a wrong note in place.
    """
    ydir = ROOT / "design" / "forms" / str(year)
    pdfs = sorted(ydir.glob(f"*--{year}-DRAFT.pdf"))
    stale = sorted(
        n for n in ydir.glob(f"*--{year}-DRAFT.pdf.txt")
        if not n.with_suffix("").exists()
    )
    if not pdfs and not stale:
        print(f"  no TY{year} drafts archived under {ydir.relative_to(ROOT)}")
        return 1
    for pdf in pdfs:
        stem = pdf.name.split("--", 1)[0]
        note, extract = record(pdf, draft_url(stem), year)
        print(f"  {stem:10} note {note.relative_to(ROOT)}  ·  "
              f"extract {extract.relative_to(ROOT)} ({extract.stat().st_size:,} bytes)")
    print(f"\n  relayout: {len(pdfs)} draft(s) rewritten")
    if stale:
        print("  ** FAILED — these .pdf.txt files have no binary to re-hash, so their contents "
              "cannot be trusted as a note; fetch the drafts and re-run: **")
        for n in stale:
            print(f"    {n.relative_to(ROOT)}")
        return 1
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    if "--relayout" in sys.argv:
        i = sys.argv.index("--relayout")
        if i + 1 >= len(sys.argv) or not sys.argv[i + 1].isdigit():
            print("usage: --relayout <year>")
            return 2
        return relayout(int(sys.argv[i + 1]))
    if len(sys.argv) < 2 or not sys.argv[1].isdigit():
        print(__doc__)
        return 2
    year = int(sys.argv[1])
    dry = "--dry-run" in sys.argv
    outdir = ROOT / "design" / "forms" / str(year)
    outdir.mkdir(parents=True, exist_ok=True)

    got, refused, absent, not_a_draft = [], [], [], []
    for stem in STEMS:
        url = f"https://www.irs.gov/pub/irs-dft/{stem}--dft.pdf"
        body = fetch(url)
        if body is None or not body.startswith(b"%PDF"):
            absent.append(stem)
            print(f"  {stem:10} ABSENT — no draft at {url}")
            continue
        dest = outdir / f"{stem}--{year}-DRAFT.pdf"
        tmp = dest.with_suffix(".pdf.tmp")
        tmp.write_bytes(body)
        if not says_draft(tmp):
            tmp.unlink(missing_ok=True)
            not_a_draft.append(stem)
            print(f"  {stem:10} ** REFUSED — no DRAFT cover sheet; this is a FINAL served from the "
                  f"draft path, and archiving it as a draft would mislabel it **")
            continue
        ok, found, why = declared_year(tmp, year)
        if not ok:
            tmp.unlink(missing_ok=True)
            refused.append((stem, why))
            print(f"  {stem:10} ** REFUSED — {why} **")
            continue
        if dry:
            tmp.unlink(missing_ok=True)
            print(f"  {stem:10} ok (dry-run) — {why}, {len(body):,} bytes")
            got.append(stem)
            continue
        tmp.replace(dest)
        # ★ BOTH artifacts, to their two distinct homes — see the EXTRACT_DIR block above. This
        #   line used to be `pdftotext -layout <dest> <dest>.txt`, which put the text layer on the
        #   provenance note's path and left the manifest recording `"extract": ""` for every draft.
        record(dest, url, year)
        got.append(stem)
        print(f"  {stem:10} archived — {why}, {len(body):,} bytes, "
              f"sha256:{hashlib.sha256(body).hexdigest()[:8]}")

    print(f"\n  archived {len(got)}  ·  refused {len(refused)}  ·  "
          f"refused-not-a-draft {len(not_a_draft)}  ·  absent {len(absent)}")
    if refused:
        print("  REFUSED (the document did not declare the year asked for — the check working):")
        for stem, why in refused:
            print(f"    {stem}: {why}")
    if not_a_draft:
        print(f"  served a FINAL from the draft path (refused): {', '.join(not_a_draft)}")
    if absent:
        print(f"  no draft published yet: {', '.join(absent)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
