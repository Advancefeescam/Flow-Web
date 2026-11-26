/** A browser File is the only source for a web import; no server filesystem path is accepted. */
export function pickBrowserImportFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,.xml,.opml,.csv,.txt,.html,.htm";
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}
