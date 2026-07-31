import "./styles.css";
import { createEditor } from "./editor.js";
import {
  FALLBACK_FORMATS,
  MAX_INPUT_BYTES,
  SPECIMEN_SOURCE,
  defaultState,
  describeFeatureChange,
  detectFormat,
  formatCompactNumber,
  loadState,
  normalizeApiError,
  outputFilename,
  preservationCopy,
  saveState,
  textStats,
} from "./model.js";

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];

const elements = {
  sourceFormat: $("#source-format"),
  sourceStats: $("#source-stats"),
  outputStats: $("#output-stats"),
  outputFormatNote: $("#output-format-note"),
  sourcePanel: $(".source-panel"),
  sourceEditor: $("#source-editor"),
  outputEditor: $("#output-editor"),
  formatRail: $("#format-rail"),
  formatSpecimens: $("#format-specimens"),
  emptyOutput: $("#empty-output"),
  convertButton: $("#convert-button"),
  openFileButton: $("#open-file-button"),
  fileInput: $("#file-input"),
  sampleButton: $("#sample-button"),
  clearButton: $("#clear-button"),
  wrapButton: $("#wrap-button"),
  copyButton: $("#copy-button"),
  downloadButton: $("#download-button"),
  useAsSourceButton: $("#use-as-source-button"),
  inspector: $("#inspector"),
  inspectorToggle: $("#inspector-toggle"),
  inspectorBody: $("#inspector-body"),
  preservationBadge: $("#preservation-badge"),
  inspectorSummary: $("#inspector-summary"),
  changeList: $("#change-list"),
  statusLamp: $("#status-lamp"),
  statusText: $("#status-text"),
  errorBanner: $("#error-banner"),
  errorTitle: $("#error-title"),
  errorMessage: $("#error-message"),
  dismissError: $("#dismiss-error"),
  workspace: $(".workspace"),
  themeToggle: $("#theme-toggle"),
  clearLocalData: $("#clear-local-data"),
  toast: $("#toast"),
  installCommand: $("#install-command"),
  copyInstallButton: $("#copy-install-button"),
};

const installCommands = {
  unix:
    "curl --proto '=https' --tlsv1.2 -LsSf \\\n  https://github.com/PolyMarkup/morph/releases/latest/download/morph-installer.sh | sh",
  windows:
    'powershell -ExecutionPolicy Bypass -c "irm https://github.com/PolyMarkup/morph/releases/latest/download/morph-installer.ps1 | iex"',
};

let state = loadState(localStorage);
let formats = FALLBACK_FORMATS;
let results = new Map();
let converting = false;
let saveTimer;
let toastTimer;
let installPlatform = navigator.userAgent.includes("Windows") ? "windows" : "unix";

applyTheme();

const sourceEditor = createEditor({
  parent: elements.sourceEditor,
  doc: state.input,
  format: state.inputFormat,
  wrap: true,
  onChange(value) {
    state.input = value;
    renderSourceStats();
    scheduleSave();
    invalidateResults("Source changed · ready to convert");
  },
});

const outputEditor = createEditor({
  parent: elements.outputEditor,
  doc: "",
  format: state.targetFormat,
  readOnly: true,
  wrap: state.wrap,
});

initialize();

async function initialize() {
  renderSourceStats();
  renderInspectorState();
  renderInstallCommand();
  setMobilePane(state.mobilePane);
  elements.wrapButton.setAttribute("aria-pressed", String(state.wrap));
  await loadFormats();
  renderFormatRail();
  renderFormatSpecimens();
  populateSourceFormats();
  selectTarget(state.targetFormat, false);
  sourceEditor.focus();
}

async function loadFormats() {
  try {
    const response = await fetch("/api/formats", {
      headers: { Accept: "application/json" },
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    const fallbackById = new Map(FALLBACK_FORMATS.map((format) => [format.id, format]));
    formats = data.formats
      .map((format) => {
        const fallback = fallbackById.get(format.id);
        return { ...format, ...fallback, id: format.id };
      })
      .filter((format) => format.id && format.name);
    if (formats.length !== FALLBACK_FORMATS.length) formats = FALLBACK_FORMATS;
  } catch {
    formats = FALLBACK_FORMATS;
    showToast("Using the built-in format list; the API is not reachable yet.");
  }
}

function populateSourceFormats() {
  elements.sourceFormat.replaceChildren(
    ...formats.map((format) => {
      const option = document.createElement("option");
      option.value = format.id;
      option.textContent = format.name;
      option.selected = format.id === state.inputFormat;
      return option;
    }),
  );
}

function renderFormatRail() {
  elements.formatRail.replaceChildren(
    ...formats.map((format) => {
      const result = results.get(format.id);
      const button = document.createElement("button");
      button.type = "button";
      button.className = "format-tab";
      button.role = "tab";
      button.dataset.format = format.id;
      button.dataset.status = result?.error
        ? "error"
        : (result?.preservation?.status ?? "idle");
      button.setAttribute("aria-selected", String(format.id === state.targetFormat));
      button.setAttribute("aria-controls", "output-editor");
      button.textContent = format.name.replace("strict ", "");
      button.addEventListener("click", () => selectTarget(format.id));
      button.addEventListener("keydown", handleFormatTabKeydown);
      return button;
    }),
  );
}

function handleFormatTabKeydown(event) {
  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
  event.preventDefault();
  const current = formats.findIndex((format) => format.id === state.targetFormat);
  let next = current;
  if (event.key === "ArrowLeft") next = (current - 1 + formats.length) % formats.length;
  if (event.key === "ArrowRight") next = (current + 1) % formats.length;
  if (event.key === "Home") next = 0;
  if (event.key === "End") next = formats.length - 1;
  selectTarget(formats[next].id);
  elements.formatRail.querySelector(`[data-format="${formats[next].id}"]`)?.focus();
}

function renderFormatSpecimens() {
  elements.formatSpecimens.replaceChildren(
    ...formats.map((format, index) => {
      const article = document.createElement("article");
      article.className = "format-specimen";
      const mark = document.createElement("span");
      mark.className = "syntax-mark";
      mark.textContent = format.mark ?? String(index + 1).padStart(2, "0");
      const heading = document.createElement("h3");
      heading.textContent = format.name;
      const note = document.createElement("p");
      note.textContent = format.note ?? "Read and write";
      article.append(mark, heading, note);
      return article;
    }),
  );
}

function selectTarget(formatId, save = true) {
  if (!formats.some((format) => format.id === formatId)) formatId = formats[0].id;
  state.targetFormat = formatId;
  outputEditor.setFormat(formatId);
  renderFormatRail();
  renderSelectedOutput();
  if (save) {
    scheduleSave();
    if (window.matchMedia("(max-width: 800px)").matches && results.size) {
      setMobilePane("output");
    }
  }
}

function renderSelectedOutput() {
  const format = getFormat(state.targetFormat);
  const result = results.get(state.targetFormat);
  const hasOutput = Boolean(result?.output);

  elements.emptyOutput.hidden = hasOutput || Boolean(result?.error);
  elements.copyButton.disabled = !hasOutput;
  elements.downloadButton.disabled = !hasOutput;
  elements.useAsSourceButton.disabled = !hasOutput;

  if (!result) {
    outputEditor.setDoc("");
    elements.outputStats.textContent = "Awaiting conversion";
    elements.outputFormatNote.textContent = "Choose a format after conversion";
    renderInspectorState();
    return;
  }

  if (result.error) {
    outputEditor.setDoc(`Conversion error\n\n${result.error}`);
    elements.outputStats.textContent = "Output unavailable";
    elements.outputFormatNote.textContent = `${format.name} could not be emitted`;
  } else {
    outputEditor.setDoc(result.output);
    const stats = textStats(result.output);
    elements.outputStats.textContent = `${stats.lines} lines · ${formatCompactNumber(stats.characters)} characters`;
    elements.outputFormatNote.textContent = outputFilename(state.inputFilename, format.id);
  }
  renderInspectorState(result, format);
}

function renderInspectorState(result, format = getFormat(state.targetFormat)) {
  const report = result?.preservation;
  const copy = preservationCopy(report, format.name);
  elements.preservationBadge.className = `status-badge ${result ? copy.className : "idle"}`;
  elements.preservationBadge.textContent = result ? copy.badge : "Not checked";
  elements.inspectorSummary.textContent = result
    ? copy.summary
    : "Morph will emit this format, parse it again, and compare both document trees.";
  elements.changeList.replaceChildren();

  for (const change of report?.changes ?? []) {
    const description = describeFeatureChange(change);
    const item = document.createElement("li");
    const label = document.createElement("span");
    label.textContent = description.label;
    const value = document.createElement("code");
    value.textContent = description.value;
    item.append(label, value);
    elements.changeList.append(item);
  }
  elements.inspectorBody.hidden = !state.inspectorOpen;
  elements.inspectorToggle.setAttribute("aria-expanded", String(state.inspectorOpen));
}

function renderSourceStats() {
  const stats = textStats(sourceEditor?.getDoc?.() ?? state.input);
  elements.sourceStats.textContent = `${stats.lines} lines · ${formatCompactNumber(stats.characters)} characters`;
}

function getFormat(id) {
  return formats.find((format) => format.id === id) ?? FALLBACK_FORMATS[0];
}

function scheduleSave() {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => saveState(localStorage, state), 450);
}

function invalidateResults(message = "Ready for copy") {
  results = new Map();
  renderFormatRail();
  renderSelectedOutput();
  setStatus("idle", message);
}

async function convertAll() {
  if (converting) return;
  hideError();
  const input = sourceEditor.getDoc();
  const stats = textStats(input);
  if (!input.trim()) {
    showError("Nothing to convert", "Enter or open a document first.");
    return;
  }
  if (stats.bytes > MAX_INPUT_BYTES) {
    showError(
      "Document is too large",
      `This public workbench accepts 256 KiB; the current document is ${formatCompactNumber(stats.bytes)} bytes.`,
    );
    return;
  }

  converting = true;
  elements.convertButton.disabled = true;
  elements.convertButton.querySelector("span").textContent = "Setting type…";
  setStatus("working", `Parsing ${getFormat(state.inputFormat).name}`);

  try {
    const response = await fetch("/api/convert", {
      method: "POST",
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        input,
        from: state.inputFormat,
        to: formats.map((format) => format.id),
      }),
    });
    const data = await response.json().catch(() => null);
    if (!response.ok) throw normalizeApiError(data, response.status);

    results = new Map(data.results.map((result) => [result.format, result]));
    const preserved = data.results.filter(
      (result) => result.preservation?.status === "preserved",
    ).length;
    const changed = data.results.filter(
      (result) => result.preservation?.status === "changed",
    ).length;
    const errors = data.results.filter((result) => result.error).length;

    renderFormatRail();
    renderSelectedOutput();
    setStatus(
      errors ? "error" : "success",
      `${preserved} preserved · ${changed} changed${errors ? ` · ${errors} failed` : ""}`,
    );
    if (window.matchMedia("(max-width: 800px)").matches) setMobilePane("output");
  } catch (error) {
    const message =
      error?.message ??
      (navigator.onLine
        ? "The conversion service did not respond. Please try again."
        : "You appear to be offline. Reconnect before converting.");
    showError("Conversion stopped", message);
    setStatus("error", "Conversion failed");
  } finally {
    converting = false;
    elements.convertButton.disabled = false;
    elements.convertButton.querySelector("span").textContent = "Convert all ten";
  }
}

function setStatus(kind, message) {
  elements.statusLamp.className = `status-lamp ${kind === "idle" ? "" : kind}`.trim();
  elements.statusText.textContent = message;
}

function showError(title, message) {
  elements.errorTitle.textContent = title;
  elements.errorMessage.textContent = message;
  elements.errorBanner.hidden = false;
}

function hideError() {
  elements.errorBanner.hidden = true;
}

function showToast(message) {
  window.clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.add("is-visible");
  toastTimer = window.setTimeout(() => elements.toast.classList.remove("is-visible"), 2400);
}

async function copyText(value, message) {
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    const temporary = document.createElement("textarea");
    temporary.value = value;
    temporary.style.position = "fixed";
    temporary.style.opacity = "0";
    document.body.append(temporary);
    temporary.select();
    document.execCommand("copy");
    temporary.remove();
  }
  showToast(message);
}

async function openFile(file) {
  if (!file) return;
  if (file.size > MAX_INPUT_BYTES) {
    showError("File is too large", `${file.name} is larger than the 256 KiB public limit.`);
    return;
  }
  const contents = await file.text();
  const detected = detectFormat(file.name);
  state.inputFilename = file.name;
  state.input = contents;
  if (detected) {
    state.inputFormat = detected;
    elements.sourceFormat.value = detected;
    sourceEditor.setFormat(detected);
  } else {
    showToast("Format could not be inferred; keeping the current input format.");
  }
  sourceEditor.setDoc(contents);
  renderSourceStats();
  invalidateResults(`${file.name} loaded · ready to convert`);
  scheduleSave();
}

function downloadSelected() {
  const result = results.get(state.targetFormat);
  if (!result?.output) return;
  const blob = new Blob([result.output], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = outputFilename(state.inputFilename, state.targetFormat);
  document.body.append(link);
  link.click();
  link.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function useOutputAsSource() {
  const result = results.get(state.targetFormat);
  if (!result?.output) return;
  const previousSource = state.inputFormat;
  state.input = result.output;
  state.inputFormat = state.targetFormat;
  state.targetFormat =
    previousSource !== state.inputFormat
      ? previousSource
      : (formats.find((format) => format.id !== state.inputFormat)?.id ?? state.inputFormat);
  state.inputFilename = outputFilename(state.inputFilename, state.inputFormat);
  sourceEditor.setDoc(state.input);
  sourceEditor.setFormat(state.inputFormat);
  elements.sourceFormat.value = state.inputFormat;
  renderSourceStats();
  invalidateResults(`${getFormat(state.inputFormat).name} moved to source`);
  selectTarget(state.targetFormat, false);
  setMobilePane("source");
  scheduleSave();
  sourceEditor.focus();
}

function loadSpecimen() {
  state.input = SPECIMEN_SOURCE;
  state.inputFormat = "html";
  state.inputFilename = "morph-preservation-specimen.html";
  sourceEditor.setDoc(state.input);
  sourceEditor.setFormat(state.inputFormat);
  elements.sourceFormat.value = state.inputFormat;
  renderSourceStats();
  invalidateResults("Preservation specimen loaded");
  scheduleSave();
}

function clearSource() {
  state.input = "";
  state.inputFilename = "untitled.md";
  sourceEditor.setDoc("");
  renderSourceStats();
  invalidateResults("Source cleared");
  scheduleSave();
  sourceEditor.focus();
}

function setMobilePane(pane) {
  state.mobilePane = pane;
  elements.workspace.dataset.mobilePane = pane;
  for (const button of $$("[data-pane-button]")) {
    button.setAttribute("aria-selected", String(button.dataset.paneButton === pane));
  }
  scheduleSave();
}

function resolvedTheme() {
  if (state.theme !== "system") return state.theme;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function applyTheme() {
  const theme = resolvedTheme();
  document.documentElement.dataset.theme = theme;
  elements.themeToggle?.setAttribute("aria-label", `Switch to ${theme === "dark" ? "light" : "dark"} theme`);
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", theme === "dark" ? "#171714" : "#f0eadf");
}

function toggleTheme() {
  state.theme = resolvedTheme() === "dark" ? "light" : "dark";
  applyTheme();
  scheduleSave();
}

function resetLocalData() {
  window.clearTimeout(saveTimer);
  localStorage.removeItem("morph.web.v1");
  state = defaultState();
  results = new Map();
  sourceEditor.setDoc(state.input);
  sourceEditor.setFormat(state.inputFormat);
  outputEditor.setDoc("");
  outputEditor.setWrap(state.wrap);
  elements.sourceFormat.value = state.inputFormat;
  elements.wrapButton.setAttribute("aria-pressed", String(state.wrap));
  elements.workspace.dataset.mobilePane = state.mobilePane;
  for (const button of $$("[data-pane-button]")) {
    button.setAttribute("aria-selected", String(button.dataset.paneButton === state.mobilePane));
  }
  applyTheme();
  renderSourceStats();
  renderFormatRail();
  selectTarget(state.targetFormat, false);
  renderSelectedOutput();
  showToast("Local draft and preferences cleared.");
}

function renderInstallCommand() {
  elements.installCommand.textContent = installCommands[installPlatform];
  for (const button of $$("[data-install-tab]")) {
    button.setAttribute("aria-selected", String(button.dataset.installTab === installPlatform));
  }
}

elements.convertButton.addEventListener("click", convertAll);
elements.sourceFormat.addEventListener("change", () => {
  state.inputFormat = elements.sourceFormat.value;
  sourceEditor.setFormat(state.inputFormat);
  state.inputFilename = outputFilename(state.inputFilename, state.inputFormat);
  invalidateResults(`Input changed to ${getFormat(state.inputFormat).name}`);
  scheduleSave();
});
elements.openFileButton.addEventListener("click", () => elements.fileInput.click());
elements.fileInput.addEventListener("change", () => openFile(elements.fileInput.files[0]));
elements.sampleButton.addEventListener("click", loadSpecimen);
elements.clearButton.addEventListener("click", clearSource);
elements.wrapButton.addEventListener("click", () => {
  state.wrap = !state.wrap;
  outputEditor.setWrap(state.wrap);
  elements.wrapButton.setAttribute("aria-pressed", String(state.wrap));
  scheduleSave();
});
elements.copyButton.addEventListener("click", () => {
  const output = results.get(state.targetFormat)?.output;
  if (output) copyText(output, `${getFormat(state.targetFormat).name} copied.`);
});
elements.downloadButton.addEventListener("click", downloadSelected);
elements.useAsSourceButton.addEventListener("click", useOutputAsSource);
elements.inspectorToggle.addEventListener("click", () => {
  state.inspectorOpen = !state.inspectorOpen;
  renderInspectorState(results.get(state.targetFormat));
  scheduleSave();
});
elements.dismissError.addEventListener("click", hideError);
elements.themeToggle.addEventListener("click", toggleTheme);
elements.clearLocalData.addEventListener("click", resetLocalData);

for (const button of $$("[data-pane-button]")) {
  button.addEventListener("click", () => setMobilePane(button.dataset.paneButton));
}

for (const button of $$("[data-install-tab]")) {
  button.addEventListener("click", () => {
    installPlatform = button.dataset.installTab;
    renderInstallCommand();
  });
}

elements.copyInstallButton.addEventListener("click", () =>
  copyText(installCommands[installPlatform], "Install command copied."),
);

elements.sourcePanel.addEventListener("dragover", (event) => {
  event.preventDefault();
  elements.sourcePanel.classList.add("is-dragging");
});
elements.sourcePanel.addEventListener("dragleave", () =>
  elements.sourcePanel.classList.remove("is-dragging"),
);
elements.sourcePanel.addEventListener("drop", (event) => {
  event.preventDefault();
  elements.sourcePanel.classList.remove("is-dragging");
  openFile(event.dataTransfer.files[0]);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
    event.preventDefault();
    convertAll();
  }
});

window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if (state.theme === "system") applyTheme();
});
