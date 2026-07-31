export const STORAGE_KEY = "morph.web.v1";
export const MAX_INPUT_BYTES = 256 * 1024;

export const FALLBACK_FORMATS = [
  { id: "md", name: "Markdown", extension: "md", mark: "#", note: "CommonMark-style" },
  { id: "adoc", name: "AsciiDoc", extension: "adoc", mark: "=", note: "Structured writing" },
  { id: "rst", name: "reStructuredText", extension: "rst", mark: "..", note: "Python documentation" },
  { id: "typ", name: "Typst", extension: "typ", mark: "#", note: "Modern typesetting" },
  { id: "tex", name: "LaTeX", extension: "tex", mark: "\\", note: "Scientific typesetting" },
  { id: "dj", name: "Djot", extension: "dj", mark: "{", note: "Unambiguous markup" },
  { id: "org", name: "Org mode", extension: "org", mark: "*", note: "Emacs documents" },
  { id: "textile", name: "Textile", extension: "textile", mark: "h1.", note: "Concise publishing" },
  { id: "html", name: "strict HTML", extension: "html", mark: "<>", note: "Morph-safe subset" },
  { id: "dbk", name: "strict DocBook", extension: "dbk", mark: "</>", note: "Technical publishing" },
];

export const DEFAULT_SOURCE = `# A portable document

Morph converts **meaning**, not just punctuation. This paragraph has *emphasis*,
\`inline code\`, and a [link](https://github.com/PolyMarkup/morph).

> A quotation can contain structure.
>
> - Nested item one
> - Nested item two

1. Parse the source
2. Preserve its structure
3. Emit every target

\`\`\`rust
fn main() {
    println!("one AST, ten dialects");
}
\`\`\`

| Format | Direction | Promise |
| :--- | :---: | ---: |
| Markdown | both | tested |
| AsciiDoc | both | tested |
`;

export const SPECIMEN_SOURCE = `<h1>Morph preservation specimen</h1>
<p>This strict HTML source contains <strong>bold</strong>, <em>italic</em>, <strong><em>both</em></strong>, <del>strikeout</del>, H<sub>2</sub>O, x<sup>2</sup>, <code>inline code</code>, and a <a href="https://github.com/PolyMarkup/morph" title="Morph source">titled link</a>.<br>A hard break begins this sentence.</p>
<blockquote>
<p>Nested blocks remain nested.</p>
<ul><li><p>First quoted item</p></li><li><p>Second quoted item</p></li></ul>
</blockquote>
<ol start="4"><li><p>Starts at four</p></li><li><p>Continues at five</p></li></ol>
<dl><dt>Morph</dt><dd><p>A preservation-focused converter.</p></dd></dl>
<pre><code class="language-rust">fn main() {
    println!("ten formats");
}</code></pre>
<table>
<thead><tr><th align="left">Feature</th><th align="center">State</th><th align="right">Count</th></tr></thead>
<tbody><tr><td rowspan="2">Spans</td><td>preserved</td><td>2</td></tr><tr><td colspan="2">Two columns wide</td></tr></tbody>
</table>
<hr>`;

const EXTENSION_ALIASES = {
  md: "md",
  markdown: "md",
  adoc: "adoc",
  asciidoc: "adoc",
  asc: "adoc",
  rst: "rst",
  typ: "typ",
  typst: "typ",
  tex: "tex",
  latex: "tex",
  dj: "dj",
  djot: "dj",
  org: "org",
  textile: "textile",
  html: "html",
  htm: "html",
  dbk: "dbk",
  docbook: "dbk",
};

export function defaultState() {
  return {
    version: 1,
    input: DEFAULT_SOURCE,
    inputFormat: "md",
    targetFormat: "adoc",
    inputFilename: "portable-document.md",
    wrap: true,
    inspectorOpen: true,
    theme: "system",
    mobilePane: "source",
  };
}

export function sanitizeState(value) {
  const fallback = defaultState();
  if (!value || typeof value !== "object" || value.version !== 1) return fallback;
  const formatIds = new Set(FALLBACK_FORMATS.map((format) => format.id));
  return {
    version: 1,
    input: typeof value.input === "string" ? value.input.slice(0, MAX_INPUT_BYTES) : fallback.input,
    inputFormat: formatIds.has(value.inputFormat) ? value.inputFormat : fallback.inputFormat,
    targetFormat: formatIds.has(value.targetFormat) ? value.targetFormat : fallback.targetFormat,
    inputFilename:
      typeof value.inputFilename === "string" && value.inputFilename
        ? value.inputFilename.slice(0, 180)
        : fallback.inputFilename,
    wrap: typeof value.wrap === "boolean" ? value.wrap : fallback.wrap,
    inspectorOpen:
      typeof value.inspectorOpen === "boolean" ? value.inspectorOpen : fallback.inspectorOpen,
    theme: ["system", "light", "dark"].includes(value.theme) ? value.theme : fallback.theme,
    mobilePane: ["source", "output"].includes(value.mobilePane)
      ? value.mobilePane
      : fallback.mobilePane,
  };
}

export function loadState(storage) {
  try {
    return sanitizeState(JSON.parse(storage.getItem(STORAGE_KEY)));
  } catch {
    return defaultState();
  }
}

export function saveState(storage, state) {
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(sanitizeState(state)));
    return true;
  } catch {
    return false;
  }
}

export function detectFormat(filename) {
  const match = /\.([^.]+)$/.exec(filename.trim().toLowerCase());
  return match ? EXTENSION_ALIASES[match[1]] ?? null : null;
}

export function outputFilename(inputFilename, formatId) {
  const format = FALLBACK_FORMATS.find((candidate) => candidate.id === formatId);
  const extension = format?.extension ?? formatId;
  const safeName = (inputFilename || "morph-output")
    .split(/[\\/]/)
    .pop()
    .replace(/\.[^.]+$/, "")
    .replace(/[^\p{L}\p{N}._-]+/gu, "-")
    .replace(/^-+|-+$/g, "");
  return `${safeName || "morph-output"}.${extension}`;
}

export function textStats(text) {
  return {
    characters: [...text].length,
    bytes: new TextEncoder().encode(text).length,
    lines: text ? text.split(/\r\n|\r|\n/).length : 0,
  };
}

export function formatCompactNumber(value) {
  return new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }).format(value);
}

export function normalizeApiError(responseBody, status) {
  if (responseBody && typeof responseBody.message === "string") {
    return {
      code: typeof responseBody.code === "string" ? responseBody.code : "request_failed",
      message: responseBody.message,
      status,
    };
  }
  return {
    code: "request_failed",
    message: `The conversion request failed (${status}).`,
    status,
  };
}

const FEATURE_LABELS = {
  alignment_center: "center-aligned columns",
  alignment_default: "default-aligned columns",
  alignment_left: "left-aligned columns",
  alignment_right: "right-aligned columns",
  block_nodes: "block nodes",
  bold: "bold spans",
  bold_italic: "bold-italic spans",
  block_quotes: "block quotes",
  code_blocks: "code blocks",
  code_languages: "code-language labels",
  column_spans: "column spans",
  column_span_width: "combined column-span width",
  definitions: "definitions",
  description_items: "description terms",
  description_lists: "description lists",
  hard_breaks: "hard line breaks",
  headings: "headings",
  horizontal_rules: "horizontal rules",
  image_titles: "image titles",
  images: "images",
  inline_code: "inline code spans",
  inline_nodes: "inline nodes",
  italic: "italic spans",
  linked_images: "linked images",
  links: "links",
  link_titles: "link titles",
  list_items: "list items",
  maximum_block_depth: "maximum nesting depth",
  nondefault_ordered_starts: "custom list starts",
  ordered_lists: "ordered lists",
  ordered_start_total: "combined ordered-list starts",
  paragraphs: "paragraphs",
  raw_block_formats: "tagged raw blocks",
  raw_blocks: "raw blocks",
  raw_inline_formats: "tagged raw inlines",
  raw_inlines: "raw inlines",
  row_spans: "row spans",
  row_span_height: "combined row-span height",
  soft_breaks: "soft line breaks",
  strikethrough: "strikeout spans",
  subscript: "subscripts",
  superscript: "superscripts",
  table_cells: "table cells",
  table_rows: "table rows",
  tables: "tables",
  text_characters: "text characters",
  unordered_lists: "unordered lists",
};

export function describeFeatureChange(change) {
  if (change.feature === "target_reparse") {
    return { label: "Generated output could not be parsed again", value: "check unavailable" };
  }
  if (change.feature === "semantic_structure") {
    return { label: "Structure normalized differently", value: "AST changed" };
  }
  return {
    label: FEATURE_LABELS[change.feature] ?? change.feature.replaceAll("_", " "),
    value: `${change.before} → ${change.after}`,
  };
}

export function preservationCopy(report, formatName) {
  if (!report) {
    return {
      badge: "Error",
      summary: `${formatName} could not be generated.`,
      className: "error",
    };
  }
  if (report.status === "preserved") {
    return {
      badge: "Preserved",
      summary: `The ${formatName} output reparses to the same Morph document tree.`,
      className: "preserved",
    };
  }
  if (report.status === "unverifiable") {
    return {
      badge: "Unverifiable",
      summary: `Morph emitted ${formatName}, but could not parse that output again for comparison.`,
      className: "unverifiable",
    };
  }
  return {
    badge: "Changed",
    summary: `${formatName} cannot express every part of this document without structural change.`,
    className: "changed",
  };
}
