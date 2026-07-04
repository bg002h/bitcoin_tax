# btctax — developer convenience targets.
# The real build/test entry points remain `cargo build` / `cargo test --workspace`.

.PHONY: docs docs-man help

## docs: regenerate the committed man pages AND the PDFs (requires `groff` with the pdf device)
docs:
	cargo run -p xtask -- docs --pdf

## docs-man: regenerate only the committed man pages docs/man/*.1 (no groff needed)
docs-man:
	cargo run -p xtask -- docs

## help: list targets
help:
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/^## //'
