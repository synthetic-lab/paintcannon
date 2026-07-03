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

const status = pc.createTextNode(
  "Percent scroll demo. Resize the terminal; wheel over the panel. Ctrl-C exits.",
);
header.appendChild(status);

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
viewport.style.scrollbarColor = "white black";

const content = pc.createElement("div");
content.style.display = "flex";
content.style.flexDirection = "column";
content.style.width = "100%";

const rowCount = 10_000;
pc.transaction(() => {
  for (let index = 1; index <= rowCount; index += 1) {
    const line = pc.createElement("div");
    line.style.width = "100%";
    line.appendChild(
      pc.createTextNode(
        `percent row ${String(index).padStart(2, "0")} - resize changes visible content`,
      ),
    );
    content.appendChild(line);
  }
});

viewport.appendChild(content);
body.appendChild(viewport);
root.appendChild(header);
root.appendChild(body);

viewport.addEventListener("scroll", event => {
  updateStatus(event.scrollTop, event.scrollHeight, viewport.clientHeight);
});

pc.addEventListener("resize", () => {
  updateStatus(viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight);
});

function updateStatus(scrollTop: number, scrollHeight: number, clientHeight: number): void {
  status.nodeValue = `scrollTop=${scrollTop}/${scrollHeight}, clientHeight=${clientHeight}`;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
