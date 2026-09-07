# PII Scan — Setup Guide

This directory contains two committed scripts that together enforce the
no-PII-in-tracked-content invariant:

| Script | Role |
|--------|------|
| `pii-scan-generic.sh` | Generic SSN/EIN shape scan (CI + hook call this) |
| `pre-push` | Range-scanning pre-push hook (owner-specific + generic) |

---

## Quick install

```sh
# Option A — symlink (explicit)
ln -s ../../scripts/pre-push .git/hooks/pre-push

# Option B — hooksPath (lower friction; hook must be named exactly "pre-push")
git config core.hooksPath scripts
```

---

## Creating `scripts/.pii-patterns`

The hook requires a **local, untracked** `scripts/.pii-patterns` file.
This file is listed in both `scripts/.gitignore` and the root `.gitignore` —
it will never be committed accidentally.

```sh
# Create and edit the file:
$EDITOR scripts/.pii-patterns
```

**Format:** one ERE pattern per line. Lines starting with `#` and blank lines
are ignored. Patterns are joined with `|` to form a combined ERE for
`git grep -InE`.

Example structure (replace with your real patterns — do NOT put real values
in any committed file or message):

```
# Owner name patterns
YourName.*Holdings

# Exchange account identifier patterns
EXCH-[0-9]{8}

# Wallet address fragments (partial anchors as appropriate)
bc1q[a-z0-9]{30,}
```

**Important:** the pattern file uses ERE, so special regex characters must be
escaped where literal (`\.`, `\[`, etc.).

---

## One-time bootstrap bypass

If the patterns file does not yet exist and you need to push immediately:

```sh
BTCTAX_PII_BYPASS=1 git push
```

The bypass suppresses the fail-closed exit on a missing or empty patterns file
and issues a warning instead. The generic-shape scan (SSN/EIN shapes) still
runs even with the bypass active.

---

## bash version requirement

The hook uses `mapfile` (a bash 4 built-in). Linux ships bash 4+.
macOS ships bash 3.2 by default — install bash via Homebrew
(`brew install bash`) and ensure the Homebrew bash is the interpreter when the
hook runs. This is a future item; the current spec scope is Linux only.

---

## Path allowlist

The owner-specific scan excludes four paths, each a deliberate, accepted
exception (the user's standing rule per R0-I4):

- `LICENSE` — the MIT copyright-holder line.
- `legal/text/`, `legal/primary-sources/`, `design/forms/extract/` — **archived
  public text** (statute, regulation, Federal Register notices, IRS forms and
  instructions), every file a verbatim public document pinned by sha256
  (`legal/SHA256SUMS`, `design/forms/MANIFEST.json`, checked by `xtask
  authority-manifest`). Owner data cannot enter
  one of these files without also failing the manifest, and public text
  legitimately carries ordinary names and place names that an owner pattern
  collides with (grown 2026-09-07 after a Federal Register list of Kansas
  counties blocked a push in 247 commits; owner ruling).

Growing the allowlist further is a spec-level change, not an edit-in-place of
this hook.

---

## What is NOT in this file

No pattern examples that could be real owner data. The format section above
uses obvious placeholders. The real `scripts/.pii-patterns` lives only on
your local machine and is never committed.
