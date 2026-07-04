# btctax — developer convenience targets.
# The real build/test entry points remain `cargo build` / `cargo test --workspace`.

.PHONY: docs docs-man bundles help

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

## help: list targets
help:
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/^## //'
