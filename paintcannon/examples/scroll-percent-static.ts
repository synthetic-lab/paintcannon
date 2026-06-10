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
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const header = pc.createElement("div");
header.style.width = "100%";
header.style.height = "10%";
header.style.backgroundColor = "cyan";
header.appendChild(
  pc.createTextNode("10k rows, no text updates on scroll. Wheel over the panel. Ctrl-C exits."),
);

const body = pc.createElement("div");
body.style.display = "flex";
body.style.flexDirection = "row";
body.style.width = "100%";
body.style.height = "90%";

const viewport = pc.createElement("div");
viewport.style.width = "100%";
viewport.style.height = "100%";
viewport.style.overflowY = "scroll";
viewport.style.overflowX = "hidden";
viewport.style.backgroundColor = "blue";
viewport.style.selectionBackgroundColor = "yellow";

const content = pc.createElement("div");
content.style.display = "flex";
content.style.flexDirection = "column";
content.style.width = "100%";

const rowCount = 10_000;
pc.transaction(() => {
  for (let index = 1; index <= rowCount; index += 1) {
    const line = pc.createElement("div");
    line.style.width = "100%";
    line.appendChild(pc.createTextNode(`static percent row ${String(index).padStart(5, "0")}`));
    content.appendChild(line);
  }
});

viewport.appendChild(content);
body.appendChild(viewport);
root.appendChild(header);
root.appendChild(body);

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
