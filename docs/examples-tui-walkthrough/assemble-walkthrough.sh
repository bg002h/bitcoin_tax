#!/usr/bin/env bash
#
# assemble-walkthrough.sh — interleave a journey's PROSE narration, captured CLI CONSOLE transcripts, and
# captured TUI FRAMES into a single roff -man stream for `groff -man -T pdf` (the TUI screen-walkthrough
# PDF; design/tui-walkthrough SPEC §5). Convenience render ONLY — the gated artifacts are the `.txt` frame
# goldens (each TUI crate's `*_walkthrough_goldens_match_committed`), the `.console.md` transcripts
# (xtask's `walkthrough_console_golden_matches_committed`), and the manifest (grammar + FRAME⇄golden and
# CONSOLE⇄transcript bijections, xtask's `walkthrough_manifests_valid_and_complete`); the PDF is NOT
# byte-gated and is git-ignored (docs/pdf/).
#
# Usage: assemble-walkthrough.sh <manifest> <frames-dir> <tui-wrap.awk> <man-wrap.awk>
#
# Manifest grammar (one directive per line; blank lines and `#`-comments ignored):
#   PROSE <roff text>            -> a `.PP` paragraph (roff escapes like \fB \(em pass through verbatim)
#   CONSOLE <file> <caption...>  -> the captured CLI transcript <frames-dir>/<file> (a Markdown ```console
#                                   fragment) rendered VERBATIM (monospace) via man-wrap.awk, under a `.SH`
#                                   caption — the exact command + output a reader runs, never hand-typed
#   FRAME <file> <caption...>    -> the frame <frames-dir>/<file> rendered COLORIZED via tui-wrap.awk, under
#                                   its `.SH` caption
# Any other non-blank, non-comment line is a manifest error (fail closed — a typo must not silently drop a
# frame, a transcript, or narration from the walkthrough).
set -euo pipefail

manifest="${1:?usage: assemble-walkthrough.sh <manifest> <frames-dir> <tui-wrap.awk> <man-wrap.awk>}"
framedir="${2:?missing <frames-dir>}"
wrap="${3:?missing <tui-wrap.awk>}"
manwrap="${4:?missing <man-wrap.awk>}"

# A directive's caption is everything after its `<file>` token, whitespace-trimmed. Fail closed on an empty
# caption (M-4): the `.SH` would otherwise degrade to the file path.
caption_of() { # <rest-after-directive> <file> <directive-name>
  local cap
  cap="$(printf '%s' "${1#"$2"}" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
  if [ -z "$cap" ]; then
    echo "assemble-walkthrough: $3 '$2' has no caption (the .SH would read as the path)" >&2
    exit 1
  fi
  printf '%s' "$cap"
}

while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    '' | '#'*)
      continue
      ;;
    'PROSE '*)
      printf '.PP\n%s\n' "${line#PROSE }"
      ;;
    'CONSOLE '*)
      rest="${line#CONSOLE }"
      file="${rest%% *}"
      caption="$(caption_of "$rest" "$file" CONSOLE)"
      console_path="$framedir/$file"
      if [ ! -f "$console_path" ]; then
        echo "assemble-walkthrough: no such console transcript: $console_path" >&2
        exit 1
      fi
      printf '.SH "%s"\n' "$caption"
      # Reuse the examples PDF's proven ```console rendering (verbatim, page-wrapped, roff-escaped) in
      # FRAGMENT mode: no BEGIN `.TH`, and the document-level front-matter/comment rules are disabled so a
      # `---` or `<!--` line in the transcript renders verbatim instead of swallowing the block (review
      # M-1). `grep -v '^\.TH '` is a belt-and-suspenders backstop should an older renderer be passed.
      awk -v fragment=1 -f "$manwrap" "$console_path" | grep -v '^\.TH '
      ;;
    'FRAME '*)
      rest="${line#FRAME }"
      file="${rest%% *}"
      caption="$(caption_of "$rest" "$file" FRAME)"
      frame_path="$framedir/$file"
      if [ ! -f "$frame_path" ]; then
        echo "assemble-walkthrough: no such frame: $frame_path" >&2
        exit 1
      fi
      awk -v name="$caption" -f "$wrap" "$frame_path"
      ;;
    *)
      echo "assemble-walkthrough: bad manifest line (expected PROSE/CONSOLE/FRAME): $line" >&2
      exit 1
      ;;
  esac
done <"$manifest"
