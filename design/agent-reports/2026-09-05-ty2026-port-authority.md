# TY2026 port — the FORM AUTHORITY PIPELINE lens

**Headline: nothing about TY2026 can be archived as authority before roughly November 2026 — every
`irs-prior/<stem>--2026.pdf` is 404 today (36/36, measured) — but 20 of the 30 annual documents already
exist as TY2026 DRAFTS, the three TY2025 authority gaps are two `curl`s and one alias decision away, and
three pipeline defects that will bite the port are fixable this afternoon.**

Agent: form-authority-pipeline recon lens. All measurements taken 2026-09-05 in the shared worktree at
`/scratch/code/bitcoin_tax`. No git/cargo/make/nextest was run; `./target/debug/xtask` was invoked
directly, and network probes went to `www.irs.gov` only. Downloads landed in the session scratchpad, not
the repo.

---

## 0. What the pipeline is, measured

Four artefacts per document, plus one index:

| step | artefact | count today |
|---|---|---|
| ① archive | `design/forms/<TY>/<stem>--<TY>.pdf` (gitignored) + `<…>.pdf.txt` note (URL, sha256, bytes) | 64 note-backed documents |
| ② extract | `design/forms/extract/<stem>--<TY>.txt` — the committed text layer everything reads | 64 files |
| ③ record | `design/forms/MANIFEST.json` | 106 entries |
| ④ geometry | `design/forms/geometry/<stem>--<TY>.json` (only where the label reader is used) | 12 files |

Live, from the prebuilt binary:

```
$ ./target/debug/xtask authority-manifest
authority-manifest: 106 entries — 42 committed, 64 note-only
authority-manifest: by kind — form 36, guidance 12, instructions 30, publication 6, regulation 6, statute 16
authority-manifest: 0 document(s) archived under more than one path (pinned 0 — the archives were reconciled 2026-09-04; any duplicate reds)
authority-manifest: OK — every entry resolves and every source is listed

$ ./target/debug/xtask archive-check
archive-check: no primary source outside the 5 accounted-for tree(s)
archive-check: 3 accounted-for archive(s) — hybrid, decided 2026-07-30; duplicates reconciled 2026-09-04 and pinned at 0 by `authority-manifest`

$ ./target/debug/xtask cite-check
cite-check: OK — 51 quotations, all verbatim.
cite-check: authority archived + extracted for 1/16 emitted forms (f1040s1a); 15 awaiting archive
```

**Independent check the suite does not perform.** `verify()`
(`crates/xtask/src/authority_manifest.rs:233-268`) hashes only `Storage::Committed` files; for
`Storage::Note` it asserts *the note exists*, nothing more. I hashed all 64 `design/forms` binaries
against the manifest myself:

```
design/forms manifest entries: 64   on-disk hash-true: 64   absent locally: 0   MISMATCH: 0
```

and round-tripped three against the live IRS (all byte-exact):

```
f6251    live=6995bfd29c6fe1b80fdc396c  disk=6995bfd29c6fe1b80fdc396c
f1040sa  live=c14acf3478f4c33f27ac6e8e  disk=c14acf3478f4c33f27ac6e8e
f8949    live=274513891e4e281d11e14286  disk=274513891e4e281d11e14286
```

**URL conventions**, confirmed against the manifest rather than the README: of the 31 TY2025 entries,
**29 use the year-pinned `irs-prior/<stem>--2025.pdf`** and **2 use the moving `irs-pdf/<stem>.pdf`** —
`f8275r--2025` and `i8275r--2025`, both periodic forms with no year edition (README case 3).

---

## 1. ★ The three missing TY2025 forms — established, and two are one `curl` each

`design/ty2025/recon-year-port-delta.md:19-48` already identified the gap; I re-measured it live today
and it is unchanged. The three names are **exactly** the three TY2025 bundled templates that are also
missing (`crates/btctax-forms/forms/2025/` holds 15, `…/2024/` holds 17: 2025 lacks `f1040s1`, `f8275`,
`f8995a` and adds `f1040s1a`). The archive and the template bundle moved in lockstep, which is why the
lists coincide.

### 1a. `f1040s1` and `f8995a` — available RIGHT NOW, no obstacle at all

Probed 2026-09-05:

```
200   https://www.irs.gov/pub/irs-prior/f1040s1--2025.pdf
404   https://www.irs.gov/pub/irs-prior/f8275--2025.pdf
200   https://www.irs.gov/pub/irs-prior/f8995a--2025.pdf
404   https://www.irs.gov/pub/irs-prior/i1040s1--2025.pdf
404   https://www.irs.gov/pub/irs-prior/i8275--2025.pdf
200   https://www.irs.gov/pub/irs-prior/i8995a--2025.pdf
```

Fetched, hashed, and **the printed tax year verified on the face** (`pdftotext -layout -f 1 -l 1`), not
assumed:

| document | URL | bytes | sha256 | face |
|---|---|---|---|---|
| `f1040s1--2025.pdf` | `irs-prior/f1040s1--2025.pdf` | 99,940 | `8dafec719f6a4716c259a2bdaca546d9bb9e262d1eabef885fe116a7327458fa` | `SCHEDULE 1 … 2025` |
| `f8995a--2025.pdf` | `irs-prior/f8995a--2025.pdf` | 117,129 | `3362db81b8ef60cfaa93354858c6c92bac16e63984be0805c6479d272a6ec9aa` | `Form 8995-A … 2025` |
| `i8995a--2025.pdf` | `irs-prior/i8995a--2025.pdf` | 248,825 | `6df1301c1ce74ef391219094831accd22cfeac26ce510f4499c4834484f55d97` | `2025 Instructions for Form 8995-A` |

`i1040s1--2025` is **404 and correctly so** — Schedule 1 has no standalone booklet; its instructions are
inside `i1040gi--2025`, which is already archived. That is the same "a missing year is sometimes the
correct answer" case `design/forms/README.md:26-27` records for `f1040s1a--2024`.

### 1b. `f8275` — the 404 is right, and archiving it under 2025 will RED the suite

Form 8275 is **periodic**. The current edition is **Rev. October 2024** (read off the face of what
`irs-pdf/f8275.pdf` serves today), and:

```
f8275  irs-pdf today  sha256 9b4b82e3d0dd4eceac81eec700573481be91cfafc4d7f7e9796fd4dcec5fa164  74414 bytes
MANIFEST design/forms/2024/f8275--2024.pdf  sha256 9b4b82e3d0dd4ece…  74414 bytes   ← IDENTICAL
i8275  irs-pdf today  sha256 a7b4d8b93922ad83f3fb9c65f05637b0a8e1eeb4dcbe36149aeb549694a0f785  150903 bytes
MANIFEST design/forms/2024/i8275--2024.pdf  sha256 a7b4d8b93922ad83…  150903 bytes  ← IDENTICAL
```

**So the TY2025 authority for Form 8275 is already in the repo**, under `2024/`. There is nothing to
fetch. But copying those bytes to `design/forms/2025/f8275--2025.pdf` creates a second path with the same
sha256, and `duplicates()` is keyed on **content hash**
(`crates/xtask/src/authority_manifest.rs:330-346`) against a pin of **0**:

```rust
// crates/xtask/src/authority_manifest.rs:1207
assert_eq!(dups.len(), DUPLICATE_SOURCE_GROUPS, …)
```

The code already predicts this exact moment
(`crates/xtask/src/authority_manifest.rs`, `DUPLICATE_SOURCE_GROUPS` doc comment):

> ★ **Known shape that will red this legitimately**, when it arrives: an unrevised periodic form
> archived under two tax years (`2025/f8275--2025` byte-equal to `2024/f8275--2024`). That is the
> moment to teach `duplicates()` the alias mechanism — same tree, same stem, different year — with a
> planted-defect test, per B1.

**That moment is now.** The decision is an owner call, not a fetch: either build the alias mechanism, or
leave `f8275` archived once under 2024 and make the year-independence explicit in its note. Doing
neither and copying the file turns the suite red.

**This is not a TY2025-only problem — it scales at TY2026.** Six documents are periodic:
`f8275`, `i8275`, `f8275r`, `i8275r`, `f8283`, `i8283`. Each will be a byte-identical duplicate the
moment it is archived under a second tax year while unrevised. Today `f8283--2025` is Rev. December 2025
and `irs-pdf/f8283.pdf` serves exactly those bytes (`389ab1b7c01b5427…`), so a TY2026 copy would be a
duplicate too unless the IRS revises it first. (`i8283` is *already* archived under both 2024 and 2025
and does **not** duplicate — `c217aed9…` vs `2de015f7…` — because it genuinely was revised.)

---

## 2. ★ The September-2026 hazard is real but MIS-STATED — and it has not fired

The brief warns that the plain `irs-pdf/<stem>.pdf` URL may serve a TY2026 **draft**. Measured today
across all 36 stems the archive uses:

- **Every `irs-pdf/<stem>.pdf` serves the TY2025 FINAL.** All 36 print 2025 on the face (or the
  periodic `Rev. October 2024` / `Rev. November 2024` / `Rev. December 2025`), and **0 of 36** carry a
  draft watermark.
- Drafts live at a different path entirely — `irs-pdf/` never served one in this sample. Every
  `irs-dft/<stem>--dft.pdf` copy carries `Caution: DRAFT—NOT FOR FILING` on page 1 and
  `TREASURY/IRS AND OMB USE ONLY DRAFT` across page 2.

So the correct statement of the hazard is: **`irs-pdf/<stem>.pdf` will silently flip from the TY2025
final to the TY2026 final** once each form is finalised (§4 puts that at roughly Nov 2026 – Jan 2027).
That exposes exactly the two moving-URL notes. Both are still current today:

```
design/forms/2025/f8275r--2025.pdf   url irs-pdf/f8275r.pdf   manifest sha 132e3f27…  live sha 132e3f27…  MATCH
design/forms/2025/i8275r--2025.pdf   url irs-pdf/i8275r.pdf   manifest sha 0e8fc129…  live sha 0e8fc129…  MATCH
```

**But nothing in the suite re-fetches.** `verify()` never touches the network, by design — that is what
makes the conformance tests offline. The revision alarm is a sha comparison an operator performs by
hand when following the note's `Fetch:`/`Verify:` lines. It is a good alarm; it is not an automatic one.
Anyone who re-fetches those two after the flip and skips the `Verify:` step gets a different document
with no complaint from the repo.

---

## 3. Three pipeline defects that will bite the TY2026 port

### 3a. The regeneration command 58 extract files name does not exist

Every generated extract carries a header. 58 of the 64 say:

```
# GENERATED — do not hand-edit. Text layer of design/forms/2025/i8283--2025.pdf
# sha256:2de015f75bcf2fa9…  |  pdftotext (no flags)
# Regenerate: cargo run -p xtask -- forms extract
```

There is no `forms` subcommand:

```
$ ./target/debug/xtask forms extract
usage: cargo run -p xtask -- <docs [--pdf] | examples | subcommand-coverage | check-isolation |
 line-coverage | cite-check | prompt-check | authority-conflicts | harness-check | archive-check |
 authority-manifest [--regen] | extract-geometry <stem> | label-census <stem> | label-proof <stem> |
 label-boxes <stem> | classify-path <path> | extract-schedule-1a | dump-fields <pdf>>
```

`extract-schedule-1a` (`crates/xtask/src/cite_check.rs:488-529`) is the closest thing, and it writes to
`crates/btctax-core/src/tax/fixtures/`, **not** to `design/forms/extract/`. Grepping the whole repo for
a writer of `design/forms/extract/` finds none. **The ② step of the pipeline — the one every conformance
instrument reads — has no committed producer.**

The recipe is not lost: each header records its own flags (`pdftotext -layout` × 32, `pdftotext (no
flags)` × 27, one explicit `pdftotext -layout crates/btctax-forms/forms/2024/f8283.pdf …`). But the
TY2026 port means running that step 30-odd times, and today it is folklore rather than a command.
`FOLLOWUPS.md:1302` propagates the phantom command into the §G-12 unblock recipe.

### 3b. The `--DRAFT` stem suffix breaks the stem→year derivation

The geometry and label pipeline derives the year by splitting the stem on the **last** `--`:

```rust
// crates/xtask/src/form_geometry.rs:186-187   (identical at crates/xtask/src/label_reader.rs:456-457)
let year = stem.rsplit("--").next().unwrap_or("2025");
let pdf  = root.join(format!("design/forms/{year}/{stem}.pdf"));
```

The repo's one TY2026 artefact is named `f6251--2026-DRAFT`, so:

```
$ ./target/debug/xtask extract-geometry f6251--2026-DRAFT
xtask extract-geometry: /scratch/code/bitcoin_tax/design/forms/2026-DRAFT/f6251--2026-DRAFT.pdf not
present. It is gitignored; re-fetch it from the URL in
/scratch/code/bitcoin_tax/design/forms/2026-DRAFT/f6251--2026-DRAFT.pdf.txt
```

Year `2026-DRAFT`, a directory that will never exist — and the message blames a *missing fetch* and
points at a note that also does not exist. Since drafts are the only TY2026 material available for the
next two months, this is squarely on the port's path.

### 3c. Nothing in the manifest distinguishes a DRAFT from a FINAL

```json
{
  "path": "design/forms/2026/f6251--2026-DRAFT.pdf",
  "kind": "form",
  "storage": "note",
  "sha256": "a547fc9d629e1f04bdc30e088214c716667b88cb1b45447c580c0ae33095b5cc",
  "bytes": 295209,
  "url": "https://www.irs.gov/pub/irs-dft/f6251--dft.pdf",
  "extract": ""
}
```

`kind` is `form`, the same as a final. Only the filename says DRAFT, and the filename is what the stem
derivation above chokes on. If a draft is ever archived under a clean stem (`f6251--2026`) to make the
tooling work, the manifest records it as authority indistinguishable from a final. The `.pdf.txt` note
carries the warning in prose — it is not a field anything can check.

Good news on the same artefact: the draft **round-trips**. Archived 2026-07-29, and today's live
`irs-dft/f6251--dft.pdf` is byte-identical (`a547fc9d629e1f04…`, all three of note/disk/live agree),
despite the note's warning that a draft may be replaced in place.

---

## 4. ★ When the IRS publishes — measured from the repo, not recalled

The IRS publishes **no** finalisation schedule. I checked: `https://www.irs.gov/draft-tax-forms` shows a
"Posted Date" column for drafts (e.g. Form 1098-VLI posted 09/04/2026) and says only *"Do not file draft
forms"* — no draft→final date anywhere. So the honest basis is the archive's own two years, read from
PDF `CreationDate` metadata (periodic forms excluded, since they have no TY edition):

| | first | p25 | p50 | p75 | last |
|---|---|---|---|---|---|
| **TY2024** (n=30) | 2024-09-23 `i8995` | 2024-11-12 | **2024-12-03** | 2024-12-16 | 2025-02-27 `i1040sd` |
| **TY2025** (n=29) | 2025-08-07 `i8959` | 2025-12-08 | **2026-01-02** | 2026-01-08 | 2026-02-25 `i1040gi` |

Forms alone (the 16 `f*` stems): TY2024 ran **2024-10-22 → 2024-12-27**; TY2025 ran **2025-11-03 →
2026-01-13**.

**TY2025 ran about a month later than TY2024** at every percentile (p50 slipped 30 days), consistent
with the Pub. L. 119-21 restructuring. Two years is a weak base and I will not pretend otherwise, but
the shape is stable: forms finalise Oct–Jan, instructions trail, and the general instructions
`i1040gi` are last or nearly last both years (2025-01-03 for TY2024, **2026-02-25** for TY2025).

**Projection for TY2026, stated as a range because that is what two data points support:**

- first TY2026 form final on `irs-prior`: **late Oct – mid Nov 2026**
- half the annual set available: **mid-Dec 2026 – early Jan 2027**
- complete, `i1040gi--2026` included: **Jan – Feb 2027**

One confirmed mechanism that helps: **the year-pinned `irs-prior/<stem>--<TY>.pdf` URL is live at
publication, not only after supersession.** `irs-prior/f1040--2025.pdf` is 200 today and byte-identical
to what `irs-pdf/f1040.pdf` serves (`3d31c226df0d189c…`). So the archive can use the stable URL for
TY2026 from the first final onward; no moving URL is needed for any annual form.

---

## 5. ★ What exists for TY2026 TODAY

Probed all 36 stems the archive uses:

- **`irs-prior/<stem>--2026.pdf`: 404 on all 36.** No TY2026 final exists in any form.
- **`irs-dft/<stem>--dft.pdf`: 200 on all 36** — but a 200 proves nothing about the year, so I read
  page 2 of each (page 1 is the IRS draft cover sheet).

**TY2026 drafts that genuinely print 2026 — 20 of the 30 annual documents:**

| forms (16 of 17) | `f1040s1` `f1040s1a` `f1040s2` `f1040s3` `f1040sa` `f1040sb` `f1040sc` `f1040sd` `f1040sse` `f6251` `f8615` `f8949` `f8959` `f8960` `f8995` `f8995a` |
|---|---|
| **instructions (4 of 13)** | `i1040sb` `i1040sse` `i8615` `i8959` |

**Still serving a TY2025 (or older) draft — 10 of 30:**

| | |
|---|---|
| **`f1040` itself** | `irs-dft/f1040--dft.pdf` prints **2025**; its ModDate is **2025-09-08**. The TY2026 Form 1040 draft has not been posted. |
| instructions (9) | `i1040gi` `i1040sc` `i1040sca` `i1040sd` `i6251` `i8949` `i8960` `i8995` `i8995a` |

The six periodic documents are, as expected, not year-scoped: the drafts are the current revisions
(`f8275`/`i8275` Rev. October 2024; `f8275r`/`i8275r` Rev. November 2024; `f8283`/`i8283` Rev. December
2025).

**When the TY2026 drafts appeared** (PDF `CreationDate` → `ModDate`), which is the leading indicator for
the finals:

```
f1040sd    2026-04-01 → 2026-05-15      f6251      2026-05-26 → 2026-05-28
f8949      2026-04-01 → 2026-05-26      f8959      2026-05-27 → 2026-05-29
f1040sb    2026-04-07 → 2026-05-20      f8960      2026-06-01 → 2026-06-01
f1040s1    2026-04-24 → 2026-06-05      f1040s1a   2026-06-16 → 2026-09-04   ← modified YESTERDAY
f1040s2    2026-04-27 → 2026-06-04      i8615      2026-08-03 → 2026-08-04
f1040s3    2026-04-27 → 2026-06-05      i1040sse   2026-08-06 → 2026-08-14
f1040sse   2026-04-27 → 2026-05-21      i1040sb    2026-08-10 → 2026-08-12
f8995      2026-05-01 → 2026-05-29      i8959      2026-08-26 → 2026-08-28
f8615      2026-05-07 → 2026-06-30
f1040sa    2026-05-12 → 2026-06-05
f1040sc    2026-05-15 → 2026-05-28
f8995a     2026-05-26 → 2026-05-29
```

Schedule 1-A's TY2026 draft was revised **2026-09-04** — the form the current cycle is building is still
moving at the IRS.

---

## 6. The TY2026 target set, derived rather than hand-listed

TY2025 archives **31** documents (27 annual + 4 periodic). Adding the three §1 gaps plus the two
already-in-repo periodic 8275 documents gives a complete year of **36**, which is exactly the stem set
swept above. TY2026 is the same 36 stems (assuming no new form; a new schedule is possible — Schedule
1-A itself did not exist before TY2025):

- **30 annual**: 17 forms `f1040 f1040s1 f1040s1a f1040s2 f1040s3 f1040sa f1040sb f1040sc f1040sd
  f1040sse f6251 f8615 f8949 f8959 f8960 f8995 f8995a`; 13 instructions `i1040gi i1040sb i1040sc
  i1040sca i1040sd i1040sse i6251 i8615 i8949 i8959 i8960 i8995 i8995a`.
- **6 periodic**: `f8275 i8275 f8275r i8275r f8283 i8283` — archive once per revision, not once per year
  (see §1b).

**Structurally the archive is already ready for 2026.** `design/forms/2026/` exists and sits inside the
`design/forms` KNOWN_ARCHIVE, so the A3 write hook admits it:

```
$ ./target/debug/xtask classify-path design/forms/2026/f1040--2026.pdf
ok — inside accounted-for tree design/forms/
$ ./target/debug/xtask classify-path design/forms/2026/f1040--2026.pdf.txt
ok — inside accounted-for tree design/forms/
```

`extract-geometry` and `label-census` derive the year from the stem, so `f1040s1a--2026` needs **no code
change** — given a clean stem (§3b).

**One operational trap in the regen path.** `regen()` refuses if any currently-listed document's binary
is absent (`crates/xtask/src/authority_manifest.rs:657-686`) — deliberately, and it is
mutation-verified. Consequence: `authority-manifest --regen` can only be run in a tree where **all 64**
gitignored PDFs have been fetched. A fresh clone cannot regenerate the manifest. That is correct
behaviour, but whoever does the TY2026 port needs the full archive on disk first, not just the new year.

---

## CAN BE DONE TODAY — no dependency on the IRS

1. **Fetch and archive `f1040s1--2025`, `f8995a--2025`, `i8995a--2025`.** URLs, sizes, sha256 and
   verified face-year are in §1a. Closes the TY2025 authority archive to within the 8275 question.
2. **Decide the periodic-alias question for `f8275`/`i8275`** (§1b). The bytes are already in the repo;
   the blocker is `DUPLICATE_SOURCE_GROUPS = 0`. Two honest options — teach `duplicates()` the
   same-tree/same-stem/different-year alias with a B1 planted-defect test, or leave the document
   archived once under 2024 and record its year-independence in the note. Either is a decision, not a
   fetch. Doing this now also disarms the same trap for the other four periodic documents at TY2026.
3. **Land a real `xtask forms extract`** (or correct the 58 headers and `FOLLOWUPS.md:1302` to name the
   command that actually exists). Per-file `pdftotext` flags are already recorded in each header, so the
   table is already written; this is wiring, not discovery.
4. **Fix the stem→year derivation** at `form_geometry.rs:186` and `label_reader.rs:456` so a suffixed
   stem is either handled or rejected with a message that names the real problem. Today it reports a
   missing fetch.
5. **Give the manifest a draft/final discriminator.** A `draft: true` field (or a `Kind::FormDraft`)
   makes "this line came from a draft" checkable instead of prose in a note.
6. **Archive the 20 available TY2026 drafts now** if the owner wants TY2026 mapping to start early. The
   convention exists (`design/forms/2026/`, note with the replaced-in-place warning, manifest entry) and
   the one existing draft round-trips byte-exactly. Do 3b and 3c first, or the drafts are unusable by
   the geometry pipeline and indistinguishable from finals in the manifest.
7. **Fix the TY2026 fetch list** — the 36 stems in §6, derived from the archive rather than recalled.
8. **Write the TY2026 fetch script now.** Every URL is mechanical: `irs-prior/<stem>--2026.pdf` for the
   30 annual documents, `irs-pdf/<stem>.pdf` for the 6 periodic. The script can be committed, tested
   against TY2025 (where every URL is live and every hash is known), and left to run against 2026 as
   each form lands. Nothing about it needs the IRS to publish first.

## MUST WAIT — and on exactly what

1. **Every TY2026 final.** `irs-prior/<stem>--2026.pdf` is **404 on all 36 stems** today. There is no
   TY2026 authority to archive, and a draft is not authority. Earliest first final ≈ **late Oct – mid
   Nov 2026**; half the set ≈ **mid-Dec 2026 – early Jan 2027** (§4).
2. **`i1040gi--2026`** — the general instructions carry Schedule 1-A, Schedule 2 and Schedule 3, and were
   **last** both observed years: 2025-01-03 for TY2024, **2026-02-25** for TY2025. Expect **Jan–Feb
   2027**. Anything gated on Schedule 1-A / 2 / 3 instruction text for TY2026 waits on this one document.
3. **The TY2026 Form 1040 itself** — no draft yet; `irs-dft/f1040--dft.pdf` still prints 2025 (ModDate
   2025-09-08). Every other headline form has a TY2026 draft; the 1040 does not.
4. **Nine of thirteen TY2026 instruction drafts** (`i1040gi i1040sc i1040sca i1040sd i6251 i8949 i8960
   i8995 i8995a`) — still serving TY2025. On the TY2025 draft dates these landed Nov 2025 – Feb 2026,
   i.e. expect **Nov 2026 – Feb 2027**.
5. **Whether the six periodic forms get revised for TY2026.** Unknowable now; it decides whether the
   alias mechanism of §1b is needed once or six times. Cheap to re-probe monthly: fetch
   `irs-pdf/<stem>.pdf` and compare to the pinned sha.
6. **Whether TY2026 adds a form btctax does not yet emit.** Schedule 1-A appeared for TY2025 under
   Pub. L. 119-21; the archive cannot enumerate a form that does not exist. The drafts are the early
   warning, and reading them is item 6 of "today".

---

## Not my lane, flagged for whoever owns it

- **`EMITTED_FORMS` (`crates/xtask/src/cite_check.rs:678-681`) lists 16 stems and omits `f8995a`,**
  which `crates/btctax-forms/src/form8995a.rs` emits and `crates/btctax-forms/src/packet.rs:192` adds to
  the packet. The authority ratchet `authority_coverage_may_only_improve` is keyed on `EMITTED_FORMS`, so
  it cannot notice a missing authority for a form it does not know is emitted. `f8615` may be the same
  shape (`design/ty2025/SPEC_form8615_kiddie_tax.md` exists; no `form8615.rs` in the forms crate).
  → *code year-seams / AcroForm map lenses.*
- `crates/xtask/src/prompt_check.rs` hardcodes `design/forms/extract/i1040gi--2025.txt` and
  `i8615--2025.txt` in 8 places. → *code year-seams lens.*

---

## Method note

Every URL status, byte count, sha256 and printed year in this report was measured today with `curl`,
`sha256sum`, `pdftotext` and `pdfinfo`; every code claim was read from the file at the cited line or
produced by running `./target/debug/xtask`. Nothing here is quoted from a doc comment or an earlier
report without independent confirmation — including the `f8275` byte-identity, which
`design/ty2025/recon-year-port-delta.md:46-48` had already found and which I re-derived rather than
carried forward. The one thing I could not establish is an IRS-published finalisation schedule: it does
not exist, so §4 substitutes the archive's own measured cadence and says so.
