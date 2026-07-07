# btctax

**An offline, single-user US Bitcoin tax ledger.**

`btctax` computes your US federal tax picture for Bitcoin — per-lot cost basis, realized
short-/long-term gains, income, gifts and donations, and the IRS forms (8949, Schedule D, 8283,
Schedule SE) — from your exchange CSVs, entirely on your own machine. It is an **event-sourced ledger**:
every fact and every decision is an append-only event in a passphrase-encrypted vault, so results are
reproducible and auditable.

- **Offline by default.** The tax binaries (`btctax`, `btctax-tui`, `btctax-tui-edit`) make **no network
  calls** — verifiably (they link no HTTP client). Your data never leaves your machine. A **separate,
  opt-in** tool, `btctax-update-prices`, is the *only* online path; you run it explicitly, and it only
  ever writes to a local price cache.
- **Encrypted at rest.** The vault (`vault.pgp`) is passphrase-encrypted (OpenPGP, pure-Rust crypto).
- **Reproducible.** The tax result is a pure function of the ledger **and** its price inputs (the bundled
  daily-close dataset, plus the optional local price cache — see [Price data](#price-data)).

> ⚠️ **This is software, not tax advice — and it does not file or prepare your taxes.** Scope is **US federal**
> and **BTC-only**. Review everything with a qualified professional before filing. Please read the
> [Disclaimer](#disclaimer).

---

## The three tools

| Binary | What it is |
|---|---|
| **`btctax`** | The CLI engine — init, import, reconcile, and compute. Everything scriptable lives here. |
| **`btctax-tui`** | A read-only terminal viewer for your holdings, disposals, income, and forms. Press **`w`** for the read-only what-if planner (hypothetical sell / harvest tax report). |
| **`btctax-tui-edit`** | An interactive terminal editor for reconciling — guided flows over the full decision surface. Press **`?`** in-app for the keymap. |
| **`btctax-update-prices`** | An **optional, online** helper that fetches newer daily BTC/USD closes into a local price cache. The **only** tool that touches the network; the three tax binaries above never do. See [Price data](#price-data). |

Every command has rich `--help` (including inline file-format examples), and there are man pages for all
three binaries — see [Getting help](#getting-help).

> **TUI keys (both viewer and editor):** the Holdings / Disposals / Income tables are now column-sortable.
> `←`/`→` (or `h`/`l`) move the column cursor, `s` sorts the focused column (toggling ascending/descending),
> and `[` / `]` change the tax year (previously `←`/`→`). In the editor, select-lots and link-transfer moved
> to **`S`** and **`L`** (freeing `s`/`l` for sorting/cursor). In the viewer, **`w`** opens the read-only
> what-if planner — type a hypothetical BTC sell amount (or toggle to Harvest and pick a target) for a live
> marginal-tax report; **`Tab`** toggles Sell/Harvest, **`Enter`** computes, **`Esc`** closes. It never
> writes the vault (same engine as [`btctax what-if`](#plan-a-sale-before-you-make-it--what-if)).

---

## Install

**Prerequisites:** [Rust](https://rustup.rs) ≥ **1.88** and a C toolchain (the bundled SQLite is compiled
from source — `cc`/clang/MSVC). Linux, macOS, and Windows are all supported and CI-tested.

From a clone of this repository:

```sh
# install all three binaries into ~/.cargo/bin
cargo install --path crates/btctax-cli       # -> btctax
cargo install --path crates/btctax-tui       # -> btctax-tui
cargo install --path crates/btctax-tui-edit  # -> btctax-tui-edit
```

Or build without installing:

```sh
cargo build --release      # binaries land in ./target/release/
```

> **crates.io:** publishing is planned — until then, install from source as above.

---

## Quickstart tutorial

This walks the canonical workflow end-to-end with a tiny synthetic dataset. Every command below has been run
verbatim. Work in a scratch directory (not a git repo, since you'll be handling tax data).

Set your passphrase once so the commands don't prompt each time (or omit this and you'll be prompted):

```sh
export BTCTAX_PASSPHRASE='choose-a-strong-passphrase'
```

### 1. Create the vault

```sh
btctax --vault ./vault.pgp init --key-backup ./vault-key-backup.asc
```

This creates the encrypted `vault.pgp` **and** an adjacent key file `vault.key` (needed to open the vault).
`--key-backup` writes a **separate** armored backup of that key — keep it somewhere safe and offline. Point
`--key-backup` at a distinct path (not `./vault.key`, which is the live sidecar).

### 2. Import an exchange CSV

Save this minimal Coinbase-format file as `coinbase.csv` (a single inbound receive of 0.05 BTC):

```csv
Transactions
User,00000000-0000-0000-0000-000000000000
ID,Timestamp,Transaction Type,Asset,Quantity Transacted,Price Currency,Price at Transaction,Subtotal,Total (inclusive of fees and/or spread),Fees and/or Spread,Notes,Sender Address,Recipient Address
RCV-1,2025-03-01 12:00:00 UTC,Receive,BTC,0.05000000,USD,84000.00,,,,,bc1qsender,
```

```sh
btctax --vault ./vault.pgp import ./coinbase.csv
```

> Keep your **real** exchange exports outside the repository (a `ReadOnly/` folder elsewhere is a good
> convention). Never commit them.

### 3. Verify — find what needs reconciling

```sh
btctax --vault ./vault.pgp verify
```

A freshly imported inbound receive has no known cost basis, so `verify` reports a **Hard blocker**
(`UnknownBasisInbound`) and **exits 1**. That's expected — it's the gate that tax computation waits on, and it
tells you exactly what to resolve next. The blocker line prints the event reference, e.g.:

```
[UnknownBasisInbound] import|coinbase|in|RCV-1 :: inbound transfer has no basis
```

### 4. Reconcile the blocker

Classify that inbound as a transfer from your own wallet (non-taxable, conservative $0 basis). Feed back the
reference from step 3 — **single-quote it**, because it contains `|` (a shell pipe):

```sh
btctax --vault ./vault.pgp reconcile classify-inbound-self-transfer 'import|coinbase|in|RCV-1'
```

Run `btctax --vault ./vault.pgp verify` again — it now exits 0.

> This is one of **24** `reconcile` subcommands (income, gifts, donations, disposals, safe-harbor lot
> selection, bulk operations, …). For the guided, discoverable way to work through everything, use the
> editor: `btctax-tui-edit --vault ./vault.pgp` (press `?` for the keymap). See `btctax reconcile --help` and
> the man pages for the full surface.

### 5. Set a tax profile, then report

Per-year tax needs your filing status and income (otherwise `report --tax-year` says "not computable"):

```sh
btctax --vault ./vault.pgp tax-profile --year 2025 \
  --filing-status single \
  --ordinary-taxable-income 80000 \
  --magi-excluding-crypto 80000 \
  --qualified-dividends 0

btctax --vault ./vault.pgp report --tax-year 2025
```

`report --tax-year 2025` prints the year's tax result and Schedule D summary. Plain `report` (or
`report --year 2025`) shows holdings and realized activity without a profile.

### 6. Export the forms

```sh
btctax --vault ./vault.pgp export-snapshot --out ./export --tax-year 2025
```

This writes a decrypted SQLite database plus CSVs — including Form 8949 and Schedule D — into `./export/`.

**Fill the official IRS PDFs** directly from the same computed data:

```sh
btctax --vault ./vault.pgp export-irs-pdf --out ./export --tax-year 2025
# or an earlier year:      export-irs-pdf --out ./export --tax-year 2024
# or a historical year:    export-irs-pdf --out ./export --tax-year 2017
# or restrict the packet: --forms f8949,schedule-d
```

Supported years: **2017**, **2024** and **2025** — btctax fills the right revision per year. On **2025**
(the 1099-DA revision) Bitcoin goes in **Box I/L** and the 1040 capital gain is **line 7a**. On **2024**
(pre-1099-DA) it is **Box C/F** ("not reported on a 1099-B"), the 8949 grid holds **14 rows/part**, the
1040 capital gain is **line 7**, and Form 8283 is **Rev. 12-2023**. **2017** is the OLD pre-TCJA packet:
still **Box C/F** (14 rows/part), but the **§B long Schedule SE**, Form 8283 **Rev. 12-2014** (no
digital-asset box — BTC uses **"j Other" + a printed note**), the 1040 capital gain on **line 13**, and
**no Digital-Asset question** (so an income-only 2017 year produces no 1040). Each money amount on the
2017 SE/1040/8283 is split into a **dollars field + a cents field**.

This writes a whole packet, each form only when it applies, populated from btctax's projection
(no capital-gains figure is recomputed):

- **`f8949.pdf` + `schedule_d.pdf`** — always. Bitcoin is filed under the year's digital-asset boxes
  (**Box I/L** on 2025, **Box C/F** on 2024/2017) — never the wrong pair. Rows beyond a part's grid (11
  in 2025, 14 in 2024/2017) paginate onto multiple copies, each with its own totals; Schedule D carries
  the grand totals (2017's Schedule D predates the QOF question, so it is omitted).
- **`schedule_se.pdf`** — when you have **business** self-employment income (mining, etc.) whose net
  earnings are **≥ the $400 floor** (below it, no SE tax is owed and the form is skipped). Line 12
  (SE tax) = **Social Security + regular Medicare only**; the 0.9% **Additional Medicare Tax** is a
  **Form 8959** item and is flagged on stderr, *not* placed on Schedule SE. Needs a stored
  `tax-profile` (filing status) for the year — a missing profile prints a NOTE, not a fabricated form.
- **`form_8283.pdf`** — when you made **BTC donations**. Fills the donee/appraiser **identity** and
  the per-donation property rows (Section A ≤ $5,000, or **Section B** > $5,000). The property-type box
  is **"k Digital assets"** on the Rev. 12-2023/2025 forms; the 2017 **Rev. 12-2014** form has no such
  box, so BTC uses **"j Other" + a printed note**. It leaves **every other party's declaration/signature blank**
  (Part III taxpayer signature, Part IV appraiser signature, Part V donee acknowledgment) — a Section
  B 8283 is **not filing-ready** until those are signed, and a donation with incomplete appraiser
  details is escalated for review. Multi-lot donations overflow onto additional copies.
- **`form_1040_capgains.pdf`** — when there is reportable capital/digital-asset activity. btctax fills
  the **capital-gain line** (**7a** in 2025, **7** in 2024, **13** in 2017, when Schedule D is active and
  line 16 ≥ 0; active-and-netted-to-zero prints `-0-`; a **net loss leaves it blank** — the §1211
  $3,000/$1,500-MFS cap on Schedule D line 21 is yours), and — on 2024/2025 — the **Digital-Asset
  question** (**Yes** iff you had any disposal, income, gift, or donation — never a "No"). The **2017
  form has no Digital-Asset question**, so an income-only 2017 year produces no 1040. The 7b checkboxes
  are left untouched, and a partial-scope notice lists exactly what was filled.
- **Verified placement.** Every written value is read back _geometrically_ against the blank form's
  own field coordinates; a mis-placed cell fails closed, so a wrong tax form is never written.
- **XFA dropped.** The engine removes the forms' XFA layer (otherwise Acrobat opens them blank) and
  sets `NeedAppearances`.
- **Estimates are watermarked.** A pseudo-reconciled ledger requires the same attestation as
  `export-snapshot` **and** stamps a diagonal `DRAFT — ESTIMATE, NOT FOR FILING` on every page.
- **Scope.** Schedule D lines **17–22** (28%-rate / unrecaptured-§1250 / QDI worksheet, incl. the
  line-21 loss limit) are out of scope and left blank — a notice is printed. Rows on an exchange
  that may carry 1099-DA broker reporting are flagged on stderr.

> ⚠️ **These files contain your unencrypted tax data and are _not_ git-ignored.** Write `--out` to a
> directory **outside** any git repository.

---

## Plan a sale before you make it — `what-if`

`what-if sell` posits a **hypothetical, non-persisted** sale and shows its **marginal** federal-tax
effect on the current-year position — the *incremental* cost of that one sale, not the whole-year figure.
It routes through the same audited tax engine as `report --tax-year`; it **writes nothing** (no event,
no side-table, no vault change) and is tax decision-support, not buy/sell/hold advice.

```sh
btctax --vault ./vault.pgp what-if sell \
  --sell 100000000 --wallet self:cold --at 2025-08-01 --price 95000
```

It reports the lots the sale would consume, the short-/long-term split, **which §1(h) LTCG bracket
(0/15/20%)** it lands in and the room to the next breakpoint, the **marginal tax** (headline), the
effective rate, the **§1212(b) carryforward** carried to next year plus this year's ordinary offset,
and the **§1411 NIIT** delta (with its sign — a loss harvest can *reduce* NIIT).

- **Price.** `--price` is USD **per whole BTC**; omit it to use the bundled daily-close FMV for `--at`
  (a future/off-dataset date then requires `--price`).
- **Method.** Omit `--method` to consume by your **standing method** (the account's in-force election),
  exactly as a real disposal would; or force `--method fifo|lifo|hifo`.
- **A loss sale** surfaces the carryforward disclosure: the current-year marginal alone does **not**
  represent the sale's value — the loss carried to next year is. The this-year ordinary offset is the
  *actual* §1211(b) delta — **$0** (not "$3,000") when you have already used the cap this year.

**Plan without a stored profile.** Supply an ad-hoc, non-persisted profile inline:

```sh
btctax --vault ./vault.pgp what-if sell --sell 50000000 --wallet self:cold --at 2025-09-01 \
  --price 95000 --filing-status single --income 120000 --magi 130000 --carryforward-in 20000
```

`--magi` **defaults to `--income`** when omitted (never $0 — a $0 MAGI would silently suppress the NIIT
disclosure); a caveat notes the assumption. With no ad-hoc flags, the stored `tax-profile` for the sale
year is used.

### Harvest — the max BTC to sell under a target

`what-if harvest` answers the inverse question: **how much can I sell such that a target still holds?**
It finds the largest N such that the target holds on the *entire prefix* `[0, N]` — safe even under a
partial fill — computed through the same audited engine, consuming lots in your **standing method**'s
order (never re-optimized). Four `--target`s:

```sh
btctax --vault ./vault.pgp what-if harvest --target zero-ltcg \
  --wallet self:cold --at 2025-08-01 --price 95000 --filing-status single --income 40000
```

- **`zero-ltcg`** — sell all the gain that fits entirely in the §1(h) **0% bracket**.
- **`fifteen-ltcg`** — stay at/under **15%** (no 20%-bracket dollars).
- **`gain=$X`** — realize **at most $X** of gain *with this sale* (e.g. `gain=$25,000`).
- **`tax=$X`** — add **at most $X** of *marginal* federal tax; **`tax=$0`** is the flagship "sell as much
  as possible while adding **zero** federal tax" harvest.

The optimizer is a **lot-edge segment walk**, not a bisection: because HIFO realizes losses first, the
marginal-tax curve is genuinely non-monotone (a loss-first dip, the §1211(b) $3,000 pin, and a §1212
carryforward-absorption plateau), so a naive bisection would land on the wrong side. Every answer is
**engine-verified** (never an unfolded number). The report reads your *whole stacked position*, so a
short-term gain lot correctly shrinks the 0% room, and it **discloses**:

- the **§1212(b) carryforward burn** (a gain that spends a carried loss for $0 current-year tax is not
  free — the carryforward is gone),
- the **§1411 NIIT kink** (a 0%/15%-bracket answer can *still* cost +3.8%), and
- the **§1211(b) plateau** on an all-loss position (only $3,000 / $1,500 MFS is deductible this year).

The ad-hoc profile (`--filing-status`/`--income`/`--magi`/`--carryforward-in`) works exactly as for
`sell`; a long-term `--carryforward-in` *expands* the harvestable-gain room. Like every what-if, it
**writes nothing**.

---

## Pseudo-reconcile — a fast, honest starting point

Reconciling a fresh import can mean resolving hundreds of blockers before anything computes. **Pseudo-reconcile
mode** gets you from *N blockers → an on-screen estimate in one command*, by filling deliberately-fictional but
reasonable **default** decisions — so you have a scaffold to correct toward the truth instead of a blank wall.

It is built to be **unmistakably a placeholder**: every fictional value is flagged `[PSEUDO]` on screen, the
defaults assume *all movement is non-taxable* (so the estimate trends toward ~zero tax — nobody's real activity
does that), and **you cannot export a form or CSV from a pseudo state without personally attesting**. Real
decisions you make always win over the defaults. Nothing fictional is ever written to your vault.

> This is exactly the "help yourself prepare, but *you* are the preparer" philosophy from the
> [Disclaimer](#disclaimer). The defaults are a starting point, not an answer.

### 1. Turn it on and see the estimate

```sh
btctax --vault ./vault.pgp reconcile pseudo on
btctax --vault ./vault.pgp verify        # blockers cleared; synthetic defaults marked [PSEUDO]
btctax --vault ./vault.pgp report --tax-year 2025   # a flagged estimate (a placeholder profile fills in)
```

While the mode is on, the engine synthesizes defaults *only where you haven't made a real decision*: unknown-basis
inbounds become **$0-basis self-transfers**, unclassified rows and import conflicts get accept-first defaults,
and a placeholder tax profile lets a per-year estimate compute. Every one of those shows up as `[PSEUDO]`.

> **Realistic reconcile defaults (behavior change).** Two fallbacks were made less punitive, since they
> better match how most people actually hold BTC:
> - **Cost-basis method defaults to HIFO** (the most commonly elected method), not FIFO — global, for
>   both real projection and the auto-reconcile estimate, wherever no per-account/global method election
>   is on file. It stays *unattested*, so you're still prompted to affirm it per exchange (HIFO requires
>   specific-identification records). An explicit `--set-forward-method fifo` election still yields FIFO.
> - **An unknown-basis inbound self-transfer now defaults to a *long-term* holding period** — the
>   acquisition date is dated **one year + one day before receipt** (most received BTC is a long-held
>   cold-storage deposit). Basis still defaults to a conservative **$0**. Both are disclosed by advisories;
>   supply the real values with `classify-inbound-self-transfer --basis <cost> --acquired <YYYY-MM-DD>`.

### 2. Correct the ones that are actually taxable

The defaults are wrong on purpose — fix the events that really were sales, income, or had a known basis. Each
real decision **supersedes** the pseudo default for that event (no conflict, no undo needed). For example, mark
a withdrawal that was really a sale, or attest the cost-basis method your broker used **per exchange account**:

```sh
# Attest the cost-basis method a specific brokerage account used (IRS 2025+ per-account rule).
# The account must already exist in your vault — it's created when you import that exchange's
# transactions; btctax lists the known accounts if you mistype one:
btctax --vault ./vault.pgp config --set-forward-method hifo --exchange exchange:coinbase:<your-account-id>
```

(A global election is just `--set-forward-method fifo` with no `--exchange`.)

### 3. Bulk-approve the defaults you accept

Where a default *is* right (e.g. an outbound transfer really was a move to your own wallet), promote it from
fiction to a **real, attested** decision — in bulk, with a preview:

```sh
btctax --vault ./vault.pgp reconcile pseudo approve --dry-run     # preview what would be promoted
btctax --vault ./vault.pgp reconcile pseudo approve --yes         # promote all remaining defaults
# or filter: --kind <type>  --wallet exchange:PROVIDER:ACCOUNT  --year 2025
```

Approved decisions are real and survive `pseudo off`.

### 4. Export requires an attestation (only while pseudo values remain)

As long as any `[PSEUDO]` value is contributing, producing a form or CSV requires you to type the exact phrase,
which the tool displays for you to type or paste:

```sh
btctax --vault ./vault.pgp export-snapshot --out ../export --tax-year 2025 \
  --attest "I attest this is true"
```

Without `--attest`, an interactive run **prompts** for the phrase; a scripted (non-terminal) run **refuses**
rather than silently exporting a draft. The `btctax-tui` viewer's `e` export is gated the same way. Once you've
replaced or approved every default so nothing is `[PSEUDO]` anymore, exports need no attestation — you've already
attested through your real decisions.

### 5. Turn it off

```sh
btctax --vault ./vault.pgp reconcile pseudo off   # reverts instantly; approved (real) decisions remain
```

> The exported files themselves are **clean** — the `[PSEUDO]` markers are on-screen only; they never appear in
> a CSV or form. The attestation is what lets a draft be exported on purpose while making an accidental filing
> impossible.

## Getting help

- **`btctax <command> --help`** — every command documents its arguments, including file formats with examples.
- **Man pages** — `man -l docs/man/btctax.1` (and one page per subcommand, e.g.
  `man -l docs/man/btctax-reconcile-import-selections.1`), plus `btctax-tui.1` / `btctax-tui-edit.1`.
  Run `make docs` to regenerate them, or `make bundles` for one combined PDF per binary.
- **In the editor** — press **`?`** for the keyboard shortcuts.

## Price data

Income fair-market values and disposal proceeds (when you don't supply an explicit price) resolve against a
**bundled daily-close BTC/USD dataset** — one row per calendar day, ~2010-07-17 through mid-2026, compiled
into the binaries. These are public daily-close market facts, derived from public Binance / CoinGecko data.

**Newer dates than the bundle** (or dates it doesn't cover) are handled by the optional
`btctax-update-prices` tool, which fetches daily closes (Binance primary, CoinGecko fallback) into a **local
price cache** (`<data-dir>/btctax/price_cache.csv`, overridable with `--price-cache` or `$BTCTAX_PRICE_CACHE`):

```console
$ btctax-update-prices              # fetch newer closes into the cache
$ btctax-update-prices --dry-run    # preview only; write nothing
$ btctax-update-prices --lag 8      # skip the N most-recent (still-settling) days (default 8)
```

The tax binaries then read **cache-over-bundled** with no network access of their own. Two reproducibility
notes:

- The **cache is a documented local input**, like your vault: a projection is reproducible *given* (events +
  bundled dataset + cache). The **bundled-only** projection (no cache) is the published-reproducible
  baseline — delete the cache to reproduce it exactly.
- Under **pseudo-reconcile** mode, a missing income FMV can be filled from the daily close as a loudly
  `[PSEUDO]`-flagged estimate; those never reach an export unless you explicitly attest.

## Data & privacy

`btctax` is offline and stores everything in the passphrase-encrypted `vault.pgp`. **Never commit `vault.pgp`,
`vault.key`, or your exchange exports** — the repository's `.gitignore` already excludes `vault*` / `*.pgp` /
`*.asc`. Note that the `.gitignore` does **not** cover the `export-snapshot` CSVs, so always export to a
location outside any git repo. The **online** `btctax-update-prices` tool contacts public price APIs (Binance,
CoinGecko) and sends a generic User-Agent with no personal data; the three tax binaries never touch the
network.

## Contributing

Build and test the workspace with `cargo test --workspace`. All non-trivial work follows
[`STANDARD_WORKFLOW.md`](./STANDARD_WORKFLOW.md) (spec → independent review to green → phased TDD → whole-diff
review → ship). CI runs the test suite on Linux, macOS, and Windows.

## Disclaimer

**btctax does not file or prepare your tax return.** It is not tax-preparation software, it is not a tax
preparer, and it is not a substitute for professional tax or legal advice.

What btctax *does* is help you **document and attest to what is true** — your acquisitions, disposals,
transfers, income, gifts, and the cost-basis and reconciliation decisions you make about them — so that the
figures you take to your return (or to your accountant) are as accurate, complete, and defensible as you can
make them. Every number it produces is a consequence of the facts you import and the decisions you confirm.
The software surfaces what follows from those inputs; it does not decide, on your behalf, what to report.

**You are the preparer.** You choose the transactions, you make the reconciliation and basis decisions, and
you — together with any tax professional you engage — are responsible for the accuracy and honesty of what you
ultimately file. btctax is a record-keeping and computation aid built on the principle that an accurate,
auditable, honest ledger is the best foundation for filing correctly. It cannot and does not vouch for the
truthfulness of the inputs you give it.

Use it to help yourself get it right. Do not use it — or point to it — as cover for a return you know to be
wrong: it tells you plainly, right here, that it is not preparing your taxes. **You are.**

## No warranty — verify it yourself

**btctax is early-stage, pre-release software, provided "as is" and without warranty of any kind.** It has not
been independently audited, professionally reviewed, or validated against real-world tax filings, and it may
contain errors that produce incorrect results.

Do not trust its output without checking it yourself. Before you rely on anything btctax computes — for a
filing or any other purpose — independently verify it: reconcile the numbers against your own records, test it
on inputs whose answers you already know, and confirm the results with a qualified tax professional. Treat
every figure as something you are responsible for satisfying yourself is correct, not as an authoritative
answer.

**If in doubt, don't rely on it.**

## License

Licensed under either of **MIT** or **The Unlicense** at your option.

The bundled daily-close price dataset (`crates/btctax-adapters/data/btc_usd_daily_close.csv`) is public
market data (factual daily closes derived from public Binance / CoinGecko data) and carries no separate
license or attribution.
