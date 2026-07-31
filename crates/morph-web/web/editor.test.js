import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { describe, expect, it } from "vitest";
import { editorAccessExtensions } from "./editor.js";

describe("editor access", () => {
  it("keeps output focusable and selectable while preventing edits", () => {
    const state = EditorState.create({
      extensions: editorAccessExtensions(true),
    });

    expect(state.facet(EditorState.readOnly)).toBe(true);
    expect(state.facet(EditorView.editable)).toBe(true);
  });

  it("keeps source focusable and editable", () => {
    const state = EditorState.create({
      extensions: editorAccessExtensions(false),
    });

    expect(state.facet(EditorState.readOnly)).toBe(false);
    expect(state.facet(EditorView.editable)).toBe(true);
  });
});
