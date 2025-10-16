#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

/// Parsed output of `ls -lpa` command
pub struct LsOutput {
    /// Sorted list of files
    pub files: Vec<LsOutputFile>,
    /// Sorted list of folders
    pub folders: Vec<String>,
}

/// File
pub struct LsOutputFile {
    /// File name
    pub name: String,
    /// File size in bytes
    pub size_bytes: i64,
}

/// Parsing error with the offending input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Specific parsing failure.
    pub kind: ErrorKind,
    /// The line that failed to parse.
    pub line: String,
}

/// Possible parsing error kinds when processing `ls -lpa` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Missing file mode column.
    MissingFileMode,
    /// Missing link count column.
    MissingLinkCount,
    /// Missing owner column.
    MissingOwner,
    /// Missing group column.
    MissingGroup,
    /// Missing size column.
    MissingSize,
    /// Found a size column that is not a number.
    InvalidSize {
        /// The token that failed to parse.
        token: String,
    },
    /// Missing timestamp month column.
    MissingMonth,
    /// Missing timestamp day column.
    MissingDay,
    /// Missing timestamp time or year column.
    MissingTimestamp,
    /// Missing file or directory name.
    MissingName,
    /// Found an empty quoted name.
    EmptyQuotedName,
    /// Found an unterminated escape sequence in a quoted name.
    InvalidEscapeSequence,
}

impl std::str::FromStr for LsOutput {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut files = Vec::new();
        let mut folders = Vec::new();
        let input = s
            .strip_prefix("\\\r\n")
            .or_else(|| s.strip_prefix("\\\n"))
            .unwrap_or(s);

        for raw_line in input.lines() {
            let line = raw_line.trim();

            let parsed = parse_line(line).map_err(|kind| Error::new(kind, line.to_string()))?;

            if let Some(parsed) = parsed {
                match parsed {
                    ParsedLine::File(file) => files.push(file),
                    ParsedLine::Folder(folder) => folders.push(folder),
                }
            }
        }

        files.sort_by(|a, b| a.name.cmp(&b.name));
        folders.sort();

        Ok(Self { files, folders })
    }
}

fn unescape_double_quoted(input: &str) -> Result<String, ErrorKind> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars.next().ok_or(ErrorKind::InvalidEscapeSequence)?;
            result.push(match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

fn parse_name(raw: &str) -> Result<String, ErrorKind> {
    if raw.is_empty() {
        return Err(ErrorKind::MissingName);
    }

    if raw.len() >= 2 {
        let bytes = raw.as_bytes();
        if bytes[0] == b'"' && bytes[raw.len() - 1] == b'"' {
            let value = unescape_double_quoted(&raw[1..raw.len() - 1])?;
            if value.is_empty() {
                return Err(ErrorKind::EmptyQuotedName);
            }
            return Ok(value);
        }

        if bytes[0] == b'\'' && bytes[raw.len() - 1] == b'\'' {
            let value = &raw[1..raw.len() - 1];
            if value.is_empty() {
                return Err(ErrorKind::EmptyQuotedName);
            }
            return Ok(value.to_string());
        }
    }

    Ok(raw.to_string())
}

enum ParsedLine {
    File(LsOutputFile),
    Folder(String),
}

fn parse_line(line: &str) -> Result<Option<ParsedLine>, ErrorKind> {
    if line.is_empty() || line.starts_with("total ") {
        return Ok(None);
    }

    let mut parts = line.split_whitespace();
    let file_mode = parts.next().ok_or(ErrorKind::MissingFileMode)?;
    if file_mode.len() == 10 {
        match file_mode.as_bytes()[0] {
            b'l' => return Ok(None), // skip symlinks
            b'b' => return Ok(None), // skip block devices
            b'c' => return Ok(None), // skip char devices
            _ => {}
        }
    }

    // Skip link count, owner and group info. We only care about size.
    parts.next().ok_or(ErrorKind::MissingLinkCount)?;
    parts.next().ok_or(ErrorKind::MissingOwner)?;
    parts.next().ok_or(ErrorKind::MissingGroup)?;

    let size_token = parts.next().ok_or(ErrorKind::MissingSize)?;
    let size: i64 = size_token.parse().map_err(|_| ErrorKind::InvalidSize {
        token: size_token.to_string(),
    })?;

    // Skip month, day and time/year columns.
    parts.next().ok_or(ErrorKind::MissingMonth)?;
    parts.next().ok_or(ErrorKind::MissingDay)?;
    parts.next().ok_or(ErrorKind::MissingTimestamp)?;

    let mut raw_name = parts.collect::<Vec<_>>().join(" ");
    if raw_name.is_empty() {
        return Err(ErrorKind::MissingName);
    }

    let is_directory = raw_name.ends_with('/');
    if is_directory {
        while raw_name.ends_with('/') {
            raw_name.pop();
        }
    }

    let name = parse_name(&raw_name)?;

    if name == "." || name == ".." {
        return Ok(None);
    }

    if is_directory {
        if name.is_empty() {
            return Ok(None);
        }

        Ok(Some(ParsedLine::Folder(name)))
    } else {
        Ok(Some(ParsedLine::File(LsOutputFile {
            name,
            size_bytes: size,
        })))
    }
}

impl Error {
    fn new(kind: ErrorKind, line: String) -> Self {
        Self { kind, line }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFileMode => write!(f, "missing file mode field"),
            Self::MissingLinkCount => write!(f, "missing link count field"),
            Self::MissingOwner => write!(f, "missing owner field"),
            Self::MissingGroup => write!(f, "missing group field"),
            Self::MissingSize => write!(f, "missing size field"),
            Self::InvalidSize { token } => write!(f, "invalid size value `{token}`"),
            Self::MissingMonth => write!(f, "missing timestamp month field"),
            Self::MissingDay => write!(f, "missing timestamp day field"),
            Self::MissingTimestamp => write!(f, "missing timestamp time or year field"),
            Self::MissingName => write!(f, "missing file name"),
            Self::EmptyQuotedName => write!(f, "empty quoted file name"),
            Self::InvalidEscapeSequence => write!(f, "unterminated escape sequence in file name"),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} in line `{}`", self.kind, self.line)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    const EDGE_CASE_FILE_ENTRIES: [(&str, i64); 42] = [
        (r" -space-dash-", 13),
        (r" multiple consecutive spaces ", 34),
        (r"!exclamation!mark!", 18),
        (r#""double"quote""#, 14),
        (r"#hash#tag#", 10),
        (r"$dollar$sign$", 13),
        (r"%percent%value%", 15),
        (r"&ampersand&symbol&", 18),
        (r"'$'\n''newline'$'\n''line", 17),
        (r"'$'\r''return'$'\r''carriage'$'\r", 20),
        (r"'$'\t''tab'$'\t''indent'$'\t", 15),
        (r"'single'quote'", 14),
        (r"(paren(open(", 12),
        (r")paren)close)", 13),
        (r"*asterisk*star*", 15),
        (r"+plus+sign+", 11),
        (r",comma,list,", 12),
        (r"---dash---triple---", 19),
        (r"-hyphen-entry-", 14),
        (r"..double..dot..", 15),
        (r".hidden. with spaces.", 21),
        (r":colon:case:", 12),
        (r";semicolon;case;", 16),
        (r"<less<than<", 11),
        (r"=equals=case=", 13),
        (r">greater>than>", 14),
        (r"?question?mark?", 15),
        (r"@at@symbol@", 11),
        (r"[bracket[left[", 14),
        (r"\backslash\path\", 19),
        (r"\x20space\x20pad\x20", 20),
        (r"]bracket]right]", 15),
        (r"^caret^symbol^", 14),
        (r"_underscore_label_", 18),
        (r"`backtick`quote`", 16),
        (r"{brace{left{", 12),
        (r"|pipe|vertical|", 15),
        (r"}brace}right}", 13),
        (r"~tilde~wave~", 12),
        (r"файл", 8),
        (r"文件", 6),
        (r"🚀rocket🚀ship🚀", 22),
    ];

    #[test]
    fn folders() {
        let input = "\
total 16
drwxr-xr-x  5 user user  4096 Jan  1 12:00 ./
drwxr-xr-x  2 user user  4096 Jan  1 12:01 ../
drwxr-xr-x  4 user user  4096 Jan  1 12:02 zeta/
drwxr-xr-x  4 user user  4096 Jan  1 12:02 alpha/
";

        let output = LsOutput::from_str(input).unwrap();

        assert_eq!(output.folders.len(), 2);
        assert_eq!(output.files.len(), 0);
        assert_eq!(output.folders, vec!["alpha", "zeta"]);
    }

    #[test]
    fn files() {
        let input = "\
total 12
drwxr-xr-x  5 root root 4096 Jan  1 00:00 ./
drwxr-xr-x  5 root root 4096 Jan  1 00:00 ../
-rw-r--r--  1 root root   16 Jan  1 00:01 arrow -> name
-rw-r--r--  1 root root   16 Jan  1 00:01 notes.txt
-rw-r--r--  1 root root    8 Jan  1 00:02 .hidden
";

        let output = LsOutput::from_str(input).unwrap();

        assert_eq!(output.folders.len(), 0);
        assert_eq!(output.files.len(), 3);
        let files: Vec<(&str, i64)> = output
            .files
            .iter()
            .map(|f| (f.name.as_str(), f.size_bytes))
            .collect();
        assert_eq!(
            files,
            vec![(".hidden", 8), ("arrow -> name", 16), ("notes.txt", 16)]
        );
    }

    #[test]
    fn ignores_symlinks() {
        let input = "\
lrwxrwxrwx  1 user user     6 Jan  1 12:04 link -> target
";

        let output: LsOutput = input.parse().unwrap();
        assert_eq!(output.folders.len(), 0);
        assert_eq!(output.files.len(), 0);
    }

    #[test]
    fn ignores_device_files() {
        let input = "\
brw-rw----  1 root disk 8, 0 Jan  1 12:00 sda
crw-rw----  1 root disk 8, 1 Jan  1 12:00 sda1
";

        let output: LsOutput = input.parse().unwrap();
        assert_eq!(output.folders.len(), 0);
        assert_eq!(output.files.len(), 0);
    }

    #[test]
    fn unicode_names() {
        let input = "\
drwxrwxr-x 2 imbolc imbolc 4096 Oct 14 10:43 пора/
-rw-rw-r-- 1 imbolc imbolc    0 Oct 14 10:43 спать
";

        let output: LsOutput = input.parse().unwrap();
        assert_eq!(output.folders.len(), 1);
        assert_eq!(output.folders[0], "пора");
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].name, "спать");
    }

    #[test]
    fn spaces() {
        let input = r#"\
drwxrwxr-x 2 imbolc imbolc 4096 Oct 14 10:49 "let's play"/
-rw-rw-r-- 1 imbolc imbolc    0 Oct 14 10:50 'давай играть'
"#;

        let output: LsOutput = input.parse().unwrap();
        assert_eq!(output.folders.len(), 1);
        assert_eq!(output.folders[0], "let's play");
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].name, "давай играть");
    }

    #[test]
    fn error_includes_offending_line() {
        let err = match "broken line".parse::<LsOutput>() {
            Err(err) => err,
            Ok(_) => panic!("expected error"),
        };
        assert!(err.to_string().contains("broken line"));
        assert_eq!(err.line, "broken line");
    }

    #[test]
    fn rejects_malformed_line() {
        assert!("broken line".parse::<LsOutput>().is_err());
    }

    // Tests files generated by ./edge-case-samples.sh
    #[test]
    fn edge_case_files() {
        let input = r#"\
total 176
drwxrwxr-x 2 imbolc imbolc 4096 Oct 15 12:05  ./
drwxrwxr-x 4 imbolc imbolc 4096 Oct 15 12:05  ../
-rw-rw-r-- 1 imbolc imbolc   13 Oct 15 12:05 '$dollar$sign$'
-rw-rw-r-- 1 imbolc imbolc   18 Oct 15 12:05 '&ampersand&symbol&'
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05 '*asterisk*star*'
-rw-rw-r-- 1 imbolc imbolc   11 Oct 15 12:05  @at@symbol@
-rw-rw-r-- 1 imbolc imbolc   19 Oct 15 12:05 '\backslash\path\'
-rw-rw-r-- 1 imbolc imbolc   16 Oct 15 12:05 '`backtick`quote`'
-rw-rw-r-- 1 imbolc imbolc   12 Oct 15 12:05  {brace{left{
-rw-rw-r-- 1 imbolc imbolc   13 Oct 15 12:05  }brace}right}
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05 '[bracket[left['
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05  ]bracket]right]
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05 '^caret^symbol^'
-rw-rw-r-- 1 imbolc imbolc   12 Oct 15 12:05  :colon:case:
-rw-rw-r-- 1 imbolc imbolc   12 Oct 15 12:05  ,comma,list,
-rw-rw-r-- 1 imbolc imbolc   19 Oct 15 12:05  ---dash---triple---
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05  ..double..dot..
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05 '"double"quote"'
-rw-rw-r-- 1 imbolc imbolc   13 Oct 15 12:05 '=equals=case='
-rw-rw-r-- 1 imbolc imbolc   18 Oct 15 12:05 '!exclamation!mark!'
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05 '>greater>than>'
-rw-rw-r-- 1 imbolc imbolc   10 Oct 15 12:05 '#hash#tag#'
-rw-rw-r-- 1 imbolc imbolc   21 Oct 15 12:05 '.hidden. with spaces.'
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05  -hyphen-entry-
-rw-rw-r-- 1 imbolc imbolc   11 Oct 15 12:05 '<less<than<'
-rw-rw-r-- 1 imbolc imbolc   34 Oct 15 12:05 '  multiple  consecutive   spaces  '
-rw-rw-r-- 1 imbolc imbolc   17 Oct 15 12:05 ''$'\n''newline'$'\n''line'
-rw-rw-r-- 1 imbolc imbolc   13 Oct 15 12:05 ')paren)close)'
-rw-rw-r-- 1 imbolc imbolc   12 Oct 15 12:05 '(paren(open('
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05  %percent%value%
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05 '|pipe|vertical|'
-rw-rw-r-- 1 imbolc imbolc   11 Oct 15 12:05  +plus+sign+
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05 '?question?mark?'
-rw-rw-r-- 1 imbolc imbolc   20 Oct 15 12:05 ''$'\r''return'$'\r''carriage'$'\r'
-rw-rw-r-- 1 imbolc imbolc   22 Oct 15 12:05  🚀rocket🚀ship🚀
-rw-rw-r-- 1 imbolc imbolc   16 Oct 15 12:05 ';semicolon;case;'
-rw-rw-r-- 1 imbolc imbolc   14 Oct 15 12:05 "'single'quote'"
-rw-rw-r-- 1 imbolc imbolc   13 Oct 15 12:05 ' -space-dash-'
-rw-rw-r-- 1 imbolc imbolc   15 Oct 15 12:05 ''$'\t''tab'$'\t''indent'$'\t'
-rw-rw-r-- 1 imbolc imbolc   12 Oct 15 12:05 '~tilde~wave~'
-rw-rw-r-- 1 imbolc imbolc   18 Oct 15 12:05  _underscore_label_
-rw-rw-r-- 1 imbolc imbolc   20 Oct 15 12:05 '\x20space\x20pad\x20'
-rw-rw-r-- 1 imbolc imbolc    8 Oct 15 12:05  файл
-rw-rw-r-- 1 imbolc imbolc    6 Oct 15 12:05  文件
"#;

        let output: LsOutput = input.parse().unwrap();
        assert!(output.folders.is_empty());
        let parsed_files: Vec<(&str, i64)> = output
            .files
            .iter()
            .map(|file| (file.name.as_str(), file.size_bytes))
            .collect();
        assert_eq!(parsed_files, EDGE_CASE_FILE_ENTRIES);
    }

    // Tests folders generated by ./edge-case-samples.sh
    #[test]
    fn edge_case_folders() {
        let input = r#"\
total 176
drwxrwxr-x 44 imbolc imbolc 4096 Oct 15 12:05  ./
drwxrwxr-x  4 imbolc imbolc 4096 Oct 15 12:05  ../
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '$dollar$sign$'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '&ampersand&symbol&'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '*asterisk*star*'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  @at@symbol@/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '\backslash\path\'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '`backtick`quote`'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  {brace{left{/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  }brace}right}/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '[bracket[left['/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  ]bracket]right]/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '^caret^symbol^'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  :colon:case:/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  ,comma,list,/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  ---dash---triple---/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  ..double..dot../
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '"double"quote"'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '=equals=case='/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '!exclamation!mark!'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '>greater>than>'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '#hash#tag#'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '.hidden. with spaces.'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  -hyphen-entry-/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '<less<than<'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '  multiple  consecutive   spaces  '/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ''$'\n''newline'$'\n''line'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ')paren)close)'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '(paren(open('/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  %percent%value%/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '|pipe|vertical|'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  +plus+sign+/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '?question?mark?'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ''$'\r''return'$'\r''carriage'$'\r'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  🚀rocket🚀ship🚀/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ';semicolon;case;'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 "'single'quote'"/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ' -space-dash-'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 ''$'\t''tab'$'\t''indent'$'\t'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '~tilde~wave~'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  _underscore_label_/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05 '\x20space\x20pad\x20'/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  файл/
drwxrwxr-x  2 imbolc imbolc 4096 Oct 15 12:05  文件/
"#;

        let output: LsOutput = input.parse().unwrap();
        assert!(output.files.is_empty());
        let parsed_folders: Vec<&str> = output.folders.iter().map(String::as_str).collect();
        let expected_folders: Vec<&str> = EDGE_CASE_FILE_ENTRIES
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(parsed_folders, expected_folders);
    }
}
