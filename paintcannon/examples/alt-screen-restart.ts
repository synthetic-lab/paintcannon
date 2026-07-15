import { PaintCannon, type KeyboardEvent } from "../main.ts";

const first = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: true,
  fps: 30,
});

const root = first.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.width = "100%";
root.style.height = "100%";
root.style.gap = 1;
root.style.backgroundColor = "#020617";
root.style.color = "#e5e7eb";
first.setRoot(root);

const panel = first.createElement("div");
panel.style.display = "flex";
panel.style.flexDirection = "column";
panel.style.justifyContent = "center";
panel.style.alignItems = "center";
panel.style.width = 44;
panel.style.height = 7;
panel.style.border = "rounded";
panel.style.borderColor = "#38bdf8";
panel.style.backgroundColor = "#0f172a";
panel.style.gap = 1;

const title = first.createElement("div");
title.style.width = "100%";
title.style.height = 1;
title.style.color = "#bfdbfe";
title.appendChild(first.createTextNode("alt-screen PaintCannon"));

const status = first.createElement("div");
status.style.width = "100%";
status.style.height = 1;
status.appendChild(first.createTextNode("press Ctrl-C to stop and render goodbye"));

panel.appendChild(title);
panel.appendChild(status);
root.appendChild(panel);

let stopped = false;

function renderGoodbye() {
  const second = new PaintCannon({ fps: 10 });
  const goodbye = second.createElement("div");
  goodbye.style.width = "100%";
  goodbye.style.height = 1;
  goodbye.style.backgroundColor = "#111827";
  goodbye.style.color = "#f9fafb";
  goodbye.appendChild(second.createTextNode(`goodbye ${new Date().toISOString()}`));
  second.setRoot(goodbye);
  second.renderSync();
  second.stop();
}

function stopFirstAndContinue() {
  if (stopped) {
    return;
  }
  stopped = true;
  first.stop();
  renderGoodbye();
}

first.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.ctrlKey && event.code === "KeyC") {
    event.preventDefault();
    stopFirstAndContinue();
  }
});

process.once("SIGINT", () => {
  stopFirstAndContinue();
});
