import { describe, expect, it } from "vitest";
import {
  DEFAULT_SOURCE,
  STORAGE_KEY,
  defaultState,
  describeFeatureChange,
  detectFormat,
  loadState,
  normalizeApiError,
  outputFilename,
  preservationCopy,
  sanitizeState,
  saveState,
  textStats,
} from "./model.js";

describe("format detection", () => {
  it("detects every supported extension alias", () => {
    expect(detectFormat("guide.md")).toBe("md");
    expect(detectFormat("guide.ASCIIDOC")).toBe("adoc");
    expect(detectFormat("guide.typst")).toBe("typ");
    expect(detectFormat("guide.htm")).toBe("html");
    expect(detectFormat("guide.dbk")).toBe("dbk");
  });

  it("returns null for ambiguous or absent extensions", () => {
    expect(detectFormat("README")).toBeNull();
    expect(detectFormat("document.xml")).toBeNull();
  });
});

describe("local state", () => {
  it("falls back safely from malformed state", () => {
    expect(sanitizeState({ version: 0 })).toEqual(defaultState());
    expect(sanitizeState(null).input).toBe(DEFAULT_SOURCE);
  });

  it("round trips through a storage-compatible interface", () => {
    const values = new Map();
    const storage = {
      getItem: (key) => values.get(key) ?? null,
      setItem: (key, value) => values.set(key, value),
    };
    const state = { ...defaultState(), input: "# Saved", theme: "dark" };
    expect(saveState(storage, state)).toBe(true);

    expect(values.has(STORAGE_KEY)).toBe(true);
    expect(loadState(storage)).toEqual(state);
  });

  it("keeps working when local storage is unavailable", () => {
    const storage = {
      setItem: () => {
        throw new Error("blocked");
      },
    };

    expect(saveState(storage, defaultState())).toBe(false);
  });
});

describe("output helpers", () => {
  it("builds safe output filenames", () => {
    expect(outputFilename("/tmp/My draft.md", "adoc")).toBe("My-draft.adoc");
    expect(outputFilename("", "dbk")).toBe("morph-output.dbk");
  });

  it("counts Unicode characters separately from bytes", () => {
    expect(textStats("a\n🦀")).toEqual({ characters: 3, bytes: 6, lines: 2 });
  });

  it("normalizes structured and unstructured errors", () => {
    expect(normalizeApiError({ code: "too_big", message: "Too big" }, 413)).toEqual({
      code: "too_big",
      message: "Too big",
      status: 413,
    });
    expect(normalizeApiError(null, 500).message).toContain("500");
  });

  it("describes preservation changes and statuses", () => {
    expect(
      describeFeatureChange({ feature: "column_spans", before: 2, after: 0 }),
    ).toEqual({ label: "column spans", value: "2 → 0" });
    expect(preservationCopy({ status: "preserved" }, "HTML").badge).toBe("Preserved");
    expect(preservationCopy({ status: "changed" }, "Markdown").className).toBe("changed");
  });
});
