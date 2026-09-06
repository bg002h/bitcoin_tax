#!/usr/bin/env python
"""Archive IRS DRAFT forms for a tax year — fetch, VERIFY THE YEAR, extract, record.

★★★ THE YEAR IS VERIFIED, NEVER ASSUMED. The IRS serves drafts from one unversioned path
(`irs-dft/<stem>--dft.pdf`) and replaces them IN PLACE, so the file behind a URL is whatever is
current — which is not necessarily the year you wanted. Measured 2026-09-05: every headline form had
a TY2026 draft except Form 1040, whose draft URL still served a 2025 document. A pipeline that
trusted the URL would have archived a 2025 form as TY2026 authority-adjacent evidence.

So each fetch is checked against the year printed ON the document, and a mismatch is REFUSED with
the year actually found. That refusal is the feature.

★ Everything archived here is a DRAFT: `-DRAFT` in the stored filename and an `irs-dft` URL, the two
signals `authority_manifest::Entry::is_draft` reads. Drafts are EVIDENCE ONLY, never transcribed —
see design/ty2025/SPEC.md. What they are FOR is structure: which forms exist, how lines renumber,
what a port must change. That is how Critical R2 (Schedule 1-A line 37 -> 43) was found.

Usage:  .venv/bin/python scripts/archive_drafts.py 2026 [--dry-run]
"""

import hashlib
import json
import pathlib
import re
import subprocess
import sys
import urllib.request

UA = "btctax-archiver/0.18 (US federal tax form archival; +https://github.com/bg002h/bitcoin_tax)"
ROOT = pathlib.Path(__file__).resolve().parent.parent

# The stems btctax emits or reads, in IRS draft-URL spelling.
STEMS = [
    "f1040", "f1040s1", "f1040s1a", "f1040s2", "f1040s3",
    "f1040sa", "f1040sb", "f1040sc", "f1040sd", "f1040sse",
    "f6251", "f8275", "f8283", "f8949", "f8959", "f8960", "f8995", "f8995a",
]


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


def printed_year(pdf: pathlib.Path) -> int | None:
    """The tax year printed ON the form, read from the TEXT LAYER.

    ★ Never from the filename and never from the URL: the whole point is that those lie. The IRS
    serves drafts from ONE unversioned path and replaces them in place.

    ★★ Pages 1-3, not page 1. A first draft of this read only page 1 and returned None for all 18
    forms — because page 1 is the DRAFT COVER SHEET and carries no year. It refused everything,
    which was the right DIRECTION and the wrong DIAGNOSIS: "the URL served another year" when the
    truth was "my reader was looking at the wrong page". A fail-closed check that reports the wrong
    reason still costs someone an afternoon.
    """
    out = _text(pdf, 1, 3)
    years = [int(y) for y in re.findall(r"\b20(?:1[5-9]|2[0-9])\b", out)]
    return max(years) if years else None


def main() -> int:
    if len(sys.argv) < 2 or not sys.argv[1].isdigit():
        print(__doc__)
        return 2
    year = int(sys.argv[1])
    dry = "--dry-run" in sys.argv
    outdir = ROOT / "design" / "forms" / str(year)
    outdir.mkdir(parents=True, exist_ok=True)

    got, wrong_year, absent, not_a_draft = [], [], [], []
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
        found = printed_year(tmp)
        if found != year:
            tmp.unlink(missing_ok=True)
            wrong_year.append((stem, found))
            print(f"  {stem:10} ** REFUSED — the draft at that URL prints {found}, not {year} **")
            continue
        if dry:
            tmp.unlink(missing_ok=True)
            print(f"  {stem:10} ok (dry-run) — prints {found}, {len(body):,} bytes")
            got.append(stem)
            continue
        tmp.replace(dest)
        subprocess.run(["pdftotext", "-layout", str(dest), str(dest) + ".txt"], check=False)
        got.append(stem)
        print(f"  {stem:10} archived — prints {found}, {len(body):,} bytes, "
              f"sha256:{hashlib.sha256(body).hexdigest()[:8]}")

    print(f"\n  archived {len(got)}  ·  refused-wrong-year {len(wrong_year)}  ·  "
          f"refused-not-a-draft {len(not_a_draft)}  ·  absent {len(absent)}")
    if wrong_year:
        print("  REFUSED (the URL served a different year — this is the check working):")
        for stem, found in wrong_year:
            print(f"    {stem}: prints {found}")
    if not_a_draft:
        print(f"  served a FINAL from the draft path (refused): {', '.join(not_a_draft)}")
    if absent:
        print(f"  no draft published yet: {', '.join(absent)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
