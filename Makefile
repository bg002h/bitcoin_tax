# btctax — developer convenience targets.

.PHONY: check test lint docs docs-man examples examples-tui bundles help

## check: the validation gate — the full test suite AND clippy, run CONCURRENTLY (~6s warm)
##
## Clippy is given its own target directory on purpose. It compiles with different rustc flags
## than `test` does, so sharing one `target/` makes the two evict each other's artifacts and
## silently forces a near-full rebuild on every alternation. Separate dirs keep both warm — and
## let them run at the same time, which makes clippy effectively free.
##
## Speed, warm, on this suite: `cargo test --workspace` took 402s. This takes about 6.
## The win is mostly `[profile.dev] opt-level` (see Cargo.toml): the suite is dominated by
## integration tests that spawn the btctax binary, so it was bound by how fast the compiled code
## RAN, not by how fast it built. nextest (parallel across test binaries) and lld do the rest.
##
## ★ The exit status is PROPAGATED. A bare `wait` (no operands) always returns 0, so the obvious
## way to write this reports SUCCESS on a failing suite — a gate that lies is worse than no gate.
## Each job's PID is waited on individually and the statuses are OR-ed.
##
## ★ `--no-fail-fast`: on red, report EVERY failure, not just the first. nextest stops at the first by
## default, which turns a fold into a serial rediscovery loop — fix one, re-run, find the next. The
## suite is 7 seconds; there is no time to save by stopping early, and a gate that under-reports what
## it caught is a gate you have to run repeatedly to trust.
check:
	@cargo nextest run --workspace --no-fail-fast & t=$$!; \
	 CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings & c=$$!; \
	 st=0; wait $$t || st=1; wait $$c || st=1; \
	 if [ $$st -ne 0 ]; then echo "make check: FAILED"; fi; \
	 exit $$st

## test: the full suite on its own (nextest — parallel across test binaries, unlike `cargo test`)
test:
	cargo nextest run --workspace --no-fail-fast

## lint: clippy on its own, in its own target dir so it does not thrash the test cache
lint:
	CARGO_TARGET_DIR=target-clippy cargo clippy --workspace --all-targets --all-features -- -D warnings

## docs: regenerate the committed man pages AND the PDFs (requires `groff` with the pdf device)
docs:
	cargo run -p xtask -- docs --pdf

## bundles: one combined PDF per binary — btctax-manual.pdf (CLI: root + every subcommand)
## plus the per-page btctax-tui.pdf / btctax-tui-edit.pdf (the TUI manuals). Needs `gs`. Runs `docs` first.
bundles: docs
	{ echo docs/pdf/btctax.pdf; ls docs/pdf/btctax-*.pdf | grep -vE 'btctax-tui|btctax-manual' | sort; } \
	  | xargs gs -q -dNOPAUSE -dBATCH -sDEVICE=pdfwrite -sOutputFile=docs/pdf/btctax-manual.pdf
	@echo "wrote docs/pdf/btctax-manual.pdf (+ docs/pdf/btctax-tui{,-edit}.pdf are the TUI manuals)"

## docs-man: regenerate only the committed man pages docs/man/*.1 (no groff needed)
docs-man:
	cargo run -p xtask -- docs

## examples: render the worked-examples golden (docs/examples/examples.md) to a PDF via groff.
## Convenience render only — the Markdown golden is the gated artifact; the PDF is NOT byte-gated
## and is git-ignored (docs/pdf/). Needs `groff` with the pdf device.
examples:
	@mkdir -p docs/pdf
	@awk -f docs/examples/man-wrap.awk docs/examples/examples.md \
	  | groff -k -man -T pdf -dpaper=letterl -P-pletterl -rLL=10i -rPO=0.4i > docs/pdf/btctax-examples.pdf
	@head -c4 docs/pdf/btctax-examples.pdf | grep -q '%PDF' \
	  && echo "wrote docs/pdf/btctax-examples.pdf (landscape)" \
	  || { echo "examples: groff did not emit a PDF (is groff installed with the pdf device?)"; exit 1; }

## examples-tui: render the style-aware TUI goldens (docs/examples-tui/*.txt) to a SEPARATE PDF via groff.
## Convenience render (glyph grids, COLORIZED from the style runs — UX-P3-2; the .txt goldens are the gated
## artifact, test-gated by the crates' `*_goldens_match_committed` tests; NOT byte-gated here). git-ignored.
## Needs `groff`. Asserts the roff carries `\m[]` color escapes so a silent regression to monochrome fails.
examples-tui:
	@mkdir -p docs/pdf
	@{ echo ".TH BTCTAX-TUI 7 \"\" \"btctax\" \"TUI Screens\""; \
	   for f in docs/examples-tui/*.txt; do \
	     awk -v name="$$(basename $$f .txt)" -f docs/examples-tui/tui-wrap.awk "$$f"; \
	   done; } > docs/pdf/.tui-screens.roff
	@grep -qF '\m[' docs/pdf/.tui-screens.roff \
	  || { echo "examples-tui: colorization missing — no \\m[] escapes (UX-P3-2 regressed)"; exit 1; }
	@groff -k -man -T pdf -dpaper=letterl -P-pletterl -rLL=10i -rPO=0.4i \
	   docs/pdf/.tui-screens.roff > docs/pdf/btctax-tui-screens.pdf
	@rm -f docs/pdf/.tui-screens.roff
	@head -c4 docs/pdf/btctax-tui-screens.pdf | grep -q '%PDF' \
	  && echo "wrote docs/pdf/btctax-tui-screens.pdf (colorized, landscape)" \
	  || { echo "examples-tui: groff did not emit a PDF"; exit 1; }

## help: list targets
help:
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/^## //'
