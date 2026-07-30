// === Knowledge OS — Drag-drop file handling utility ===

const SUPPORTED_EXTENSIONS = new Set([".md", ".pdf", ".html", ".htm"]);

export interface FileDropEvent {
  paths: string[];
}

export type DropHandler = (event: FileDropEvent) => void;

/**
 * Parse a `dragover` or `drop` event and extract supported file paths
 * from `DataTransfer`.
 *
 * Chrome/Firefox encode dropped files as `file://` URIs in the data
 * transfer. This utility extracts the path component and filters for
 * supported extensions (.md, .pdf, .html, .htm).
 */
export function extractFilePaths(dataTransfer: DataTransfer): string[] {
  const paths: string[] = [];

  // Try reading files from `files` list
  const files = Array.from(dataTransfer.files);
  for (const file of files) {
    const path = (file as any).path;
    if (path) {
      paths.push(path);
    }
  }

  // Fallback: parse text/uri-list from plain text if no File.path available
  if (paths.length === 0) {
    const text = dataTransfer.getData("text/uri-list") || dataTransfer.getData("text/plain");
    if (text) {
      for (const uri of text.split("\n")) {
        const trimmed = uri.trim();
        if (trimmed.startsWith("file://")) {
          const path = decodeURIComponent(trimmed.replace("file://", ""));
          if (isSupportedFile(path)) {
            paths.push(path);
          }
        }
      }
    }
  }

  return paths;
}

/**
 * Check whether a file path has a supported extension (.md, .pdf, .html, .htm).
 */
export function isSupportedFile(path: string): boolean {
  const lower = path.toLowerCase();
  for (const ext of SUPPORTED_EXTENSIONS) {
    if (lower.endsWith(ext)) return true;
  }
  return false;
}

/**
 * Setup drag-drop zone event listeners on a given element.
 *
 * Returns a cleanup function to remove the listeners.
 */
export function setupDropZone(
  element: HTMLElement,
  onDrop: DropHandler
): () => void {
  let dragCounter = 0;

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.dataTransfer!.dropEffect = "copy";
  }

  function handleDragEnter(_e: DragEvent) {
    dragCounter++;
    element.classList.add("drag-over");
  }

  function handleDragLeave(_e: DragEvent) {
    dragCounter--;
    if (dragCounter <= 0) {
      dragCounter = 0;
      element.classList.remove("drag-over");
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragCounter = 0;
    element.classList.remove("drag-over");

    const paths = extractFilePaths(e.dataTransfer!);
    if (paths.length > 0) {
      onDrop({ paths });
    }
  }

  element.addEventListener("dragover", handleDragOver);
  element.addEventListener("dragenter", handleDragEnter);
  element.addEventListener("dragleave", handleDragLeave);
  element.addEventListener("drop", handleDrop);

  return () => {
    element.removeEventListener("dragover", handleDragOver);
    element.removeEventListener("dragenter", handleDragEnter);
    element.removeEventListener("dragleave", handleDragLeave);
    element.removeEventListener("drop", handleDrop);
  };
}
