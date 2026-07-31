import markdown from "../../../demo/generated/all-elements.md?raw";
import asciidoc from "../../../demo/generated/all-elements.adoc?raw";
import restructuredText from "../../../demo/generated/all-elements.rst?raw";
import typst from "../../../demo/generated/all-elements.typ?raw";
import latex from "../../../demo/generated/all-elements.tex?raw";
import djot from "../../../demo/generated/all-elements.dj?raw";
import orgMode from "../../../demo/generated/all-elements.org?raw";
import textile from "../../../demo/generated/all-elements.textile?raw";
import html from "../../../demo/generated/all-elements.html?raw";
import docbook from "../../../demo/generated/all-elements.dbk?raw";

const SPECIMENS = new Map([
  ["md", markdown],
  ["adoc", asciidoc],
  ["rst", restructuredText],
  ["typ", typst],
  ["tex", latex],
  ["dj", djot],
  ["org", orgMode],
  ["textile", textile],
  ["html", html],
  ["dbk", docbook],
]);

export function specimenFor(formatId) {
  return SPECIMENS.get(formatId) ?? null;
}
