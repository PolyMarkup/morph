# Morph

Morph is a universal markup converter for Markdown, AsciiDoc,
reStructuredText, Typst, and LaTeX.

## Install

On macOS or Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/polymarkup/morph/releases/latest/download/morph-installer.sh | sh
```

On Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/polymarkup/morph/releases/latest/download/morph-installer.ps1 | iex"
```

Prebuilt archives and their SHA-256 checksums are also available from
[GitHub Releases](https://github.com/polymarkup/morph/releases).

## Usage

Convert between formats using file extensions:

```sh
morph document.md document.typ
```

Convert stdin to stdout by naming the formats explicitly:

```sh
printf '# Hello\n' | morph --from markdown --to asciidoc
```

Run `morph --help` for all options and format names.

## How does Morph compare to Pandoc?

[Pandoc](https://pandoc.org/) is the better choice when you need a broad
document-processing toolkit: it supports many more formats, citations,
templates, filters, and PDF generation.

Morph is a smaller, focused converter for Markdown, AsciiDoc,
reStructuredText, Typst, and LaTeX. Its priority is **lossless conversion for
supported constructs**. Morph's shared AST explicitly represents structure
such as nested blocks, inline markup, and table row and column spans, and its
test suite checks conversions between every supported format as well as
round trips.

"Lossless" here means semantic rather than byte-for-byte preservation: when a
construct is represented by Morph and both formats can express it, converting
from A to B and back preserves the equivalent document structure. The exact
source spelling, formatting, or whitespace may be normalized. If a target
format cannot represent a feature—for example, row spans in a plain Markdown
table—Morph degrades it predictably instead of claiming that no information
was lost.

Pandoc also uses an intermediate AST, but its
[user guide](https://pandoc.org/MANUAL.html#description) notes that its
representation is less expressive than some input formats and that those
conversions can therefore be lossy. Morph takes a different trade-off: fewer
formats and features, with a smaller preservation surface that is tested
explicitly.

In short: choose Pandoc for breadth and publishing workflows; choose Morph for
a lightweight binary and preservation-focused conversion among its supported
markup formats.

## Build from source

Morph requires the Rust toolchain:

```sh
cargo build --release -p morph-cli
```

The resulting binary is written to `target/release/morph`.

## Release

Releases are built by GitHub Actions with
[dist](https://axodotdev.github.io/cargo-dist/). Update the workspace version
in `Cargo.toml`, commit it, then push a matching version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The workflow publishes macOS, Linux, and Windows binaries, checksums, and the
shell and PowerShell installers to a GitHub Release.

## License

Morph is licensed under the [Apache License 2.0](LICENSE).
