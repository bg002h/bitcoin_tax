# man-wrap.awk — turn the generated Markdown golden (docs/examples/examples.md) into a roff -man
# document for `groff -man -T pdf`. The console blocks are wrapped in `.nf`/`.fi` so their verbatim
# alignment survives; headings map to `.SH`/`.SS`. The PDF is a convenience render, NOT byte-gated
# (Task 1.5) — this only has to keep groff from choking and emit a readable page.
#
# Used by `make examples`:  awk -f docs/examples/man-wrap.awk docs/examples/examples.md | groff -k -man -T pdf
# `-v fragment=1`: FRAGMENT mode (used by the TUI screen-walkthrough's CONSOLE directive, which pipes one
# committed `.console.md` transcript through this renderer). A fragment is spliced INTO an existing roff
# doc, so it must emit no document-level `.TH`, and it must NOT run the front-matter / HTML-comment rules
# below — those are document-level and, because they are evaluated before the `inpre` fence check, a `---`
# or `<!--` line INSIDE a fenced transcript would otherwise open phantom front matter and swallow the rest
# of the block (walkthrough review M-1). In fragment mode every line is either a fence or verbatim content.
BEGIN {
    if (!fragment) print ".TH BTCTAX-EXAMPLES 7 \"\" \"btctax\" \"Worked Examples\""
    fm = 0; fmdone = 0; inpre = 0; incomment = 0
    WRAP = 118 # cols that fit the landscape page (see `make examples`); longer console lines are wrapped
}

# A verbatim console line, word-wrapped so a long advisory/report line does not run off the page (the PDF
# is a convenience render; the byte-gated artifact is the Markdown golden). Short lines print unchanged, so
# aligned tables (all < WRAP) are untouched; over-long lines break at the last space <= WRAP with a 4-space
# hanging indent, hard-breaking only a single token longer than the column.
function emit_pre(line,   brk, seg, rest) {
    while (length(line) > WRAP) {
        brk = WRAP
        while (brk > 40 && substr(line, brk, 1) != " ") brk--
        if (substr(line, brk, 1) != " ") brk = WRAP # an unbroken token wider than the column — hard break
        seg = substr(line, 1, brk); sub(/ +$/, "", seg); print seg
        rest = substr(line, brk + 1); sub(/^ +/, "", rest)
        line = "    " rest
    }
    print line
}
# YAML front matter: the first `---` opens it, the second closes it; drop the block and the fences.
# (Skipped entirely in `fragment` mode — see the BEGIN note; a `---` in a transcript is verbatim content.)
/^---[ \t]*$/ {
    if (!fragment && fmdone == 0) { fm = !fm; if (fm == 0) fmdone = 1; next }
}
fm == 1 { next }
# HTML comment block (the GENERATED banner) — drop it, however many lines it spans. (Skipped in `fragment`
# mode: a transcript that happens to print `<!--` is verbatim content, not a doc comment.)
incomment == 1 { if ($0 ~ /-->/) incomment = 0; next }
/<!--/ { if (!fragment) { if ($0 !~ /-->/) incomment = 1; next } }
# Fenced code blocks -> no-fill + constant-width. Drop the ``` fence lines themselves. `.ft CR`/`.ft P`
# (constant-width roman / previous font) is portable across groff devices, including gropdf (-T pdf).
/^```/ {
    if (inpre == 0) { print ".nf"; print ".ft CR"; inpre = 1 }
    else { print ".ft P"; print ".fi"; inpre = 0 }
    next
}
# Headings (only outside a code block).
inpre == 0 && /^## / { s = substr($0, 4); gsub(/\\/, "\\e", s); print ".SH \"" s "\""; next }
inpre == 0 && /^### / { s = substr($0, 5); gsub(/\\/, "\\e", s); print ".SS \"" s "\""; next }
# Body. Escape backslashes; protect a leading control char (`.`/`'`) with \&; blank -> .PP when filling.
{
    line = $0
    gsub(/\\/, "\\e", line)
    gsub(/⚠/, "(!)", line)   # U+26A0 has no gropdf glyph — render a plain marker instead of a warning
    if (line ~ /^[.']/) line = "\\&" line
    if (inpre == 0 && line == "") { print ".PP"; next }
    if (inpre == 1) { emit_pre(line); next } # verbatim console line — wrap if it would overrun the page
    print line
}
