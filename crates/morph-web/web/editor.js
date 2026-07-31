import { basicSetup, EditorView } from "codemirror";
import { Compartment, EditorState } from "@codemirror/state";
import { StreamLanguage } from "@codemirror/language";
import { markdown } from "@codemirror/lang-markdown";
import { html } from "@codemirror/lang-html";
import { xml } from "@codemirror/lang-xml";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import { textile } from "@codemirror/legacy-modes/mode/textile";

const genericMarkup = StreamLanguage.define({
  startState() {
    return { fenced: false };
  },
  token(stream, state) {
    if (stream.sol()) {
      if (stream.match(/^\s*(```|----|\.\. code-block::|#\+begin_|bc\.\.)/i)) {
        state.fenced = !state.fenced;
        stream.skipToEnd();
        return "meta";
      }
      if (stream.match(/^\s*(#{1,6}|={1,6}|\*{1,6}|h[1-6]\.)\s+/)) {
        stream.skipToEnd();
        return "heading";
      }
      if (stream.match(/^\s*(?:[-+*]|\d+[.)]|#\.|\.\.)\s+/)) return "list";
      if (stream.match(/^\s*(?:>|bq\.|#\+begin_quote)/i)) {
        stream.skipToEnd();
        return "quote";
      }
      if (stream.match(/^\s*(?:\|.*\||\+[-+=]+\+)/)) {
        stream.skipToEnd();
        return "contentSeparator";
      }
    }
    if (state.fenced) {
      stream.skipToEnd();
      return "string";
    }
    if (stream.match(/https?:\/\/[^\s)\]}>]+/)) return "link";
    if (stream.match(/(?:\*\*|__|\*_).*?(?:\*\*|__|_\*)/)) return "strong";
    if (stream.match(/(?:\*|_).*?(?:\*|_)/)) return "emphasis";
    if (stream.match(/`[^`]+`/)) return "monospace";
    if (stream.match(/(?:<!--|\/\/|#\+|\/\/!).*$/)) return "comment";
    stream.next();
    return null;
  },
});

const editorTheme = EditorView.theme({
  "&": {
    backgroundColor: "transparent",
    color: "var(--ink)",
  },
  ".cm-content": {
    caretColor: "var(--accent)",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "var(--accent)",
  },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
    backgroundColor: "var(--accent-wash)",
  },
  ".cm-gutters": {
    backgroundColor: "color-mix(in srgb, var(--paper), transparent 35%)",
    color: "var(--ink-faint)",
    border: "none",
  },
});

function languageFor(format) {
  switch (format) {
    case "md":
    case "dj":
      return markdown();
    case "html":
      return html({ autoCloseTags: false });
    case "dbk":
      return xml();
    case "tex":
      return StreamLanguage.define(stex);
    case "textile":
      return StreamLanguage.define(textile);
    default:
      return genericMarkup;
  }
}

export function createEditor({
  parent,
  doc = "",
  format = "md",
  readOnly = false,
  wrap = true,
  onChange = () => {},
}) {
  const language = new Compartment();
  const wrapping = new Compartment();
  const editable = new Compartment();
  let silent = false;

  const state = EditorState.create({
    doc,
    extensions: [
      basicSetup,
      editorTheme,
      language.of(languageFor(format)),
      wrapping.of(wrap ? EditorView.lineWrapping : []),
      editable.of([EditorState.readOnly.of(readOnly), EditorView.editable.of(!readOnly)]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged && !silent) onChange(update.state.doc.toString());
      }),
    ],
  });

  const view = new EditorView({ state, parent });

  return {
    view,
    getDoc() {
      return view.state.doc.toString();
    },
    setDoc(value) {
      if (value === view.state.doc.toString()) return;
      silent = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
      silent = false;
    },
    setFormat(value) {
      view.dispatch({ effects: language.reconfigure(languageFor(value)) });
    },
    setWrap(value) {
      view.dispatch({
        effects: wrapping.reconfigure(value ? EditorView.lineWrapping : []),
      });
    },
    setReadOnly(value) {
      view.dispatch({
        effects: editable.reconfigure([
          EditorState.readOnly.of(value),
          EditorView.editable.of(!value),
        ]),
      });
    },
    focus() {
      view.focus();
    },
    destroy() {
      view.destroy();
    },
  };
}
