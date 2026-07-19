# tui-wrap.awk — render one style-aware TUI golden to COLORIZED roff -man for `groff -man -T pdf`.
#
# Convenience render only, NOT byte-gated (the `.txt` golden is the gated artifact, test-gated by the
# crates' `*_goldens_match_committed` tests; Task 3.3). UX-P3-2: the per-cell style OVERLAY that the old
# render dropped is now applied — foreground color + bold, driven by the golden's own style runs.
#
# Two passes over the single file: BUFFER the glyph grid, PARSE the style runs into a per-(row,column)
# style, then in END emit each grid row wrapping cells in `\m[<color>]` (foreground) and `\f[CB]` (bold)
# escapes. The screen name is passed via `-v name=<stem>`.
#
# Column-safety: color is applied by CELL COLUMN, so every multi-byte glyph is first mapped 1:1 to a single
# ASCII char (box-drawing -> -|+, arrows/triangle -> <^>v, em-dash -> -). After that the row is pure ASCII,
# so `substr`/`length` are cell-accurate in any awk (byte == char). A FUTURE golden glyph outside this map
# would shift color alignment for cells to its right (cosmetic only, non-gated) — add it to the map below.
#
# Modifier fidelity: any modifier containing BOLD -> bold font (`\f[CB]`). UNDERLINED / REVERSED are
# approximated as bold (nofill roff in the constant-width family has no faithful underline / reverse-video);
# background color is dropped. The gated `.txt` golden retains the full fg/bg/modifier truth.

BEGIN { print ".SH \"" name "\""; sec = ""; maxrow = -1 }

/^── glyphs ──/ { sec = "glyphs"; next }
/^── styles/    { sec = "styles"; next }

# Glyph grid: "  N│<cells...>". Buffer the cell line (everything after the FIRST │) keyed by row number N.
sec == "glyphs" {
    hdr = $0; sub(/│.*$/, "", hdr); gsub(/[^0-9]/, "", hdr); n = hdr + 0
    cells = $0; sub(/^[ 0-9]*│/, "", cells)
    glyph[n] = cells
    if (n > maxrow) maxrow = n
    next
}

# Style runs: "  N│ start..end fg=Color bg=Color mod=MODS" (any of fg/bg/mod may be absent). Record, per
# cell column in [start,end], the foreground color and whether it is bold.
sec == "styles" {
    hdr = $0; sub(/│.*$/, "", hdr); gsub(/[^0-9]/, "", hdr); n = hdr + 0
    body = $0; sub(/^[ 0-9]*│/, "", body)
    nf = split(body, tok, /[ \t]+/)
    range = ""; fg = ""; mod = ""
    for (t = 1; t <= nf; t++) {
        if (tok[t] ~ /\.\./)       range = tok[t]
        else if (tok[t] ~ /^fg=/)  fg = substr(tok[t], 4)
        else if (tok[t] ~ /^mod=/) mod = substr(tok[t], 5)
        # bg= is intentionally ignored (see header)
    }
    if (range != "") {
        si = index(range, "..")
        start = substr(range, 1, si - 1) + 0
        end = substr(range, si + 2) + 0
        bold = (mod ~ /BOLD/) ? 1 : 0
        for (c = start; c <= end; c++) {
            if (fg != "") colfg[n SUBSEP c] = fg
            if (bold)     colb[n SUBSEP c] = 1
        }
    }
    next
}

END {
    print ".nf"
    print ".ft CR"
    for (n = 0; n <= maxrow; n++) {
        line = glyph[n]
        # Map every multi-byte glyph 1:1 to one ASCII char so cell columns == byte offsets (see header).
        gsub(/─/, "-", line); gsub(/│/, "|", line)
        gsub(/[┌┐└┘├┤┬┴┼]/, "+", line)
        gsub(/—/, "-", line)
        gsub(/←/, "<", line); gsub(/↑/, "^", line); gsub(/→/, ">", line); gsub(/↓/, "v", line)
        gsub(/▲/, "^", line); gsub(/▼/, "v", line)
        out = ""; curfg = ""; curb = 0
        L = length(line)
        for (c = 1; c <= L; c++) {
            ch = substr(line, c, 1)
            if (ch == "\\") ch = "\\e" # escape a stray backslash for roff
            f = ((n SUBSEP c) in colfg) ? colfg[n SUBSEP c] : ""
            b = ((n SUBSEP c) in colb) ? 1 : 0
            if (f != curfg || b != curb) {
                if (curb)        out = out "\\f[CR]" # close bold
                if (curfg != "") out = out "\\m[]"   # close color
                if (f != "")     out = out "\\m[" tolower(f) "]"
                if (b)           out = out "\\f[CB]"
                curfg = f; curb = b
            }
            out = out ch
        }
        if (curb)        out = out "\\f[CR]"
        if (curfg != "") out = out "\\m[]"
        if (out ~ /^[.']/) out = "\\&" out # protect a leading roff control char
        print out
    }
    print ".ft P"
    print ".fi"
}
