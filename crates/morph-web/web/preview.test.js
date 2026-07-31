import { describe, expect, it } from "vitest";
import {
  buildPreviewDocument,
  previewAvailability,
  renderPreview,
} from "./preview.js";

describe("preview availability", () => {
  it("enables formats with browser renderers", () => {
    for (const format of ["md", "adoc", "rst", "tex", "dj", "org", "textile", "html"]) {
      expect(previewAvailability(format).supported).toBe(true);
    }
  });

  it("disables formats without a compatible browser renderer", () => {
    expect(previewAvailability("typ")).toMatchObject({
      supported: false,
      description: expect.stringContaining("Typst"),
    });
    expect(previewAvailability("dbk")).toMatchObject({
      supported: false,
      description: expect.stringContaining("DocBook"),
    });
  });
});

describe("preview rendering", () => {
  it(
    "renders the non-DOM markup formats through their native libraries",
    async () => {
      const cases = [
        ["md", "# Markdown", "<h1>Markdown</h1>"],
        ["adoc", "= AsciiDoc", "AsciiDoc"],
        ["rst", "reStructuredText\n================", "reStructuredText"],
        ["tex", "\\section{LaTeX}\n\nA \\href{https://example.com}{link}.", "<h3>LaTeX</h3>"],
        ["dj", "# Djot", "<h1"],
        ["org", "* Org mode", "<h1>Org mode</h1>"],
        ["textile", "h1. Textile", "<h1>Textile</h1>"],
      ];

      for (const [format, source, expected] of cases) {
        expect(await renderPreview(format, source)).toContain(expected);
      }
    },
    20_000,
  );

  it("passes strict HTML to the browser preview document", async () => {
    const html = "<article><h1>Strict HTML</h1></article>";
    expect(await renderPreview("html", html)).toBe(html);
  });

  it("renders the LaTeX structures emitted by Morph", async () => {
    const html = await renderPreview(
      "tex",
      String.raw`\section{Portable}

A \href{https://example.com}{link}.

\begin{lstlisting}[language=rust]
fn main() {}
\end{lstlisting}

\begin{tabular}{lr}
Name & Count \\
Morph & 10 \\
\end{tabular}`,
    );

    expect(html).toContain('href="https://example.com"');
    expect(html).toContain("environment lstlisting");
    expect(html).toContain('class="tabular"');
  });
});

describe("preview document isolation", () => {
  it("adds a restrictive policy and escapes document metadata", () => {
    const document = buildPreviewDocument(
      "<h1>Preview</h1>",
      'Title"><script>alert(1)</script>',
      "dark",
    );

    expect(document).toContain('script-src \'none\'');
    expect(document).toContain('img-src data: blob:');
    expect(document).toContain('data-theme="dark"');
    expect(document).toContain("&lt;script&gt;");
    expect(document).not.toContain("<title>Title\"><script>");
  });
});
