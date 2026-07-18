# tui-wrap.awk — render one style-aware TUI golden's GLYPH grid to roff -man for `groff -man -T pdf`.
# Convenience render only, NOT byte-gated (the .txt golden is the gated artifact; Task 3.3). Monochrome:
# the per-cell style OVERLAY (colors) is metadata here and is dropped — colorized render is a follow-up
# (UX-P3-2). The screen name is passed via `-v name=<stem>`.
#
# Used by `make examples-tui`:  awk -v name=<stem> -f docs/examples-tui/tui-wrap.awk <golden> | groff …
BEGIN { print ".SH \"" name "\""; ingl = 0 }
# The glyph section opens the frame; the style section ends the render.
/^── glyphs ──/  { ingl = 1; print ".nf"; print ".ft CR"; next }
/^── styles/     { if (ingl) { print ".ft P"; print ".fi"; ingl = 0 } next }
ingl == 1 {
    line = $0
    sub(/^[ 0-9]*│/, "", line)   # strip the "  0│" row-number prefix (up to and incl. the first │)
    gsub(/\\/, "\\e", line)      # escape backslashes for roff
    gsub(/⚠/, "(!)", line)       # U+26A0 has no gropdf glyph
    # gropdf lacks the box-drawing glyphs — map to ASCII for a warning-free render (the .txt golden keeps
    # the Unicode; this is the convenience PDF only).
    gsub(/[─═]/, "-", line)
    gsub(/[│║]/, "|", line)
    gsub(/[┌┐└┘├┤┬┴┼╔╗╚╝╠╣╦╩╬]/, "+", line)
    if (line ~ /^[.']/) line = "\\&" line   # protect a leading roff control char
    print line
}
