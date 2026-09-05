# Recon: How a US federal individual return actually gets filed electronically, and what `btctax` would need

**Date:** 2026-09-04. **Type:** recon only, no code/repo changes.

**Framing note on "current season."** Today is 2026-09-04. Filing season 2026 (returns for **tax year 2025**) opened 2026-01-26 and the regular deadline was **Wednesday, April 15, 2026** — both already past. What is *live right now* is the back half of that same season: taxpayers who filed Form 4868 have until **October 15, 2026** to file their TY2025 return ([IRS: Oct. 15 deadline reminder](https://www.irs.gov/newsroom/irs-reminds-taxpayers-who-filed-for-extensions-of-the-oct-15-deadline)). The next filing season (TY2026 returns) doesn't open until ~January 2027. Every claim below is dated to whichever of these it describes; e-file mechanics (MeF, EFIN/ETIN, PIN signature) don't change season to season, but **channel availability does** — see §1.

---

## 1. Channels available to an individual filer

| Channel | Who's eligible | Cost | Status as of 2026-09-04 |
|---|---|---|---|
| **IRS Free File** (guided software, 8 partner companies) | AGI ≤ **$89,000** for 2025 (the threshold that applies to returns filed in the 2026 season) | Free (federal) | Live — [IRS: 2026 tax filing season opens with several free filing options](https://www.irs.gov/newsroom/2026-tax-filing-season-opens-with-several-free-filing-options-available) |
| **Free File Fillable Forms (FFFF)** | **No income limit** — anyone comfortable doing their own math | Free | Live, opened 2026-01-26 alongside the season. Manual entry only: *"This program does not allow you to attach any documents to your return, except those available through the program."* Supports Form 8949 + Schedule D for TY2025. No state returns, no prior-year returns, no import/upload API. — [IRS: FFFF program limitations and available forms](https://www.irs.gov/e-file-providers/free-file-fillable-forms-program-limitations-and-available-forms) |
| **IRS Direct File** | Was: 25 states, W-2/1099-INT/1099-R/SSA-1099/unemployment income, standard deduction only — **never supported capital gains, business income, rental income, or digital-asset dispositions** (no Schedule D/Form 8949 path existed even when live) | Free | **Discontinued.** IRS product manager Cindy Noe told state revenue departments in writing, Nov. 2025: *"IRS Direct File will not be available in Filing Season 2026. No launch date has been set for the future."* The irs.gov Direct File portal itself now shows: *"Direct File is closed. More information will be available at a later date."* Treasury Secretary Bessent: *"I think that we have better alternatives, it wasn't used very much, and we think the private sector can do a better job."* Program cost was cited at ~$41M for TY2024 (~$138/return). — [Federal News Network](https://federalnewsnetwork.com/it-modernization/2025/11/irs-direct-file-will-not-be-available-in-2026-agency-tells-states/), [Tax Notes: IRS Shutters Direct File](https://www.taxnotes.com/featured-news/irs-shutters-direct-file-citing-cost-and-low-uptake/2025/11/05/7t7q0) |
| **Commercial tax software** (TurboTax, H&R Block, FreeTaxUSA, etc.) | Anyone; income limits only apply to their own "free tier" | Varies; often free for simple returns, paid for Schedule D/crypto | Live, dominant channel |
| **Paid preparer (CPA/EA/unenrolled preparer)** | Anyone | Preparer's fee | Live |

**Bottom line for digital-asset/crypto filers specifically:** with Direct File gone and never having covered capital gains anyway, and FFFF being real but 100%-manual (no way to bulk-load Form 8949 rows), a filer with meaningful on-chain activity has exactly three practical channels — commercial software, a paid preparer, or hand-typing into FFFF/paper. This is the gap `btctax` sits next to.

---

## 2. Modernized e-File (MeF): what it is, and who may transmit

**What it is.** MeF is the IRS's XML-based system for receiving e-filed returns (it replaced the older "legacy" fixed-format e-file system). For individual returns it currently covers Forms 1040, 1040-NR, 1040-SS, 4868, 2350, 9465, and 56, with **XML Schemas and Business Rules published per tax year** (TY2023 through TY2026 are live now) for "Software Developers and Transmitters who are interested in developing software for the Modernized e-File Individual Tax Return Program." — [IRS: MeF schema and business rules for individual returns](https://www.irs.gov/node/4050)

**Who may transmit — this is gated, not open.** Only an **Authorized IRS e-file Provider** may submit return data into MeF. Per Publication 3112 (Rev. 11-2025), verbatim:

> "An Authorized IRS e-file Provider (Provider) is a business or organization authorized by the IRS to participate in IRS e-file. It may be a sole proprietorship, partnership, corporation or other entity. The firm submits an e-file application, meets the eligibility criteria and must pass a suitability check before the IRS assigns an Electronic Filing Identification Number (EFIN)."

There are seven **Provider Options**, not mutually exclusive, chosen on one application (Pub 3112 pp.4–5):

- **Electronic Return Originator (ERO)** — "begins the process of the electronic submission of tax returns to the IRS."
- **Intermediate Service Provider** — processes return data and forwards it to a transmitter.
- **Software Developer** — "writes either origination or transmission software according to the IRS e-file specifications."
- **Transmitter** — "sends the electronic return data directly to the IRS. A Transmitter must have software and computers that allow it to interface with the IRS."
- **Online Provider** — "allows taxpayers to self-prepare returns by entering return data directly on commercially available software... or through an online Internet site. Online Provider is a **secondary role**; therefore, the business must also choose another Provider Option such as Software Developer, Transmitter or Intermediate Service Provider." (This is the exact category TurboTax Online, FreeTaxUSA, etc. hold.)
- **Reporting Agent** — payroll-service companies filing employment tax returns for clients (not relevant to individual 1040s).
- **Large Taxpayer** — an entity with ≥$10M assets (or a 100+ partner partnership) that "originates the electronic submission of its own return(s)"; explicitly **not** relevant to an individual's personal Form 1040, and Pub 3112 notes a Large Taxpayer "is not an Authorized IRS e-file Provider" in the full sense.

Source: [Publication 3112, IRS e-file Application & Participation (Rev. 11-2025)](https://www.irs.gov/pub/irs-pdf/p3112.pdf), pp. 2–5.

**EFIN vs. ETIN, precisely** (Pub 3112 p.9, verbatim):

> "The IRS assigns Electronic Filing Identification Numbers (EFINs) to **all Providers** and assigns Electronic Transmission Identification Numbers (ETINs) to **Transmitters, Software Developers and Online Providers**. The IRS assigns EFINs with prefix codes 10, 21, 32, 44 and 53 to Online Providers."

So: EFIN = the firm's general e-file participant ID (every Provider role gets one). ETIN = a transmission-specific ID layered on top, held only by the roles that actually push bytes to IRS systems or write the software that does. "All Providers must include their identification numbers with the electronic return data of all returns the Provider transmits."

**Can an individual transmit their OWN return programmatically?** There is no dedicated "just my own 1040" registration tier. The application process (below) is the same one a commercial preparer uses — a sole proprietorship is an allowed applicant type, but the applicant still goes through the full suitability check, and if they also write the transmission code they must pass Assurance Testing System (ATS) — the same bar a company like Intuit clears. Nothing in Pub 3112 forbids a natural person from doing this as a sole proprietor; nothing in it makes it lighter-weight for personal-use-only either. In practice this is why no consumer product exists that lets a user "e-file straight from their own laptop with no intermediary" — the intermediary requirement is structural, not a market gap.

---

## 3. Signature: Self-Select PIN vs. Practitioner PIN vs. Form 8453 vs. Form 8879

**What a self-preparer actually uses: the Self-Select PIN.** Per [IRS: Self-Select PIN method for Forms 1040 and 4868 (MeF)](https://www.irs.gov/e-file-providers/self-select-pin-method-for-forms-1040-and-4868-modernized-e-file-mef):

- The PIN is "any five numbers (except all zeros) the taxpayer chooses to enter as their electronic signature."
- **Identity proofing** is prior-year-based, not document-based: the taxpayer supplies "date of birth and Adjusted Gross Income (AGI) amount **or** the self-select PIN from the original prior year tax return." First-time filers enter `0` for prior-year AGI (never leave blank). For an amended return, use the *originally filed* AGI/PIN, not the amended figures.
- A taxpayer with an **IP PIN** (Identity Protection PIN, for confirmed identity-theft victims or opt-in enrollees) must additionally supply that 6-digit number; it is a separate, annually-reissued anti-fraud credential, not a substitute for the Self-Select PIN.
- Ineligible: primary filers under 16 who've never filed, and under-16 spouses who didn't file the immediately prior year.

**Practitioner PIN** is the analogous mechanism when an ERO prepares the return: the taxpayer authorizes the ERO to enter/generate the PIN on their behalf. This is the *paid-preparer* path, not the self-preparer path.

**Form 8879 (IRS e-file Signature Authorization)** is required whenever the Practitioner PIN method is used — the taxpayer hand-signs (wet or approved e-signature) Form 8879, the ERO retains it (does **not** mail it to IRS), and the ERO then signs Part III with a PIN combining their EFIN + 5 self-selected digits.

**Form 8453** has been repurposed: it is *no longer* a signature-transmittal form. It is now used only to **mail specific required paper attachments** that can't travel inside the MeF XML (e.g., certain elections, a Form 8283 appraisal, other paper documents the return references) after an otherwise fully e-filed, fully-signed return has already been transmitted. A self-preparer signing via Self-Select PIN generally never touches Form 8453 unless their return happens to require one of those enumerated paper attachments.

**So, for a lone self-preparer using COTS software:** no wet signature ever leaves their computer. They pick a 5-digit PIN, answer the prior-year-AGI (or PIN) challenge for IRS authentication, and the software/transmitter embeds both in the XML submission. Nothing is mailed unless a specific attachment type triggers Form 8453.

---

## 4. The realistic paths for software like `btctax`

Four options, in ascending order of what they cost `btctax` to build/maintain:

**(a) Produce a paper return for mailing.** This is what `btctax` already does (fills the official AcroForm PDFs). Zero e-file infrastructure required, zero IRS registration. Fallback path, always available (§5).

**(b) Produce a PDF/data export the filer manually re-keys into FFFF or a commercial product.** No integration is possible here beyond "the numbers are right and the user copies them" — FFFF has no import/upload mechanism (confirmed above: manual entry only, and it explicitly can't accept outside attachments except what the program itself generates). This is strictly a labor-saving step for the *human*, not a `btctax`-to-IRS pipeline. Realistic today with no new legal exposure.

**(c) Generate MeF-conformant XML.** Technically approachable — the schemas are public per-tax-year XSDs (§2) — but XML generation alone doesn't get a return filed. Someone with an EFIN/ETIN still has to transmit it. Unless `btctax` becomes that someone (path d), this XML has nowhere to go: it isn't a file format any existing consumer product will ingest from a third party (each commercial e-file product validates/re-derives the return itself; there's no "bring your own XML" door in TurboTax or FFFF).

**(d) `btctax` itself becomes an Authorized IRS e-file Provider (Online Provider + Software Developer + Transmitter).** This is the only path that produces genuine "file straight from the CLI" behavior, and it is a real undertaking, not a form to fill out once:

1. **Registration** — one online e-file Application via e-Services; choose Provider Options (need at least Software Developer + Transmitter, plus Online Provider as the secondary role since it self-describes as letting taxpayers self-prepare). No IRS fee. Up to **45 days** to process. — [Pub 3112](https://www.irs.gov/pub/irs-pdf/p3112.pdf) p.3
2. **Suitability check** on every Principal/Responsible Official: US citizenship or lawful permanent residency, age ≥18, state licensing/bonding compliance, and (absent an Attorney/CPA/EA/officer-of-public-company/bonded-banker credential) **IRS-mandated Livescan fingerprinting** through an IRS-authorized vendor — currently no charge. Checks run: tax compliance, credit, criminal background, prior e-file compliance. — Pub 3112 pp.5–6, [IRS EFIN FAQ](https://www.irs.gov/e-file-providers/faqs-about-electronic-filing-identification-numbers-efin)
3. **Assurance Testing System (ATS)** — before being allowed to transmit "live," the software must pass IRS schema validation + business-rule validation against IRS-published test scenarios (Publication 1436 has the individual-return test returns); software developers/transmitters "must complete testing before acceptance." — [IRS MeF ATS](https://www.irs.gov/e-file-providers/modernized-e-file-mef-assurance-testing-system-ats), Pub 3112 p.8. This repeats **every tax year** as forms/schemas change.
4. **Publication 1345 Online Provider security standards** — because "Online Provider" is the role that matters here, `btctax` would inherit a specific extra compliance layer on top of general Provider rules: a written privacy/safeguard policy with IRS-mandated language, bot-challenge protection (CAPTCHA) on any web-facing intake, a **current Extended Validation SSL certificate** (TLS 1.2+, ≥2048-bit RSA/128-bit AES), and third-party **privacy-seal certification** by an IRS-acceptable vendor. This is written for a browser-based product; how a *CLI* tool satisfies "website" requirements (EV-SSL, CAPTCHA) is not addressed anywhere in the IRS guidance — because every existing Online Provider is a web app. This is a real, unresolved design question, not a checkbox.
5. **Ongoing Provider obligations** — Pub 3112's "Monitoring," "Continuous Suitability," "Revocation," and "Sanctioning" sections make clear this is a standing regulatory relationship, not a one-time approval: the IRS can conduct site visits, re-run suitability annually, and sanction/suspend/expel for violations of Pub 1345 or Pub 3112.
6. **EFIN + ETIN issuance** on acceptance; `btctax`'s transmissions must carry an EFIN with an Online-Provider prefix code (10/21/32/44/53) plus its ETIN in every submission.

This is exactly the regime TurboTax/FreeTaxUSA/etc. operate under. It is *legally open* to any qualifying firm (including, per Pub 3112, a sole proprietorship) — there's no rule saying "must be a big company" — but it is a genuine ongoing compliance program (annual re-certification testing, security audit, suitability monitoring), not a one-time integration task. **Separately, IRS Free File Alliance membership** (the 8-partner consortium behind the guided-software free tier) is a distinct, closed arrangement — a negotiated agreement with the IRS, not a form anyone can file — and isn't a realistic on-ramp for a niche crypto-tax CLI.

**Practical recommendation implied by this recon (not a decision — the user's to make):** (a)+(b) are available today with no new registration; (d) is the only route to genuine "one command files my return," and its cost center is *recurring* (yearly ATS re-testing, security posture, suitability monitoring) more than the initial 45-day/no-fee application.

---

## 5. Paper filing (the fallback `btctax` already targets)

- **Mailing address depends on two things:** the filer's state, and whether a payment is enclosed. IRS processes paper returns at six centers (Ogden UT, Kansas City MO, Louisville KY, Cincinnati OH, Austin TX, Charlotte NC), and the authoritative per-state/per-form table (1040, 1040-SR, 1040-ES, 1040-V, 1040-X, 4868 — each address further split by "enclosing a payment" vs. not) is: [IRS: Where to file paper tax returns with or without a payment](https://www.irs.gov/filing/where-to-file-paper-tax-returns-with-or-without-a-payment). This is a page `btctax` would need to keep in sync (or link to) since it is genuinely state-dependent and the IRS updates it periodically (there was a mid-2026 correction to the related 1040-ES addresses — [IRS correction notice](https://www.irs.gov/forms-pubs/correction-to-the-mailing-addresses-in-the-2026-form-1040-es)).
- **What must be physically attached:** W-2s and any 1099s showing federal withholding go on the front per the form's own attachment area; every supporting schedule/form (Schedule D, Form 8949, etc.) is assembled in the IRS's numbered attachment sequence, with a form's own supporting statements placed immediately behind it.
- **Deadlines, TY2025 (the season now in its back half as of 2026-09-04):** regular due date was **April 15, 2026**; with a timely-filed Form 4868, the extended due date is **October 15, 2026** — six weeks from today. Extension buys filing time only; tax owed was still due April 15. — [IRS: Oct. 15 deadline reminder](https://www.irs.gov/newsroom/irs-reminds-taxpayers-who-filed-for-extensions-of-the-oct-15-deadline)
- **Postmark rule:** a paper return is timely if postmarked by the due date regardless of IRS receipt date — but note the Taxpayer Advocate flagged a **2026 USPS postmark-practice change** that filers should be aware of when cutting it close by mail: [NTA blog, April 2026](https://www.taxpayeradvocate.irs.gov/news/nta-blog/new-u-s-postal-service-rules-could-affect-whether-your-tax-filing-is-considered-on-time/2026/04/).

---

## 6. What would make "file your own return from a CLI" impossible — stated plainly

Nothing makes it **legally** impossible in principle. But several things make it **practically** very hard for a project `btctax`'s size to reach "click a button, IRS has your return" without an intermediary:

1. **Transmission is gated by design, not by technology.** MeF's XML schemas are public, but *submitting* to MeF requires holding an EFIN/ETIN as an Authorized Provider. There is no "anyone with valid XML and a network connection may submit" door — this is the single hardest fact in this recon. A CLI that "generates MeF XML" still cannot file anything by itself; it can only hand that XML to whoever holds the credentials.
2. **The Online Provider security profile is written for a website**, not a CLI or a locally-run binary — EV-SSL certificates, bot-challenge (CAPTCHA) protection, and a privacy-seal audit all presuppose a hosted web front end. `btctax` (a local Rust binary that fills PDFs) would need to either build a hosted service to satisfy this, or the IRS's category simply doesn't have a clean answer for "local software that also self-transmits." This is a genuine gap in the published rules, not a solved problem elsewhere.
3. **Compliance is recurring, not one-time.** ATS re-testing every tax year as forms/schemas change, continuous suitability monitoring, and sanctioning exposure make this an ongoing regulatory relationship for as long as `btctax` wants to keep transmitting — closer to "become a small e-file company" than "add an export format."
4. **No lightweight "individual, personal-return-only" tier exists.** The "Large Taxpayer" self-filing category exists but is explicitly for $10M+ asset entities, not people. A person wanting only to submit their own 1040 must go through the identical Provider application, suitability check, and (if they wrote the software) ATS testing that a commercial preparer does — the law doesn't distinguish "just for me" from "for my clients."
5. **Nothing here is Bitcoin/crypto-specific.** All of the above binds any 1040 e-filer equally; digital-asset content changes *what* goes on Schedule D/Form 8949, not *how* a return reaches the IRS. Separately, note Direct File's exclusion of capital gains (§1) was a **product design/funding choice**, not a legal barrier — MeF and paper filing have always accepted Schedule D/Form 8949 including digital-asset transactions (Form 8949 boxes G/H/I short-term and J/K/L long-term are the digital-asset checkboxes per the [2025 Schedule D instructions](https://www.irs.gov/instructions/i1040sd)).

**Conclusion:** the legally-clean, buildable-today paths for `btctax` are (a) paper-PDF-for-mailing (already built) and (b) PDF/figures for manual re-entry into FFFF or a commercial product. Becoming a full MeF Online Provider/Transmitter (path d, §4) is legally open to `btctax` as a firm but is a standing compliance program — annual re-certification, a security posture the IRS's rules assume is a website, and continuous suitability exposure — not a feature to ship in a release.
