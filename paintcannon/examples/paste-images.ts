import { promises as fs, unlinkSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { PaintCannon, type ImageElement, type PaintFile } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureCtrlC: true,
  fps: 30,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.alignItems = "center";
root.style.width = "100%";
root.style.height = "100%";
root.style.gap = 1;
root.style.padding = "1px 2px";
root.style.backgroundColor = "#101820";
root.style.color = "#e8f1f2";
pc.setRoot(root);

const title = pc.createElement("div");
title.style.fontWeight = "bold";
title.style.color = "#7dd3fc";
title.appendChild(pc.createTextNode("Paste or drag PNG files into the input"));

const input = pc.createElement("input");
input.placeholder = "drop a PNG path here";
input.style.width = "80%";
input.style.height = 3;
input.style.padding = "0px 1px";
input.style.border = "rounded";
input.style.borderColor = "#64748b";
input.style.backgroundColor = "#020617";
input.style.color = "#f8fafc";
input.style.placeholderColor = "#64748b";

const statusText = pc.createTextNode("Waiting for an image. Press q or Escape to exit.");
const status = pc.createElement("div");
status.style.color = "#94a3b8";
status.appendChild(statusText);

const images = pc.createElement("div");
images.style.display = "flex";
images.style.flexDirection = "row";
images.style.flexWrap = "wrap";
images.style.justifyContent = "center";
images.style.alignItems = "center";
images.style.gap = 2;
images.style.width = "100%";
images.style.flexGrow = 1;
images.style.overflow = "hidden";

root.appendChild(title);
root.appendChild(input);
root.appendChild(status);
root.appendChild(images);

const temporaryFiles = new Set<string>();
let renderedImages: ImageElement[] = [];
let pasteGeneration = 0;

input.addEventListener("paste", event => {
  const pastedFiles = Array.from(event.clipboardData.files);
  if (pastedFiles.length === 0) {
    statusText.nodeValue = "No accessible image paths were detected in that paste.";
    return;
  }

  event.preventDefault();
  renderPastedImages(pastedFiles).catch(error => {
    status.style.color = "#fca5a5";
    statusText.nodeValue = error instanceof Error ? error.message : String(error);
  });
});

pc.addEventListener("keydown", event => {
  if (event.key === "q" || event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
    event.preventDefault();
    cleanupTemporaryFiles();
    pc.stop();
    process.exit(0);
  }
});

process.once("exit", cleanupTemporaryFiles);
input.focus();
pc.render();

function tick(): void {
  pc.requestAnimationFrame(tick);
}

tick();

async function renderPastedImages(files: PaintFile[]): Promise<void> {
  const unsupported = files.filter(file => file.type !== "image/png");
  if (unsupported.length > 0) {
    throw new Error(
      `PaintCannon's image renderer currently supports PNG; got ${unsupported.map(file => file.type).join(", ")}`,
    );
  }

  const generation = ++pasteGeneration;
  const sources = await Promise.all(
    files.map(async (file, index) => {
      const filePath = path.join(
        os.tmpdir(),
        `paintcannon-paste-image-${process.pid}-${generation}-${index}.png`,
      );
      await fs.writeFile(filePath, await file.bytes());
      temporaryFiles.add(filePath);
      return { file, filePath };
    }),
  );

  if (generation !== pasteGeneration) {
    for (const source of sources) {
      removeTemporaryFile(source.filePath);
    }
    return;
  }

  pc.transaction(() => {
    for (const image of renderedImages) {
      image.destroy();
    }
    renderedImages = sources.map(({ filePath }) => {
      const image = pc.createElement("img");
      image.src = filePath;
      image.style.width = 28;
      image.style.height = 10;
      image.style.imageRendering = "half-block";
      images.appendChild(image);
      return image;
    });
  });

  input.value = "";
  status.style.color = "#86efac";
  statusText.nodeValue = `Rendered ${sources.length} PNG${sources.length === 1 ? "" : "s"}: ${sources.map(({ file }) => file.name).join(", ")}`;
}

function cleanupTemporaryFiles(): void {
  for (const filePath of Array.from(temporaryFiles)) {
    removeTemporaryFile(filePath);
  }
}

function removeTemporaryFile(filePath: string): void {
  try {
    unlinkSync(filePath);
  } catch {
    // A failed cleanup should not interfere with terminal restoration.
  }
  temporaryFiles.delete(filePath);
}
