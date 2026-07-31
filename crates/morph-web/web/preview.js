const PREVIEW_FORMATS = new Map([
  ["md", "Markdown"],
  ["adoc", "AsciiDoc"],
  ["rst", "reStructuredText"],
  ["tex", "LaTeX"],
  ["dj", "Djot"],
  ["org", "Org mode"],
  ["textile", "Textile"],
  ["html", "strict HTML"],
]);

const PREVIEW_CSS = String.raw`
  :root {
    color-scheme: light dark;
    --page: #fbfbfa;
    --ink: #202322;
    --muted: #626965;
    --rule: #d9ddda;
    --code: #f0f2f1;
    --accent: #315f77;
    font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
    font-size: 16px;
  }

  :root[data-theme="dark"] {
    --page: #171a19;
    --ink: #e9ecea;
    --muted: #a5ada8;
    --rule: #39403c;
    --code: #222725;
    --accent: #8cbcd2;
  }

  * { box-sizing: border-box; }

  html {
    min-height: 100%;
    background: var(--page);
  }

  body {
    width: min(100% - 48px, 780px);
    margin: 0 auto;
    padding: 42px 0 72px;
    background: var(--page);
    color: var(--ink);
    font-size: 1rem;
    line-height: 1.65;
    overflow-wrap: anywhere;
  }

  h1, h2, h3, h4, h5, h6 {
    margin: 1.7em 0 0.55em;
    color: var(--ink);
    font-weight: 650;
    line-height: 1.2;
    letter-spacing: -0.015em;
  }

  h1:first-child, h2:first-child, h3:first-child { margin-top: 0; }
  h1 { font-size: 2rem; }
  h2 { padding-bottom: 0.28em; border-bottom: 1px solid var(--rule); font-size: 1.5rem; }
  h3 { font-size: 1.2rem; }
  p, ul, ol, dl, blockquote, pre, table, figure { margin: 0 0 1.2em; }
  ul, ol { padding-left: 1.6em; }
  li + li { margin-top: 0.24em; }
  dt { font-weight: 650; }
  dd { margin: 0 0 0.8em 1.2em; }

  a {
    color: var(--accent);
    text-decoration-thickness: 1px;
    text-underline-offset: 0.16em;
  }

  blockquote {
    margin-left: 0;
    padding: 0.1em 0 0.1em 1.1em;
    border-left: 3px solid var(--rule);
    color: var(--muted);
  }

  code, kbd, samp, pre {
    font-family: "IBM Plex Mono", "SFMono-Regular", Consolas, monospace;
  }

  code, kbd, samp {
    padding: 0.12em 0.28em;
    border-radius: 3px;
    background: var(--code);
    font-size: 0.88em;
  }

  pre, .environment.lstlisting {
    padding: 1em 1.1em;
    border: 1px solid var(--rule);
    border-radius: 5px;
    background: var(--code);
    overflow: auto;
    line-height: 1.5;
  }

  pre code { padding: 0; background: transparent; font-size: 0.9rem; }
  .environment.lstlisting {
    margin: 0 0 1.2em;
    font-family: "IBM Plex Mono", "SFMono-Regular", Consolas, monospace;
    font-size: 0.9rem;
    white-space: pre;
  }
  hr { margin: 2em 0; border: 0; border-top: 1px solid var(--rule); }
  img, svg { max-width: 100%; height: auto; }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.94rem;
  }

  th, td {
    padding: 0.55em 0.7em;
    border: 1px solid var(--rule);
    text-align: left;
    vertical-align: top;
  }

  th { background: var(--code); font-weight: 650; }
  .admonitionblock, .note, .warning, .important {
    margin: 1.2em 0;
    padding: 0.8em 1em;
    border: 1px solid var(--rule);
    border-left: 3px solid var(--accent);
    border-radius: 4px;
  }

  @media (max-width: 560px) {
    body { width: min(100% - 28px, 780px); padding-top: 26px; }
    h1 { font-size: 1.7rem; }
  }
`;

export function previewAvailability(formatId) {
  const renderer = PREVIEW_FORMATS.get(formatId);
  if (renderer) {
    return {
      supported: true,
      description: `Preview with the ${renderer} browser renderer`,
    };
  }

  const formatName =
    formatId === "typ" ? "Typst" : formatId === "dbk" ? "DocBook" : "This format";
  return {
    supported: false,
    description: `${formatName} has no compatible browser preview renderer`,
  };
}

export async function renderPreview(formatId, source) {
  switch (formatId) {
    case "md": {
      const { default: MarkdownIt } = await import("markdown-it");
      return new MarkdownIt({ html: true, linkify: true, typographer: false }).render(source);
    }
    case "adoc": {
      const { convert } = await import("@asciidoctor/core");
      return convert(source, { safe: "secure", standalone: true });
    }
    case "rst": {
      const { RstToHtmlCompiler } = await import("rst-compiler");
      return new RstToHtmlCompiler().compile(source).body;
    }
    case "tex": {
      const [
        { unified },
        { default: stringify },
        { unifiedLatexFromString },
        { unifiedLatexToHast },
      ] = await Promise.all([
        import("unified"),
        import("rehype-stringify"),
        import("@unified-latex/unified-latex-util-parse"),
        import("@unified-latex/unified-latex-to-hast"),
      ]);
      return String(
        unified()
          .use(unifiedLatexFromString)
          .use(unifiedLatexToHast)
          .use(stringify)
          .processSync(source),
      );
    }
    case "dj": {
      const { parse, renderHTML } = await import("@djot/djot");
      return renderHTML(parse(source));
    }
    case "org": {
      const [{ reorg }, { default: reorg2rehype }, { default: stringify }] =
        await Promise.all([
          import("@orgajs/reorg"),
          import("@orgajs/reorg-rehype"),
          import("rehype-stringify"),
        ]);
      const file = await reorg().use(reorg2rehype).use(stringify).process(source);
      return String(file);
    }
    case "textile": {
      const { default: textile } = await import("textile-js");
      return textile(source);
    }
    case "html":
      return source;
    default:
      throw new Error("No browser preview renderer is available for this format.");
  }
}

export function buildPreviewDocument(markup, title = "Morph preview", theme = "light") {
  const safeTitle = escapeHtml(title);
  const safeTheme = theme === "dark" ? "dark" : "light";
  const contentSecurityPolicy = [
    "default-src 'none'",
    "base-uri 'none'",
    "font-src data:",
    "form-action 'none'",
    "img-src data: blob:",
    "object-src 'none'",
    "script-src 'none'",
    "style-src 'unsafe-inline'",
  ].join("; ");

  return `<!doctype html>
<html lang="en" data-theme="${safeTheme}">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="Content-Security-Policy" content="${contentSecurityPolicy}">
    <meta name="referrer" content="no-referrer">
    <title>${safeTitle}</title>
    <style>${PREVIEW_CSS}</style>
  </head>
  <body>${markup}</body>
</html>`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
