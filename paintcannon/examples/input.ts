import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.width = "100%";
root.style.height = "100%";
root.style.gap = 1;
root.style.backgroundColor = "#111827";
root.style.color = "#e5e7eb";
pc.setRoot(root);

const title = pc.createElement("div");
title.style.color = "#93c5fd";
title.appendChild(pc.createTextNode("Tab and Shift-Tab move focus. Escape exits."));

const first = field("name", "", "your name");
const second = field("command", "", "render fast");

root.appendChild(title);
root.appendChild(first.row);
root.appendChild(second.row);

first.input.focus();
pc.render();

pc.addEventListener("keydown", event => {
  if (event.key === "Escape") {
    pc.stop();
    process.exit(0);
  }
});

function field(labelText: string, initialValue: string, placeholder: string) {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.alignItems = "center";
  row.style.gap = 2;

  const label = pc.createElement("div");
  label.style.width = 10;
  label.style.color = "#cbd5e1";
  label.appendChild(pc.createTextNode(labelText));

  const input = pc.createElement("input");
  input.type = "text";
  input.value = initialValue;
  input.placeholder = placeholder;
  input.cursorPosition = input.value.length;
  input.style.width = 32;
  input.style.height = 3;
  input.style.backgroundColor = "#020617";
  input.style.color = "#f8fafc";
  input.style.placeholderColor = "#64748b";
  input.style.border = "rounded";
  input.style.borderColor = "#64748b";

  row.appendChild(label);
  row.appendChild(input);
  return { row, input };
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
