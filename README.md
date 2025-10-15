# linux-ls-parser

[![License](https://img.shields.io/crates/l/linux-ls-parser.svg)](https://choosealicense.com/licenses/mit/)
[![Crates.io](https://img.shields.io/crates/v/linux-ls-parser.svg)](https://crates.io/crates/linux-ls-parser)
[![Docs.rs](https://docs.rs/linux-ls-parser/badge.svg)](https://docs.rs/linux-ls-parser)

Parses files and folders from the output of the `ls -lpa` Linux command.

Device files and symlinks are currently ignored. If you need them, submit a PR.

## Contributing

Please run [.pre-commit.sh] before sending a PR, it will check everything.

## License

This project is licensed under the [MIT license][license].

[.pre-commit.sh]:
  https://github.com/imbolc/linux-ls-parser/blob/main/.pre-commit.sh
[license]: https://github.com/imbolc/linux-ls-parser/blob/main/LICENSE
