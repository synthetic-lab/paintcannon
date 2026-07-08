import { PaintCannon, type PaintCannonFocusEvent } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  fps: 30,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.gap = 1;
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#111827";
root.style.color = "#e5e7eb";
pc.setRoot(root);

const panel = pc.createElement("div");
panel.style.display = "flex";
panel.style.flexDirection = "column";
panel.style.gap = 1;
panel.style.width = 72;
panel.style.height = 12;
panel.style.padding = "1 2";
panel.style.border = "chunky-rounded";

const title = pc.createElement("div");
title.style.fontWeight = "bold";
title.appendChild(pc.createTextNode("Terminal focus detection"));

const status = pc.createElement("div");
status.style.display = "flex";
status.style.justifyContent = "center";
status.style.alignItems = "center";
status.style.width = "100%";
status.style.height = 3;
status.style.border = "rounded";
status.style.fontWeight = "bold";
const statusText = pc.createTextNode("");
status.appendChild(statusText);

const details = pc.createElement("div");
details.style.height = 3;
details.style.color = "#cbd5e1";
const detailsText = pc.createTextNode("");
details.appendChild(detailsText);

const instructions = pc.createElement("div");
instructions.style.color = "#94a3b8";
instructions.appendChild(
  pc.createTextNode("Switch to another terminal tab/window and back. Escape exits."),
);

panel.appendChild(title);
panel.appendChild(status);
panel.appendChild(details);
panel.appendChild(instructions);
root.appendChild(panel);

let events = 0;
let lastEvent = "initial";

applyFocusState(pc.hasFocus);
pc.render();

pc.addEventListener("focus", handleFocusChange);
pc.addEventListener("blur", handleFocusChange);
pc.addEventListener("keydown", event => {
  if (event.key === "Escape") {
    pc.stop();
    process.exit(0);
  }
});

function handleFocusChange(event: PaintCannonFocusEvent): void {
  events += 1;
  lastEvent = event.type;
  applyFocusState(event.hasFocus);
}

function applyFocusState(focused: boolean): void {
  root.style.backgroundColor = focused ? "#0f172a" : "#27272a";
  panel.style.backgroundColor = focused ? "#102a43" : "#18181b";
  panel.style.borderColor = focused ? "#38bdf8" : "#71717a";
  status.style.backgroundColor = focused ? "#0e7490" : "#3f3f46";
  status.style.borderColor = focused ? "#67e8f9" : "#a1a1aa";
  status.style.color = focused ? "#ecfeff" : "#e4e4e7";
  statusText.nodeValue = focused ? "FOCUSED" : "NOT FOCUSED";
  detailsText.nodeValue = `pc.hasFocus=${pc.hasFocus} last=${lastEvent} events=${events}`;
}

function tick(): void {
  detailsText.nodeValue = `pc.hasFocus=${pc.hasFocus} last=${lastEvent} events=${events}`;
  pc.requestAnimationFrame(tick);
}

pc.requestAnimationFrame(tick);
