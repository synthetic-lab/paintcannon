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
viewport.style.width = "88%";
viewport.style.height = "100%";
viewport.style.overflowY = "scroll";
viewport.style.overflowX = "hidden";
viewport.style.backgroundColor = "blue";

const content = pc.createElement("div");
content.style.display = "flex";
content.style.flexDirection = "column";
content.style.width = "46px";
content.style.height = "24px";

for (let index = 1; index <= 24; index += 1) {
  const line = pc.createElement("div");
  line.style.width = "46px";
  line.style.height = "1px";
  line.appendChild(
    pc.createTextNode(`row ${String(index).padStart(2, "0")} - scroll containers are app-rendered`),
  );
  content.appendChild(line);
}

viewport.appendChild(content);

const scrollbarRail = pc.createElement("div");
scrollbarRail.style.whiteSpace = "pre";
const scrollbar = pc.createTextNode(scrollbarText(0));
scrollbarRail.appendChild(scrollbar);

viewport.addEventListener("scroll", event => {
  updateScrollbar(event.scrollTop, event.scrollHeight, event.scrollLeft, event.scrollWidth);
});

pc.addEventListener("resize", () => {
  updateScrollbar(
    viewport.scrollTop,
    viewport.scrollHeight,
    viewport.scrollLeft,
    viewport.scrollWidth,
  );
});

row.appendChild(viewport);
row.appendChild(scrollbarRail);
root.appendChild(status);
root.appendChild(row);

function updateScrollbar(
  scrollTop: number,
  scrollHeight: number,
  scrollLeft: number,
  scrollWidth: number,
): void {
  status.nodeValue = `scrollTop=${scrollTop}/${scrollHeight}, scrollLeft=${scrollLeft}/${scrollWidth}`;
  scrollbar.nodeValue = scrollbarText(scrollTop, scrollHeight, viewport.clientHeight);
}

function scrollbarText(scrollTop: number, scrollHeight = 24, clientHeight = 9): string {
  const max = Math.max(1, scrollHeight - clientHeight);
  const thumb = Math.min(8, Math.floor((scrollTop / max) * 8));
  let text = "";
  for (let row = 0; row < 9; row += 1) {
    text += row === thumb ? "#" : "|";
    if (row < 8) {
      text += "\n";
    }
  }
  return text;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
