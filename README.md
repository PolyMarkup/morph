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
