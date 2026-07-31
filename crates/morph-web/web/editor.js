import { basicSetup, EditorView } from "codemirror";
import { Compartment, EditorState } from "@codemirror/state";
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { markdown } from "@codemirror/lang-markdown";
import { html } from "@codemirror/lang-html";
import { xml } from "@codemirror/lang-xml";
import { stex } from "@codemirror/legacy-modes/mode/stex";
import { textile } from "@codemirror/legacy-modes/mode/textile";
import { tags } from "@lezer/highlight";

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

const morphHighlightStyle = HighlightStyle.define([
  {
    tag: [tags.heading, tags.heading1, tags.heading2, tags.heading3],
    color: "var(--syntax-heading)",
    fontWeight: "600",
  },
  {
    tag: [tags.heading4, tags.heading5, tags.heading6],
    color: "var(--syntax-heading)",
    fontWeight: "500",
  },
  {
    tag: [tags.strong, tags.definition],
    color: "var(--syntax-strong)",
    fontWeight: "600",
  },
  {
    tag: tags.emphasis,
    color: "var(--syntax-emphasis)",
    fontStyle: "italic",
  },
  {
    tag: tags.strikethrough,
    color: "var(--syntax-muted)",
    textDecoration: "line-through",
  },
  {
    tag: [tags.keyword, tags.controlKeyword, tags.definitionKeyword, tags.moduleKeyword],
    color: "var(--syntax-keyword)",
  },
  {
    tag: [tags.string, tags.docString, tags.character, tags.attributeValue],
    color: "var(--syntax-string)",
  },
  {
    tag: [tags.number, tags.integer, tags.float, tags.unit],
    color: "var(--syntax-number)",
  },
  {
    tag: [tags.bool, tags.null, tags.atom, tags.constant],
    color: "var(--syntax-constant)",
  },
  {
    tag: [tags.typeName, tags.className, tags.namespace],
    color: "var(--syntax-type)",
  },
  {
    tag: [tags.function, tags.macroName, tags.annotation],
    color: "var(--syntax-function)",
  },
  {
    tag: [tags.propertyName, tags.attributeName, tags.labelName],
    color: "var(--syntax-property)",
  },
  {
    tag: [tags.tagName, tags.processingInstruction],
    color: "var(--syntax-tag)",
  },
  {
    tag: [tags.link, tags.url],
    color: "var(--syntax-link)",
    textDecoration: "underline",
  },
  {
    tag: tags.monospace,
    color: "var(--syntax-code)",
    backgroundColor: "var(--syntax-code-bg)",
  },
  {
    tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment],
    color: "var(--syntax-comment)",
    fontStyle: "italic",
  },
  {
    tag: [tags.meta, tags.documentMeta],
    color: "var(--syntax-meta)",
  },
  {
    tag: [tags.list, tags.quote, tags.contentSeparator, tags.separator],
    color: "var(--syntax-marker)",
  },
  {
    tag: [
      tags.operator,
      tags.arithmeticOperator,
      tags.compareOperator,
      tags.logicOperator,
      tags.bitwiseOperator,
    ],
    color: "var(--syntax-operator)",
  },
  {
    tag: [
      tags.punctuation,
      tags.brace,
      tags.bracket,
      tags.squareBracket,
      tags.paren,
      tags.angleBracket,
    ],
    color: "var(--syntax-punctuation)",
  },
  {
    tag: [tags.escape, tags.regexp, tags.special],
    color: "var(--syntax-special)",
  },
  {
    tag: tags.inserted,
    color: "var(--syntax-inserted)",
  },
  {
    tag: [tags.deleted, tags.invalid],
    color: "var(--syntax-invalid)",
  },
]);

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

export function editorAccessExtensions(readOnly) {
  return [EditorState.readOnly.of(readOnly), EditorView.editable.of(true)];
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
      syntaxHighlighting(morphHighlightStyle),
      language.of(languageFor(format)),
      wrapping.of(wrap ? EditorView.lineWrapping : []),
      editable.of(editorAccessExtensions(readOnly)),
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
        effects: editable.reconfigure(editorAccessExtensions(value)),
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
