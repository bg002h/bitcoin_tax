#!/usr/bin/env bash
#
# assemble-walkthrough.sh — interleave a journey's PROSE narration with its captured TUI FRAMES into a
# single roff -man stream for `groff -man -T pdf` (the TUI screen-walkthrough PDF; design/tui-walkthrough
# SPEC §5). Convenience render ONLY — the gated artifacts are the `.txt` frame goldens (held by each TUI
# crate's `*_walkthrough_goldens_match_committed` test) and the manifest (grammar + a FRAME⇄golden
# bijection, held by xtask's `walkthrough_manifests_valid_and_complete`); the PDF is NOT byte-gated and is
# git-ignored (docs/pdf/).
#
# Usage: assemble-walkthrough.sh <manifest> <frames-dir> <tui-wrap.awk>
#
# Manifest grammar (one directive per line; blank lines and `#`-comments ignored):
#   PROSE <roff text>          -> a `.PP` paragraph (roff escapes like \fB \(em pass through verbatim)
#   FRAME <file> <caption...>  -> the frame <frames-dir>/<file> rendered COLORIZED via tui-wrap.awk, with
#                                 the rest of the line used as its `.SH` caption
# Any other non-blank, non-comment line is a manifest error (fail closed — a typo must not silently drop
# a frame or narration from the walkthrough).
set -euo pipefail

manifest="${1:?usage: assemble-walkthrough.sh <manifest> <frames-dir> <tui-wrap.awk>}"
framedir="${2:?missing <frames-dir>}"
wrap="${3:?missing <tui-wrap.awk>}"

while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    '' | '#'*)
      continue
      ;;
    'PROSE '*)
      printf '.PP\n%s\n' "${line#PROSE }"
      ;;
    'FRAME '*)
      rest="${line#FRAME }"
      file="${rest%% *}"
      # A FRAME line MUST carry a caption after the filename (M-4): `${rest#* }` degrades to the filename
      # itself when the caption is absent, which would silently render the `.SH` as the golden's path.
      # Trim surrounding whitespace and fail closed on an empty caption.
      caption="$(printf '%s' "${rest#"$file"}" | sed 's/^[[:space:]]*//; s/[[:space:]]*$//')"
      if [ -z "$caption" ]; then
        echo "assemble-walkthrough: FRAME '$file' has no caption (the .SH would read as the path)" >&2
        exit 1
      fi
      frame_path="$framedir/$file"
      if [ ! -f "$frame_path" ]; then
        echo "assemble-walkthrough: no such frame: $frame_path" >&2
        exit 1
      fi
      awk -v name="$caption" -f "$wrap" "$frame_path"
      ;;
    *)
      echo "assemble-walkthrough: bad manifest line (expected PROSE/FRAME): $line" >&2
      exit 1
      ;;
  esac
done <"$manifest"
