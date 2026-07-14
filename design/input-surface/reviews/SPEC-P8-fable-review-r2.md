# INDEPENDENT REVIEW — SPEC_input_surface.md r2 (P8), review r2

*Persisted VERBATIM before folding. Author = Opus; reviewer = Fable (fresh instance; r1 read from disk —
the r1 reviewer was killed twice by API errors mid-round-2, having said only "Two candidate Criticals
depend on precise code behavior. Let me verify both.").*

**VERDICT: 2 Critical / 3 Important / 4 Minor / 1 Nit**

---

**The two candidate Criticals: both confirmed, one as framed, one found in the fold.** (A) the claimed-as-dependent flag is real and worse than framed — it suppresses **two** understatement guards, not one. (B) the r2 fold introduced a new defect, as it has every round: the D-2/KAT-C combination force-answers the fail-loud tri-states, and D-7's new `--keep-identity` rebuilds the very sin D-7 bullet 2 exists to block.

Answers to the directed questions, then findings.

- **D-6 trade (names/address/DOB in plaintext):** right, and the "fourth option" (vault DOB whose absence refuses) is worse — it would wall the no-PII household out of the report, contradicting the codebase's own contract ("a household that has entered no PII still gets a report"), and the DOB-absent forfeit is already conservative-direction *and* advised loudly (`advisories.rs:113,214`). The real DOB defect is the placeholder collision (Finding 2), not the location.
- **D-5 clearing:** a *wrong* secret is fixed by overwrite (`set-pii` re-prompt) — sufficient. Rule 2's "empty ⇒ preserve" even normalizes a template's `ip_pin = ""` (`Some("")`) back to the vault's `None`. Two genuine gaps: collection identity (Finding 3) and an un-exitable `Some`→`None` for an IP PIN entered in error (Finding 8).
- **`RefusalKind`:** sound, not over-engineering — purely message-shaping (the spec says so), and a 3-variant exhaustive match is ~30 cheap lines that prevent a real failure (a future variant shipping the wrong recovery message). One scoping gap (Finding 6).
- **Drift alarm:** the three-KAT construction is mechanically airtight for *fields* — but KAT C's completeness demand is itself the engine of Finding 2, and enum-*variant* drift is un-alarmed (Finding 9).

---

### [CRITICAL] §9's dependent-filer guarantee is claimed but not delivered — the flag stays a silent-default `bool` that suppresses BOTH understatement guards

**Where:** §5 D-6, §9 bullet 3; `return_inputs.rs:164–165`, `return_1040.rs:78`, `return_1040.rs:610–628`.
**What:** §9 promises "a claimable-as-dependent filer cannot silently receive the full standard deduction," but D-6's only mechanism is template visibility. The field remains `bool` + `#[serde(default)]`: any file that omits the key (a stripped template, a hand-rolled TOML, every pre-P8 stored blob) parses to `false`, screens clean, and grants the full standard deduction (`return_1040.rs:78`). Verified worse: the Form 8615 kiddie-tax screen is keyed on the **same flag** (`return_1040.rs:618` — `if ri.header.can_be_claimed_as_dependent_taxpayer`), so the silent `false` also disarms the kiddie refusal. No advisory backstops it (zero hits in `advisories.rs`).
**Why:** A student with crypto gains claimed on a parent's return — close to the archetypal btctax user — gets `$14,600` instead of the §63(c)(5) floor *and* child-rate tax instead of Form 8615's refusal: a compounded silent **understatement**, the one direction the project's refuse architecture exists to prevent, in shipped code, on the surface P8 owns.
**Evidence:** contrast the project's own idiom for unguessable answers: `mfs_spouse_itemizes`, `foreign_accounts`, `foreign_trust` are all `Option<bool>` with "None ⇒ fail-loud" (`return_inputs.rs:365–383`). The 1040 asks this question of every filer.
**Fix (in P8 scope — it is D-6/§9's own subject matter):** make both claimed-flags `Option<bool>`; `None` fires a new UNANSWERED refusal in `screen_inputs`. Back-compat is free (stored blobs serialized `false` → `Some(false)`). Template treatment per Finding 2 — an uncommented `= false` example would just pre-answer it. Then §9's sentence is true.

### [CRITICAL] Fold-introduced: D-2's "every field present" + KAT C force the template to PRE-ANSWER the fail-loud tri-states — an ungiven "No" prints on filed Schedule B Part III

**Where:** §5 D-2 ("Every field present"), §6 KAT C; `printed.rs:936`, `return_refuse.rs:507–513`, `advisories.rs:214`.
**What:** KAT C requires every fixture key-path present in the raw template *parsed as `toml::Value`* — comments are invisible — so `foreign_accounts`, `foreign_trust`, and `mfs_spouse_itemizes` must ship **uncommented with values**. A bulk-filling user who skims imports `foreign_accounts = false`, the `ScheduleBPart3Unanswered` guard never fires, and the filed Schedule B Part III 7a prints "No" (`printed.rs:936` — `ri.foreign_accounts.unwrap_or(false)`): a foreign-account disclosure answer the user never gave, on the question with FBAR-grade stakes. The same collision forces a **plausible dummy DOB** (`Option<Date>` has no refusable-but-parseable placeholder, and TOML has no null): a recent dummy silently suppresses the §63(f) forfeit advisory (`advisories.rs:214` fires only on `is_none()`); an old dummy *grants* the aged add-on — an understatement. `ip_pin = ""` is also forced into the template that D-6 says never carries the crown jewels.
**Why:** the spec's completeness machinery structurally revokes the codebase's delivered fail-loud guarantee for exactly the ask-the-user fields it was built for, and produces a wrong filed disclosure on the primary documented workflow. This is the same disease as Finding 1 through the other door — visibility-with-a-pre-filled-answer is a guess wearing documentation's clothes.
**Fix:** define an **ask-the-user field class** (the three tri-states, both claimed-flags post-Finding-1, `date_of_birth`, `ip_pin`): they ship **commented** with a "you must answer / delete if N/A" note, and KAT C carries an explicit exemption list *asserted inside the KAT* (so the exemption set is itself tested, and a raw-text grep KAT can still require the commented doc lines to exist). D-2 gains the rule; §6 gains the exemption.

### [IMPORTANT] D-5's field-level merge has no identity rule for collection elements or absent parents — index-merge misbinds dependents' SSNs on the filed return

**Where:** §5 D-5 rules 1–2, §7 step 3 KATs; `return_inputs.rs:139–146`.
**What:** the merge is specified per-leaf but `dependents` is a `Vec`: the template exemplar carries `ssn = ""` per dependent, so every re-import merges vault SSNs into file dependents — by what key? Index-merge silently attaches dependent A's vault SSN to dependent B after a reorder or an inserted newborn: a **wrong SSN printed on the filed 1040**, undetectable by any screen. Also unspecified: the whole-`spouse`-block-removed case ("absent ⇒ preserve the vault value" read literally would resurrect a deleted spouse's SSN), and rule 1's asymmetry — `#[serde(default)]` is added to `Person` only, while `Dependent.ssn` stays a required key (`return_inputs.rs:141`), so dependents can never *omit* the secret, only empty it. None of the four KATs touches a collection.
**Why:** this is the fold's own new mechanism (r2's headline fix) shipping with its hardest case unspecified — the exact "highest-risk defect in the cycle" territory.
**Fix:** specify structure-follows-file (a parent absent in the file is absent, full stop; leaf-preservation applies only within file-present structure); dependent merge keyed by `name` with refuse-on-ambiguity (duplicate/renamed names ⇒ tell the user to run `set-pii` again); add reorder/insert/remove KATs.

### [IMPORTANT] Fold-introduced: D-7's `--keep-identity` rebuilds the "default row at precedence 1" sin that D-7 bullet 2 blocks

**Where:** §5 D-7 bullet 3; `resolve.rs` ladder (verified: `ReturnInputs` row wins and never falls through), r1 Finding 6's premise (an all-default `ReturnInputs` screens clean).
**What:** "clears the money and preserves the header" leaves a **screen-clean row** (header + zeroed money) at precedence 1. A user who hits an uncomputable year and reaches for `--keep-identity` — the flag exists precisely so recovery doesn't destroy SSNs — gets a year that now silently computes from **all-zero** non-crypto inputs, shadowing the stored `tax-profile` beneath it. That is verbatim the "two liabilities, silently different number" state bullet 2 refuses for `set-pii`, reintroduced in the *same design decision*, and it breaks §9's "no path takes a working year down" (silently-wrong is worse than down).
**Fix:** `--keep-identity` must leave the year fail-closed until re-import — e.g. the kept row stores the header but is marked incomplete and refuses at resolve with "re-import your money TOML (`income template`)". Pick the mechanism in the spec.

### [IMPORTANT] D-2 has a PII-placeholder policy but no MONEY-placeholder policy — KAT B forces exemplar rows whose leftover values import as phantom entries

**Where:** §5 D-2, §6 KAT B ("no empty array").
**What:** KAT B requires every `Vec` to carry an exemplar and KAT C forces it into the template — so the template ships a W-2, a charitable gift, a carryover item, a dependent. D-2 pins placeholder *PII* to refused values but says only "money demonstrated as a quoted string": a non-zero example (`amount = "2500"`) left unedited by the same skimming user D-2 already legislates for imports as a **phantom deduction/carryover** — understatement — with no screen able to know. (The phantom-*dependent* residue happens to fail closed: the packet refuses its empty SSN — but only at export, with no such backstop for money.)
**Fix:** one D-2 rule: every money placeholder is `"0"` (inert if unedited — "0" still demonstrates the quoted-string format), and the template's block headers instruct deleting non-applicable `[[...]]` blocks. All three KATs remain satisfiable.

### [MINOR] §3.5 misplaces `KiddieTax` among the "twenty-one input-screenable refusals" — contradicting D-4 in the same document

`KiddieTax` fires in `screen_compute_dependent` (`return_1040.rs:610–628`), as D-4 correctly lists; §3.5's UNSUPPORTED examples row includes it anyway (fold-introduced — r1's enumeration didn't have it; the count of 21 is right without it). Mechanism unharmed (`kind()` spans the whole enum; import never sees compute-dependent variants). Fix the table; while there, have §3.5 say what `kind()` means for compute-dependent/absolute variants (message templates are import-scoped — "nothing was stored" must never leak into report-time text) and record `SingleEmployerExcessSs`'s chosen box (it straddles INVALID/UNSUPPORTED).

### [MINOR] No path takes an IP PIN from `Some` to `None`

A user who entered an IP PIN they don't have (or the IRS retires) can only `income clear` (destroys all SSNs) or `--keep-identity` (keeps the bad PIN). `set-pii` validates every prompt via `IpPin::canonical` (`packet.rs:121` — empty ⇒ `Missing`), so re-prompting can't clear it. Specify a clear affordance (e.g. a sentinel or `--clear-ip-pin`).

### [MINOR] §8 punts the `income show` disposition, and the journey's last wall is unpointed

(a) "Resolve or re-defer with a reason" is a decision the spec should make, not carry open into implementation. If TOML round-trip ships, its secrets must emit as **empty strings**, not masks — a copy-forward of today's masked `***-**-6789` is non-empty, so D-5's file-wins rule stores it and `SsnMalformed` refuses every re-import. (b) Journey walk residue: template→import→report all now teach the user, but the *final* wall — packet-time `SsnError::Missing` at `export-irs-pdf` — is where a default-path user first needs `set-pii`, and nothing in the spec requires that refusal message to name it. One sentence each.

### [NIT] §6 residuals: widen the grep, record the enum-variant hole, retire "uncommented"

The convention-ban grep should match `serde(skip` generally (`skip` evades B/C exactly as `skip_serializing_if` does). Enum-**variant** drift (a new `CharitableClass`, a new allowlisted box-12 code) moves no key-path and is un-alarmed — record it beside the residual risk. KAT A's "uncommented template" wording is residue of the dead commented-PII-block design; post-Finding-2 the operative phrase is "the template with its ask-the-user lines uncommented to their example answers."

---

## Bottom line

The r2 skeleton is right — refuse-don't-store is now settled on sound ground (§3.2 verified against the resolver: a refused row genuinely never falls through), the three-KAT drift alarm is the correct construction, D-5's field-level merge and D-6's secrecy line are the right architecture. What blocks is one inherited guarantee the spec claims but does not deliver (the dependent flag — live understatement in shipped code, doubly so via the kiddie screen), and the r2 folds' own new defects: a completeness KAT that force-answers the fail-loud questions onto a filed disclosure, a merge rule without element identity, and a `--keep-identity` that resurrects the precedence-ladder sin. All fixes are spec amendments; none reverses a design decision.

**VERDICT: 2 Critical / 3 Important / 4 Minor / 1 Nit**
