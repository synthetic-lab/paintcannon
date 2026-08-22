import { PaintCannon, type DivElement, type KeyboardEvent } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 30,
});

type AnchorPane = {
  shell: DivElement;
  viewport: DivElement;
  content: DivElement;
  firstRow: DivElement;
  status: ReturnType<PaintCannon["createTextNode"]>;
};

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
header.style.height = 3;
header.style.padding = "0 1";
header.style.backgroundColor = "#18181b";
header.style.borderBottom = "heavy";
header.style.borderColor = "#3f3f46";
header.appendChild(pc.createTextNode("CSS Scroll Anchoring"));
header.appendChild(
  pc.createTextNode("Rows are inserted above both views. Space pauses; wheel scrolls; q exits."),
);

const panes = pc.createElement("div");
panes.style.display = "flex";
panes.style.flexDirection = "row";
panes.style.width = "100%";
panes.style.flexGrow = 1;
panes.style.flexShrink = 1;
panes.style.flexBasis = 0;
panes.style.minHeight = 0;
panes.style.gap = 1;
panes.style.padding = 1;

const anchored = createPane("overflow-anchor: auto", "auto", "#0c4a6e", "#38bdf8");
const unanchored = createPane("overflow-anchor: none", "none", "#4c1d2f", "#fb7185");
panes.appendChild(anchored.shell);
panes.appendChild(unanchored.shell);

root.appendChild(header);
root.appendChild(panes);

let insertedRows = 0;
let paused = false;
let initialized = false;
let nextInsertAt = 0;

anchored.viewport.addEventListener("scroll", updateStatuses);
unanchored.viewport.addEventListener("scroll", updateStatuses);

pc.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.key === "q" || event.key === "Escape") {
    event.preventDefault();
    pc.stop();
    process.exit(0);
  }

  if (event.key === " ") {
    event.preventDefault();
    paused = !paused;
    updateStatuses();
  }
});

function createPane(
  titleText: string,
  overflowAnchor: "auto" | "none",
  backgroundColor: string,
  accentColor: string,
): AnchorPane {
  const shell = pc.createElement("div");
  shell.style.display = "flex";
  shell.style.flexDirection = "column";
  shell.style.flexGrow = 1;
  shell.style.flexShrink = 1;
  shell.style.flexBasis = 0;
  shell.style.minWidth = 0;
  shell.style.minHeight = 0;

  const title = pc.createElement("div");
  title.style.height = 1;
  title.style.flexShrink = 0;
  title.style.color = accentColor;
  title.style.fontWeight = "bold";
  title.appendChild(pc.createTextNode(titleText));

  const viewport = pc.createElement("div");
  viewport.style.width = "100%";
  viewport.style.flexGrow = 1;
  viewport.style.flexShrink = 1;
  viewport.style.flexBasis = 0;
  viewport.style.minHeight = 0;
  viewport.style.overflowX = "hidden";
  viewport.style.overflowY = "scroll";
  viewport.style.overflowAnchor = overflowAnchor;
  viewport.style.scrollbarGutter = "stable";
  viewport.style.scrollbarColor = `${accentColor} ${backgroundColor}`;
  viewport.style.backgroundColor = backgroundColor;
  viewport.style.border = "rounded";
  viewport.style.borderColor = accentColor;

  const content = pc.createElement("div");
  content.style.display = "flex";
  content.style.flexDirection = "column";
  content.style.width = "100%";

  const firstRow = createRow("Original row 01", 1, accentColor);
  content.appendChild(firstRow);
  pc.transaction(() => {
    for (let index = 2; index <= 100; index += 1) {
      const row = createRow(`Original row ${String(index).padStart(2, "0")}`, index, accentColor);
      content.appendChild(row);
    }
  });

  const status = pc.createTextNode("");
  const statusBar = pc.createElement("div");
  statusBar.style.height = 1;
  statusBar.style.flexShrink = 0;
  statusBar.style.color = "#a1a1aa";
  statusBar.appendChild(status);

  viewport.appendChild(content);
  shell.appendChild(title);
  shell.appendChild(viewport);
  shell.appendChild(statusBar);

  return { shell, viewport, content, firstRow, status };
}

function createRow(text: string, index: number, accentColor: string): DivElement {
  const row = pc.createElement("div");
  row.style.width = "100%";
  row.style.height = 1;
  row.style.flexShrink = 0;
  row.style.paddingLeft = 1;
  row.style.color = index % 5 === 0 ? accentColor : "#e4e4e7";
  row.appendChild(pc.createTextNode(text));
  return row;
}

function prependRow(pane: AnchorPane, accentColor: string): void {
  const row = createRow(
    `Inserted row ${String(insertedRows).padStart(2, "0")}`,
    insertedRows,
    accentColor,
  );
  pane.content.insertBefore(row, pane.firstRow);
  pane.firstRow = row;
}

function updateStatuses(): void {
  const state = paused ? "paused" : "inserting";
  anchored.status.nodeValue = `scrollTop ${anchored.viewport.scrollTop} | ${state}`;
  unanchored.status.nodeValue = `scrollTop ${unanchored.viewport.scrollTop} | ${state}`;
}

function tick(timestamp: number): void {
  if (!initialized) {
    if (
      anchored.viewport.clientHeight > 0 &&
      anchored.viewport.scrollHeight > anchored.viewport.clientHeight
    ) {
      initialized = true;
      anchored.viewport.scrollTop = 30;
      unanchored.viewport.scrollTop = 30;
      nextInsertAt = timestamp + 1_250;
      updateStatuses();
    }
  } else if (!paused && timestamp >= nextInsertAt) {
    insertedRows += 1;
    pc.transaction(() => {
      prependRow(anchored, "#38bdf8");
      prependRow(unanchored, "#fb7185");
    });
    nextInsertAt = timestamp + 1_250;
    updateStatuses();
  }

  pc.requestAnimationFrame(tick);
}

pc.requestAnimationFrame(tick);
