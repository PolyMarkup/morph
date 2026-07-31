import { describe, expect, it } from "vitest";
import { FALLBACK_FORMATS } from "./model.js";
import { specimenFor } from "./specimens.js";

describe("source specimens", () => {
  it("provides a native specimen for every supported source format", () => {
    for (const format of FALLBACK_FORMATS) {
      const specimen = specimenFor(format.id);

      expect(specimen, format.name).toBeTypeOf("string");
      expect(specimen.trim().length, format.name).toBeGreaterThan(100);
    }
  });

  it("uses recognizably native syntax", () => {
    expect(specimenFor("md")).toMatch(/^# Morph:/);
    expect(specimenFor("adoc")).toMatch(/^= Morph:/);
    expect(specimenFor("tex")).toMatch(/^\\section\{/);
    expect(specimenFor("html")).toMatch(/^<h1>/);
    expect(specimenFor("dbk")).toMatch(/^<article /);
  });

  it("returns null for unknown formats", () => {
    expect(specimenFor("unknown")).toBeNull();
  });
});
