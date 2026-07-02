import { renderTreeTable } from "@spannerplan/core/browser";

const input = document.querySelector<HTMLTextAreaElement>("#input")!;
const output = document.querySelector<HTMLPreElement>("#output")!;
const modeSelect = document.querySelector<HTMLSelectElement>("#mode")!;
const formatSelect = document.querySelector<HTMLSelectElement>("#format")!;
const fileInput = document.querySelector<HTMLInputElement>("#file")!;
const renderButton = document.querySelector<HTMLButtonElement>("#render")!;

async function renderPlan(): Promise<void> {
  const raw = input.value;
  if (!raw.trim()) {
    output.textContent = "Paste YAML or JSON plan text, or choose a file.";
    return;
  }

  renderButton.disabled = true;
  output.textContent = "Rendering...";

  try {
    const mode = modeSelect.value as "AUTO" | "PLAN" | "PROFILE";
    const format = formatSelect.value as "TRADITIONAL" | "CURRENT" | "COMPACT";
    const result = await renderTreeTable(raw, mode, format);
    if ("error" in result) {
      output.textContent = `Error: ${result.error}`;
      return;
    }
    output.textContent = result.output;
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    output.textContent = `Error: ${message}`;
  } finally {
    renderButton.disabled = false;
  }
}

renderButton.addEventListener("click", () => {
  void renderPlan();
});

fileInput.addEventListener("change", () => {
  const file = fileInput.files?.[0];
  if (!file) {
    return;
  }
  void file.text().then((text) => {
    input.value = text;
    void renderPlan();
  });
});
