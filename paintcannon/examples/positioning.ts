import { PaintCannon, type PaintElement } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 60,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.alignItems = "center";
root.style.justifyContent = "center";
root.style.gap = 1;
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#09090b";
root.style.color = "#f4f4f5";
pc.setRoot(root);

const status = pc.createTextNode("Click an overlapping panel to bring it forward.");
root.appendChild(status);

const stage = pc.createElement("div");
stage.style.position = "relative";
stage.style.width = 72;
stage.style.height = 20;
stage.style.border = "rounded";
stage.style.borderColor = "#52525b";
stage.style.backgroundColor = "#18181b";
stage.style.overflow = "hidden";
root.appendChild(stage);

const back = panel("BACK PANEL", "#1d4ed8", 3, 2, 1);
const front = panel("FRONT PANEL", "#be123c", 24, 6, 2);

const anchored = pc.createElement("div");
anchored.style.position = "absolute";
anchored.style.right = 2;
anchored.style.bottom = 1;
anchored.style.zIndex = 3;
anchored.style.padding = "1 2";
anchored.style.border = "chunky-rounded";
anchored.style.borderColor = "#047857";
anchored.style.backgroundColor = "#047857";
anchored.appendChild(pc.createTextNode("right: 2; bottom: 1"));

const inlineExample = pc.createElement("div");
inlineExample.style.position = "absolute";
inlineExample.style.left = 3;
inlineExample.style.bottom = 2;
inlineExample.style.zIndex = 4;
inlineExample.style.color = "#a1a1aa";
inlineExample.appendChild(pc.createTextNode("Inline flow stays here; "));
const shiftedSpan = pc.createElement("span");
shiftedSpan.style.position = "relative";
shiftedSpan.style.top = -1;
shiftedSpan.style.color = "#facc15";
shiftedSpan.style.fontWeight = "bold";
shiftedSpan.appendChild(pc.createTextNode("this span moves up"));
inlineExample.appendChild(shiftedSpan);

stage.appendChild(back);
stage.appendChild(front);
stage.appendChild(anchored);
stage.appendChild(inlineExample);

root.appendChild(pc.createTextNode("Relative, absolute, and z-index positioning. Ctrl-C exits."));
pc.render();

let nextZ = 5;

function panel(
  label: string,
  color: string,
  left: number,
  top: number,
  zIndex: number,
): PaintElement {
  const element = pc.createElement("div");
  element.style.position = "absolute";
  element.style.left = left;
  element.style.top = top;
  element.style.zIndex = zIndex;
  element.style.display = "flex";
  element.style.flexDirection = "column";
  element.style.justifyContent = "center";
  element.style.alignItems = "center";
  element.style.width = 36;
  element.style.height = 9;
  element.style.border = "chunky-rounded";
  element.style.borderColor = color;
  element.style.backgroundColor = color;
  element.style.cursor = "pointer";
  element.appendChild(pc.createTextNode(label));
  element.appendChild(pc.createTextNode(`left: ${left}; top: ${top}`));
  element.addEventListener("click", event => {
    event.stopPropagation();
    element.style.zIndex = nextZ++;
    status.nodeValue = `${label} is now on top (z-index: ${nextZ - 1}).`;
  });
  return element;
}
