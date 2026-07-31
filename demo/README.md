# Morph static conversion demo

This project is a checked-in conversion specimen, not a web application.
[`source/all-elements.html`](source/all-elements.html) is a strict HTML document
fragment containing every block and inline element in Morph's document model.
The files in [`generated/`](generated/) are produced from that single source by
Morph itself.

## Regenerate

From this directory:

```sh
./generate.sh
```

The script builds the local Morph CLI and writes all ten supported formats. It
does not download or install anything.

## Verify

To prove that the checked-in files match a fresh conversion:

```sh
./generate.sh --check
```

The check exits unsuccessfully if any generated file is missing or stale.

## What the specimen covers

- headings and paragraphs
- bold, italic, bold italic, strikeout, superscript, and subscript
- inline code, links, linked images, soft breaks, and hard breaks
- fenced code with a language
- block quotes
- unordered, ordered, and nested lists
- description lists with multiple definitions
- tables with all alignment modes, column spans, and row spans
- horizontal rules
- format-tagged raw inline and block content

Some target formats cannot express every construct. For example, a plain
Markdown table has no native row-span syntax. Those outputs demonstrate Morph's
documented, predictable degradation as well as its lossless paths.
