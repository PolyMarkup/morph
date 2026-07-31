import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const css = readFileSync(fileURLToPath(new URL("./styles.css", import.meta.url)), "utf8");

function themeVariables(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const block = new RegExp(`${escaped}\\s*\\{([\\s\\S]*?)\\n\\}`).exec(css)?.[1];
  if (!block) throw new Error(`Missing CSS block: ${selector}`);
  return Object.fromEntries(
    [...block.matchAll(/--([\w-]+):\s*(#[\da-f]{6});/gi)].map((match) => [
      match[1],
      match[2],
    ]),
  );
}

function luminance(hex) {
  const channels = hex
    .match(/[\da-f]{2}/gi)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) =>
      channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
    );
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(foreground, background) {
  const values = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
  return (values[0] + 0.05) / (values[1] + 0.05);
}

describe("syntax theme", () => {
  it.each([
    [":root", "#ffffff"],
    [':root[data-theme="dark"]', "#161b21"],
  ])("keeps every syntax token readable in %s", (selector, background) => {
    const variables = themeVariables(selector);
    const syntaxColors = Object.entries(variables).filter(
      ([name]) => name.startsWith("syntax-") && !name.endsWith("-bg"),
    );

    expect(syntaxColors.length).toBeGreaterThan(15);
    for (const [name, color] of syntaxColors) {
      expect(contrast(color, background), name).toBeGreaterThanOrEqual(4.5);
    }
  });
});
