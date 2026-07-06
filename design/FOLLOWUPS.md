
### irs-8283-multi-donee-identity
**WHAT:** `fill_form_8283` (SP2) fills page-2 donee identity from the FIRST carrier row's `DonationDetails`. A tax year donating to multiple DISTINCT donees fills only the first donee's identity block (the partial-scope + needs-review notices flag it). Single-donee / multi-lot donations are fully covered.
**FIX:** group donation rows by donee and emit one 8283 copy per donee (reuse `merge_copies`), or fill per-copy identity. Surfaced SP2 whole-diff (2026-07-06); non-blocking.
