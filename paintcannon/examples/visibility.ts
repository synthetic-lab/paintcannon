import { PaintCannon, type PaintElement } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  fps: 30,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.gap = "1";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
root.style.color = "white";
pc.setRoot(root);

const frameText = pc.createTextNode("frame 0");

const stage = pc.createElement("div");
stage.style.display = "flex";
stage.style.flexDirection = "column";
stage.style.gap = "1";
stage.style.width = "96";
stage.style.height = "17";
stage.style.padding = "1 2";
stage.style.border = "chunky-rounded";
stage.style.borderColor = "#334155";
stage.style.backgroundColor = "#0f172a";

const stableRow = pc.createElement("div");
stableRow.style.display = "flex";
stableRow.style.gap = "1";
stableRow.style.width = "100%";
stableRow.style.height = "3";
stableRow.appendChild(chip("always", "#1d4ed8"));
stableRow.appendChild(chip("visible", "#047857"));
stableRow.appendChild(chip("layout", "#7c3aed"));

const blinkRow = pc.createElement("div");
blinkRow.style.display = "flex";
blinkRow.style.gap = "1";
blinkRow.style.width = "100%";
blinkRow.style.height = "5";

const blinkers = [
  chip("alpha", "#dc2626"),
  chip("bravo", "#ea580c"),
  chip("charlie", "#ca8a04"),
  chip("delta", "#16a34a"),
  chip("echo", "#0891b2"),
];
for (const blinker of blinkers) {
  blinkRow.appendChild(blinker);
}

const hiddenParent = pc.createElement("div");
hiddenParent.style.display = "flex";
hiddenParent.style.gap = "1";
hiddenParent.style.width = "100%";
hiddenParent.style.height = "3";
hiddenParent.style.visibility = "hidden";
hiddenParent.style.backgroundColor = "#7f1d1d";

const hiddenChild = chip("hidden parent", "#be123c");
const visibleOverride = chip("visible override", "#f97316");
visibleOverride.style.visibility = "visible";
hiddenParent.appendChild(hiddenChild);
hiddenParent.appendChild(visibleOverride);

const track = pc.createElement("div");
track.style.display = "flex";
track.style.width = "100%";
track.style.height = "3";
track.style.backgroundColor = "#111827";
track.style.border = "solid";
track.style.borderColor = "#475569";

const movingSpacer = pc.createElement("div");
movingSpacer.style.height = "1";
movingSpacer.style.width = "0";

const marker = pc.createElement("div");
marker.style.width = "4";
marker.style.height = "1";
marker.style.backgroundColor = "#22d3ee";
marker.style.color = "black";
marker.appendChild(pc.createTextNode(">>>>"));

track.appendChild(movingSpacer);
track.appendChild(marker);

stage.appendChild(stableRow);
stage.appendChild(blinkRow);
stage.appendChild(hiddenParent);
stage.appendChild(track);

root.appendChild(pc.createTextNode("visibility: hidden / visible"));
root.appendChild(frameText);
root.appendChild(stage);
root.appendChild(pc.createTextNode("Ctrl-C exits"));

let frame = 0;

function chip(label: string, color: string): PaintElement {
  const element = pc.createElement("div");
  element.style.display = "flex";
  element.style.justifyContent = "center";
  element.style.alignItems = "center";
  element.style.width = "18";
  element.style.height = "3";
  element.style.border = "chunky-rounded";
  element.style.borderColor = color;
  element.style.backgroundColor = color;
  element.style.color = "white";
  element.appendChild(pc.createTextNode(label));
  return element;
}

function tick() {
  frame += 1;
  frameText.nodeValue = `frame ${frame}`;

  for (const [index, element] of blinkers.entries()) {
    const visible = Math.floor((frame + index * 8) / 16) % 2 === 0;
    element.style.visibility = visible ? "visible" : "hidden";
  }

  hiddenParent.style.visibility = Math.floor(frame / 45) % 2 === 0 ? "hidden" : "visible";
  visibleOverride.style.visibility = Math.floor(frame / 30) % 2 === 0 ? "visible" : "hidden";

  const offset = Math.floor((Math.sin(frame / 10) + 1) * 29);
  movingSpacer.style.width = String(offset);

  pc.requestAnimationFrame(tick);
}

pc.requestAnimationFrame(tick);
