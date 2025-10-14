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

impl std::str::FromStr for LsOutput {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn unescape_double_quoted(input: &str) -> Result<String, ()> {
            let mut result = String::with_capacity(input.len());
            let mut chars = input.chars();

            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    let escaped = chars.next().ok_or(())?;
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

        fn normalize_name(raw: &str) -> Result<String, ()> {
            if raw.is_empty() {
                return Err(());
            }

            if raw.len() >= 2 {
                let bytes = raw.as_bytes();
                if bytes[0] == b'"' && bytes[raw.len() - 1] == b'"' {
                    let value = unescape_double_quoted(&raw[1..raw.len() - 1])?;
                    if value.is_empty() {
                        return Err(());
                    }
                    return Ok(value);
                }

                if bytes[0] == b'\'' && bytes[raw.len() - 1] == b'\'' {
                    let value = &raw[1..raw.len() - 1];
                    if value.is_empty() {
                        return Err(());
                    }
                    return Ok(value.to_string());
                }
            }

            Ok(raw.to_string())
        }

        let mut files = Vec::new();
        let mut folders = Vec::new();
        let input = s
            .strip_prefix("\\\r\n")
            .or_else(|| s.strip_prefix("\\\n"))
            .unwrap_or(s);

        for raw_line in input.lines() {
            let line = raw_line.trim();

            if line.is_empty() || line.starts_with("total ") {
                continue;
            }

            let mut parts = line.split_whitespace();
            parts.next().ok_or(())?;

            // Skip link count, owner and group info. We only care about size.
            parts.next().ok_or(())?;
            parts.next().ok_or(())?;
            parts.next().ok_or(())?;

            let size_token = parts.next().ok_or(())?;
            let size: i64 = match size_token.parse() {
                Ok(value) => value,
                Err(_) if size_token.ends_with(',') => {
                    // Device files use "major, minor". Skip the minor value.
                    parts.next().ok_or(())?;
                    0
                }
                Err(_) => return Err(()),
            };

            // Skip month, day and time/year columns.
            parts.next().ok_or(())?;
            parts.next().ok_or(())?;
            parts.next().ok_or(())?;

            let mut name = parts.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                return Err(());
            }

            if let Some(idx) = name.find(" -> ") {
                name.truncate(idx);
            }

            let is_directory = name.ends_with('/');
            if is_directory {
                while name.ends_with('/') {
                    name.pop();
                }
            }

            let name = normalize_name(&name)?;

            if name == "." || name == ".." {
                continue;
            }

            if is_directory {
                if name.is_empty() {
                    continue;
                }

                folders.push(name);
            } else {
                files.push(LsOutputFile {
                    name,
                    size_bytes: size,
                });
            }
        }

        files.sort_by(|a, b| a.name.cmp(&b.name));
        folders.sort();

        Ok(Self { files, folders })
    }
}

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
-rw-r--r--  1 root root   16 Jan  1 00:01 notes.txt
-rw-r--r--  1 root root    8 Jan  1 00:02 .hidden
";

        let output = LsOutput::from_str(input).unwrap();

        assert_eq!(output.folders.len(), 0);
        assert_eq!(output.files.len(), 2);
        let files: Vec<(&str, i64)> = output
            .files
            .iter()
            .map(|f| (f.name.as_str(), f.size_bytes))
            .collect();
        assert_eq!(files, vec![(".hidden", 8), ("notes.txt", 16)]);
    }

    #[test]
    fn synlinks() {
        let input = "\
lrwxrwxrwx  1 user user     6 Jan  1 12:04 link -> target
";

        let output: LsOutput = input.parse().unwrap();
        assert_eq!(output.folders.len(), 0);
        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].name, "link");
        assert_eq!(output.files[0].size_bytes, 6);
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
    fn rejects_malformed_line() {
        assert!("broken line".parse::<LsOutput>().is_err());
    }
}
