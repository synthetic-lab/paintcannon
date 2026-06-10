import { PaintCannon, type KeyboardEvent } from "../main.ts";

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
root.style.backgroundColor = "#0b1020";
root.style.color = "#dbeafe";
pc.setRoot(root);

const header = pc.createElement("div");
header.style.width = "100%";
header.style.height = 1;
header.style.backgroundColor = "#1e293b";
header.style.color = "#bfdbfe";
const title = pc.createTextNode("paintcannon less demo");
header.appendChild(title);

const body = pc.createElement("div");
body.style.display = "flex";
body.style.flexDirection = "row";
body.style.width = "100%";
body.style.flexGrow = 1;
body.style.flexShrink = 1;
body.style.flexBasis = 0;

const viewport = pc.createElement("div");
viewport.style.width = "92%";
viewport.style.height = "100%";
viewport.style.overflowY = "scroll";
viewport.style.overflowX = "hidden";
viewport.style.backgroundColor = "#020617";
viewport.style.color = "#e5e7eb";
viewport.style.border = "rounded";
viewport.style.borderColor = "#475569";
viewport.style.selectionBackgroundColor = "#334155";

const text = pc.createElement("span");
text.style.display = "inline";
text.style.color = "#e5e7eb";
text.style.whiteSpace = "pre-wrap";
text.appendChild(pc.createTextNode(makeLoremDocument()));
viewport.appendChild(text);

const rail = pc.createElement("div");
rail.style.width = "8%";
rail.style.height = "100%";
rail.style.backgroundColor = "#111827";
rail.style.color = "#93c5fd";
rail.style.borderLeft = "solid";
rail.style.borderColor = "#334155";
rail.style.whiteSpace = "pre";
const scrollbar = pc.createTextNode("");
rail.appendChild(scrollbar);

const footer = pc.createElement("div");
footer.style.width = "100%";
footer.style.height = 1;
footer.style.backgroundColor = "#1e293b";
footer.style.color = "#cbd5e1";
const status = pc.createTextNode("");
footer.appendChild(status);

body.appendChild(viewport);
body.appendChild(rail);
root.appendChild(header);
root.appendChild(body);
root.appendChild(footer);

viewport.addEventListener("scroll", () => {
  updateStatus();
});

pc.addEventListener("resize", () => {
  updateStatus();
});

pc.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.key === "Escape" || event.key === "q") {
    pc.stop();
    process.exit(0);
  }

  const page = Math.max(1, viewport.clientHeight - 1);
  switch (event.key) {
    case "ArrowDown":
    case "j":
      event.preventDefault();
      viewport.scrollTop += 1;
      break;
    case "ArrowUp":
    case "k":
      event.preventDefault();
      viewport.scrollTop -= 1;
      break;
    case "PageDown":
    case " ":
      event.preventDefault();
      viewport.scrollTop += page;
      break;
    case "PageUp":
    case "b":
      event.preventDefault();
      viewport.scrollTop -= page;
      break;
    case "Home":
    case "g":
      event.preventDefault();
      viewport.scrollTop = 0;
      break;
    case "End":
    case "G":
      event.preventDefault();
      viewport.scrollTop = viewport.scrollHeight;
      break;
  }
  updateStatus();
});

pc.render();
updateStatus();

function updateStatus(): void {
  const max = Math.max(0, viewport.scrollHeight - viewport.clientHeight);
  status.nodeValue = `row ${Math.min(viewport.scrollTop, max)}/${max}  ${viewport.clientWidth}x${viewport.clientHeight}`;
  scrollbar.nodeValue = scrollbarText(
    viewport.scrollTop,
    viewport.scrollHeight,
    viewport.clientHeight,
    rail.clientHeight,
  );
}

function scrollbarText(
  scrollTop: number,
  scrollHeight: number,
  clientHeight: number,
  railHeight: number,
): string {
  const height = Math.max(1, railHeight);
  const max = Math.max(1, scrollHeight - clientHeight);
  const thumbHeight = Math.max(1, Math.floor((clientHeight / Math.max(scrollHeight, 1)) * height));
  const thumbTop = Math.min(
    height - thumbHeight,
    Math.floor((scrollTop / max) * (height - thumbHeight)),
  );
  let text = "";
  for (let row = 0; row < height; row += 1) {
    text += row >= thumbTop && row < thumbTop + thumbHeight ? "#" : "|";
    if (row < height - 1) {
      text += "\n";
    }
  }
  return text;
}

function makeLoremDocument(): string {
  const paragraphs = [
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Integer vitae libero sed justo congue pretium, sed blandit ligula. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae.",
    "Suspendisse potenti. Donec aliquam, mi non feugiat molestie, justo massa tincidunt libero, in porttitor turpis erat nec ligula. Nunc sed arcu a neque faucibus eleifend vitae a justo.",
    "Praesent non turpis at est interdum volutpat. Proin sit amet urna sem. Cras feugiat, magna at feugiat efficitur, mauris risus ullamcorper orci, sed sollicitudin neque nisl eget lectus.",
    "Vivamus ultricies luctus mi, id facilisis arcu dignissim id. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas.",
    "Etiam sagittis feugiat ipsum, at molestie leo cursus vel. Curabitur id mi id erat varius fringilla. Aliquam erat volutpat. Duis elementum mi at orci posuere, non consequat tortor gravida.",
    "The quick brown fox jumps over the lazy dog while wide text like 界界界 and punctuation-heavy tokens stay aligned in terminal cells.",
  ];

  const sections: string[] = [];
  for (let index = 0; index < 36; index += 1) {
    sections.push(
      `${String(index + 1).padStart(2, "0")}. ${paragraphs[index % paragraphs.length]}`,
    );
  }
  return sections.join("\n\n");
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
