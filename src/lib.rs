#![cfg_attr(docsrs, feature(doc_auto_cfg))]
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
}
