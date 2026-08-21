import {
  IntersectionObserver,
  PaintCannon,
  type KeyboardEvent,
  type PaintElement,
} from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 30,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#09090b";
root.style.color = "#f4f4f5";
pc.setRoot(root);

const header = pc.createElement("div");
header.style.display = "flex";
header.style.flexDirection = "column";
header.style.width = "100%";
header.style.height = "5";
header.style.padding = "0 1";
header.style.backgroundColor = "#18181b";
header.style.borderBottom = "heavy";
header.style.borderColor = "#3f3f46";

const title = pc.createElement("div");
title.style.fontWeight = "bold";
title.appendChild(
  pc.createTextNode("IntersectionObserver event log: mouse wheel scrolls, q exits"),
);

const eventLog = [
  pc.createTextNode("  Scroll until a blue target enters the viewport"),
  pc.createTextNode("  Recent observer events will remain visible here"),
  pc.createTextNode(""),
];
header.appendChild(title);
for (const line of eventLog) {
  header.appendChild(line);
}

const viewport = pc.createElement("div");
viewport.style.width = "100%";
viewport.style.flexGrow = 1;
viewport.style.flexShrink = 1;
viewport.style.flexBasis = 0;
viewport.style.minHeight = 0;
viewport.style.overflowY = "scroll";
viewport.style.overflowX = "hidden";
viewport.style.scrollbarGutter = "stable";
viewport.style.scrollbarColor = "#22d3ee #09090b";
viewport.style.backgroundColor = "#09090b";

const list = pc.createElement("div");
list.style.display = "flex";
list.style.flexDirection = "column";
list.style.width = "100%";
list.style.padding = "1";
list.style.gap = "1";
viewport.appendChild(list);

root.appendChild(header);
root.appendChild(viewport);

type ObservedRow = {
  index: number;
  element: PaintElement;
};

const rowsByElement = new Map<PaintElement, ObservedRow>();
type VisibilityState = "hidden" | "partial" | "full";
const visibilityByElement = new Map<PaintElement, VisibilityState>();
const recentEvents: string[] = [];
let eventCount = 0;

const observer = new IntersectionObserver(
  entries => {
    for (const entry of entries) {
      const row = rowsByElement.get(entry.target);
      if (row === undefined) {
        continue;
      }

      const previousState = visibilityByElement.get(row.element);
      const nextState = visibilityState(entry.isIntersecting, entry.intersectionRatio);
      visibilityByElement.set(row.element, nextState);

      if (nextState === "hidden") {
        if (previousState !== undefined && previousState !== "hidden") {
          recordEvent(`EXIT     blue target ${formatRow(row.index)}`);
        }
      } else if (nextState === "full") {
        if (previousState === "partial") {
          recordEvent(`FULL     blue target ${formatRow(row.index)}`);
        } else if (previousState !== "full") {
          recordEvent(`ENTER    blue target ${formatRow(row.index)} (fully visible)`);
        }
      } else {
        const percentage = Math.round(entry.intersectionRatio * 100);
        const event = previousState === "hidden" ? "ENTER" : "PARTIAL";
        recordEvent(
          `${event.padEnd(8)} blue target ${formatRow(row.index)} (${percentage}% visible)`,
        );
      }
    }
  },
  {
    root: viewport,
    threshold: [0, 0.25, 0.5, 0.75, 1],
  },
);

pc.transaction(() => {
  for (let index = 1; index <= 30; index += 1) {
    const isObserved = index % 5 === 0;
    const row = pc.createElement("div");
    row.style.display = "flex";
    row.style.flexDirection = "column";
    row.style.alignItems = "center";
    row.style.justifyContent = "center";
    row.style.width = "100%";
    row.style.height = isObserved ? "10" : "2";
    row.style.flexShrink = 0;
    row.style.padding = "0 1";
    row.style.border = "solid";
    row.style.borderColor = isObserved ? "#38bdf8" : "#3f3f46";
    row.style.backgroundColor = isObserved ? "#075985" : "#18181b";
    row.style.color = isObserved ? "#f0f9ff" : "#a1a1aa";

    row.appendChild(
      pc.createTextNode(
        isObserved
          ? `BLUE OBSERVER TARGET ${formatRow(index)} (10 rows tall)`
          : `Ordinary row ${formatRow(index)} (not observed)`,
      ),
    );
    list.appendChild(row);

    if (isObserved) {
      rowsByElement.set(row, { index, element: row });
      observer.observe(row);
    }
  }
});

pc.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.key !== "q") {
    return;
  }
  event.preventDefault();
  observer.disconnect();
  pc.stop();
  process.exit(0);
});

function visibilityState(isIntersecting: boolean, ratio: number): VisibilityState {
  if (!isIntersecting || ratio === 0) {
    return "hidden";
  }
  return ratio >= 1 ? "full" : "partial";
}

function recordEvent(message: string): void {
  eventCount += 1;
  recentEvents.unshift(`${String(eventCount).padStart(3, "0")}  ${message}`);
  recentEvents.splice(eventLog.length);

  for (const [index, line] of eventLog.entries()) {
    line.nodeValue = recentEvents[index] ?? "";
  }
}

function formatRow(index: number): string {
  return String(index).padStart(2, "0");
}
