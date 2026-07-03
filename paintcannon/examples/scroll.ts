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
root.style.gap = "1px";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const status = pc.createTextNode("Wheel over the blue panel. Ctrl-C exits.");

const row = pc.createElement("div");
row.style.display = "flex";
row.style.flexDirection = "row";
row.style.gap = "1px";
row.style.width = "52px";
row.style.height = "9px";

const viewport = pc.createElement("div");
viewport.style.width = "100%";
viewport.style.height = "100%";
viewport.style.overflowY = "scroll";
viewport.style.overflowX = "hidden";
viewport.style.backgroundColor = "blue";
viewport.style.scrollbarColor = "white black";

const content = pc.createElement("div");
content.style.display = "flex";
content.style.flexDirection = "column";
content.style.width = "100%";
content.style.height = "24px";

for (let index = 1; index <= 24; index += 1) {
  const line = pc.createElement("div");
  line.style.width = "100%";
  line.style.height = "1px";
  line.appendChild(
    pc.createTextNode(`row ${String(index).padStart(2, "0")} - native scrollbar rail`),
  );
  content.appendChild(line);
}

viewport.appendChild(content);

viewport.addEventListener("scroll", event => {
  updateStatus(event.scrollTop, event.scrollHeight, event.scrollLeft, event.scrollWidth);
});

pc.addEventListener("resize", () => {
  updateStatus(
    viewport.scrollTop,
    viewport.scrollHeight,
    viewport.scrollLeft,
    viewport.scrollWidth,
  );
});

row.appendChild(viewport);
root.appendChild(status);
root.appendChild(row);

function updateStatus(
  scrollTop: number,
  scrollHeight: number,
  scrollLeft: number,
  scrollWidth: number,
): void {
  status.nodeValue = `scrollTop=${scrollTop}/${scrollHeight}, scrollLeft=${scrollLeft}/${scrollWidth}`;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
