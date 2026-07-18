# man-wrap.awk — turn the generated Markdown golden (docs/examples/examples.md) into a roff -man
# document for `groff -man -T pdf`. The console blocks are wrapped in `.nf`/`.fi` so their verbatim
# alignment survives; headings map to `.SH`/`.SS`. The PDF is a convenience render, NOT byte-gated
# (Task 1.5) — this only has to keep groff from choking and emit a readable page.
#
# Used by `make examples`:  awk -f docs/examples/man-wrap.awk docs/examples/examples.md | groff -k -man -T pdf
BEGIN {
    print ".TH BTCTAX-EXAMPLES 7 \"\" \"btctax\" \"Worked Examples\""
    fm = 0; fmdone = 0; inpre = 0; incomment = 0
}
# YAML front matter: the first `---` opens it, the second closes it; drop the block and the fences.
/^---[ \t]*$/ {
    if (fmdone == 0) { fm = !fm; if (fm == 0) fmdone = 1; next }
}
fm == 1 { next }
# HTML comment block (the GENERATED banner) — drop it, however many lines it spans.
incomment == 1 { if ($0 ~ /-->/) incomment = 0; next }
/<!--/ { if ($0 !~ /-->/) incomment = 1; next }
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
    print line
}
