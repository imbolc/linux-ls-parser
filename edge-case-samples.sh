#!/usr/bin/env sh
# Creates edge-case files and folders for testing 'ls -lpa' parser.
# Examples are meant to be exhaustive, concise and human-readable with intuitive meaning.

set -eu

BASE_DIR="./edge-case-samples"
FILES_DIR="$BASE_DIR/files"
FOLDERS_DIR="$BASE_DIR/folders"

rm -rf "$BASE_DIR"
mkdir -p "$FILES_DIR"
mkdir -p "$FOLDERS_DIR"

while IFS= read -r name; do
    # For the file *name*, we use `printf %b` to interpret backslash escapes like \n and \t.
    filename_interpreted=$(printf '%b' "$name")

    (cd "$FILES_DIR" && printf '%s' "$name" >"$filename_interpreted")
    mkdir -p -- "$FOLDERS_DIR/$filename_interpreted"
done <<'EOF'
  multiple  consecutive   spaces  
 -space-dash-
!exclamation!mark!
"double"quote"
#hash#tag#
$dollar$sign$
%percent%value%
&ampersand&symbol&
'single'quote'
(paren(open(
)paren)close)
*asterisk*star*
+plus+sign+
,comma,list,
---dash---triple---
-hyphen-entry-
..double..dot..
.hidden. with spaces.
:colon:case:
;semicolon;case;
<less<than<
=equals=case=
>greater>than>
?question?mark?
@at@symbol@
[bracket[left[
\\backslash\\path\\
\nnewline\nline\n
\rreturn\rcarriage\r
\ttab\tindent\t
\x20space\x20pad\x20
]bracket]right]
^caret^symbol^
_underscore_label_
`backtick`quote`
{brace{left{
|pipe|vertical|
}brace}right}
~tilde~wave~
файл
文件
🚀rocket🚀ship🚀
EOF
