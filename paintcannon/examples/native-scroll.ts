import { PaintCannon, type DivElement, type KeyboardEvent } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

type ScrollPane = {
  name: string;
  shell: DivElement;
  viewport: DivElement;
  title: ReturnType<PaintCannon["createTextNode"]>;
  status: ReturnType<PaintCannon["createTextNode"]>;
};

const panes: ScrollPane[] = [];
let activePaneIndex = 0;

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.gap = "1px";
root.style.backgroundColor = "#020617";
root.style.color = "#e5e7eb";
pc.setRoot(root);

const header = pc.createElement("div");
header.style.height = 1;
header.style.width = "100%";
header.style.backgroundColor = "#1e293b";
header.style.color = "#bfdbfe";
header.appendChild(
  pc.createTextNode("Native scroll: tab selects, arrows scroll, wheel/trackpad works over panes"),
);

const grid = pc.createElement("div");
grid.style.display = "flex";
grid.style.flexDirection = "column";
grid.style.width = "100%";
grid.style.flexGrow = 1;
grid.style.flexShrink = 1;
grid.style.flexBasis = 0;
grid.style.minHeight = 0;
grid.style.gap = "1px";

const topRow = paneRow();
const bottomRow = paneRow();

const vertical = createScrollablePane({
  name: "Y",
  title: "Y scroll",
  overflowX: "hidden",
  overflowY: "scroll",
  contentWidth: "100%",
  rows: 40,
  wide: false,
  background: "#082f49",
  accent: "#7dd3fc",
});

const horizontal = createScrollablePane({
  name: "X",
  title: "X scroll",
  overflowX: "scroll",
  overflowY: "hidden",
  contentWidth: "120px",
  rows: 8,
  wide: true,
  background: "#312e81",
  accent: "#c4b5fd",
});

const both = createScrollablePane({
  name: "XY",
  title: "X + Y scroll",
  overflowX: "scroll",
  overflowY: "scroll",
  contentWidth: "128px",
  rows: 36,
  wide: true,
  background: "#3f1d2b",
  accent: "#fda4af",
});

const gutter = createGutterPane();

topRow.appendChild(vertical.shell);
topRow.appendChild(horizontal.shell);
bottomRow.appendChild(both.shell);
bottomRow.appendChild(gutter);
grid.appendChild(topRow);
grid.appendChild(bottomRow);

root.appendChild(header);
root.appendChild(grid);

for (const pane of panes) {
  pane.viewport.addEventListener("click", () => {
    activePaneIndex = panes.indexOf(pane);
    updateAllStatuses();
  });
  pane.viewport.addEventListener("scroll", () => {
    updatePaneStatus(pane);
  });
}

pc.addEventListener("resize", () => {
  updateAllStatuses();
});

pc.addEventListener("keydown", (event: KeyboardEvent) => {
  const pane = panes[activePaneIndex];
  switch (event.key) {
    case "Tab":
      event.preventDefault();
      activePaneIndex = (activePaneIndex + (event.shiftKey ? panes.length - 1 : 1)) % panes.length;
      break;
    case "ArrowDown":
    case "j":
      event.preventDefault();
      pane.viewport.scrollTop += 1;
      break;
    case "ArrowUp":
    case "k":
      event.preventDefault();
      pane.viewport.scrollTop -= 1;
      break;
    case "ArrowRight":
    case "l":
      event.preventDefault();
      pane.viewport.scrollLeft += 4;
      break;
    case "ArrowLeft":
    case "h":
      event.preventDefault();
      pane.viewport.scrollLeft -= 4;
      break;
    case "Home":
      event.preventDefault();
      pane.viewport.scrollTop = 0;
      pane.viewport.scrollLeft = 0;
      break;
    case "End":
      event.preventDefault();
      pane.viewport.scrollTop = pane.viewport.scrollHeight;
      pane.viewport.scrollLeft = pane.viewport.scrollWidth;
      break;
    case "q":
    case "Escape":
      event.preventDefault();
      pc.stop();
      process.exit(0);
      break;
    default:
      return;
  }
  updateAllStatuses();
});

updateAllStatuses();

function paneRow(): DivElement {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.width = "100%";
  row.style.flexGrow = 1;
  row.style.flexShrink = 1;
  row.style.flexBasis = 0;
  row.style.minHeight = 0;
  row.style.gap = "1px";
  return row;
}

function createScrollablePane(options: {
  name: string;
  title: string;
  overflowX: "hidden" | "scroll";
  overflowY: "hidden" | "scroll";
  contentWidth: number | string;
  rows: number;
  wide: boolean;
  background: string;
  accent: string;
}): ScrollPane {
  const shell = paneShell();
  const title = pc.createTextNode("");
  const titleBar = paneTitle(title);
  const viewport = pc.createElement("div");
  viewport.style.width = "100%";
  viewport.style.flexGrow = 1;
  viewport.style.flexShrink = 1;
  viewport.style.flexBasis = 0;
  viewport.style.minHeight = 0;
  viewport.style.overflowX = options.overflowX;
  viewport.style.overflowY = options.overflowY;
  viewport.style.scrollbarColor = `${options.accent} #0f172a`;
  viewport.style.backgroundColor = options.background;
  viewport.style.color = "#f8fafc";
  viewport.style.border = "rounded";
  viewport.style.borderColor = "#475569";

  const content = pc.createElement("div");
  content.style.display = "flex";
  content.style.flexDirection = "column";
  content.style.width = options.contentWidth;
  content.style.height = `${options.rows}px`;

  for (let rowIndex = 1; rowIndex <= options.rows; rowIndex += 1) {
    const row = pc.createElement("div");
    row.style.width = "100%";
    row.style.height = 1;
    row.appendChild(
      pc.createTextNode(
        `${options.name} row ${String(rowIndex).padStart(2, "0")} ${options.wide ? "- ".repeat(36) : "vertical native scroll"}`,
      ),
    );
    content.appendChild(row);
  }

  const status = pc.createTextNode("");
  const statusBar = paneStatus(status);
  viewport.appendChild(content);
  shell.appendChild(titleBar);
  shell.appendChild(viewport);
  shell.appendChild(statusBar);

  const pane = {
    name: options.title,
    shell,
    viewport,
    title,
    status,
  };
  panes.push(pane);
  return pane;
}

function createGutterPane(): DivElement {
  const shell = paneShell();
  const title = paneTitle(pc.createTextNode("scrollbar-gutter: auto vs stable"));
  const body = pc.createElement("div");
  body.style.display = "flex";
  body.style.flexDirection = "row";
  body.style.width = "100%";
  body.style.flexGrow = 1;
  body.style.flexShrink = 1;
  body.style.flexBasis = 0;
  body.style.minHeight = 0;
  body.style.gap = "1px";

  body.appendChild(gutterBox("auto", "auto", "#064e3b"));
  body.appendChild(gutterBox("stable", "stable", "#4a044e"));

  const status = paneStatus(
    pc.createTextNode("hidden overflow; stable reserves one gutter cell without scrolling"),
  );
  shell.appendChild(title);
  shell.appendChild(body);
  shell.appendChild(status);
  return shell;
}

function gutterBox(label: string, gutter: "auto" | "stable", background: string): DivElement {
  const box = pc.createElement("div");
  box.style.width = "50%";
  box.style.height = "100%";
  box.style.overflowY = "hidden";
  box.style.scrollbarGutter = gutter;
  box.style.backgroundColor = "#0f172a";
  box.style.border = "rounded";
  box.style.borderColor = "#64748b";

  const content = pc.createElement("div");
  content.style.width = "100%";
  content.style.height = "100%";
  content.style.backgroundColor = background;
  content.style.color = "#f8fafc";
  content.appendChild(pc.createTextNode(`${label} gutter`));
  box.appendChild(content);
  return box;
}

function paneShell(): DivElement {
  const shell = pc.createElement("div");
  shell.style.display = "flex";
  shell.style.flexDirection = "column";
  shell.style.width = "50%";
  shell.style.height = "100%";
  shell.style.minHeight = 0;
  shell.style.backgroundColor = "#111827";
  return shell;
}

function paneTitle(text: ReturnType<PaintCannon["createTextNode"]>): DivElement {
  const title = pc.createElement("div");
  title.style.width = "100%";
  title.style.height = 1;
  title.style.backgroundColor = "#1f2937";
  title.style.color = "#dbeafe";
  title.appendChild(text);
  return title;
}

function paneStatus(text: ReturnType<PaintCannon["createTextNode"]>): DivElement {
  const status = pc.createElement("div");
  status.style.width = "100%";
  status.style.height = 1;
  status.style.backgroundColor = "#030712";
  status.style.color = "#cbd5e1";
  status.appendChild(text);
  return status;
}

function updateAllStatuses(): void {
  for (const pane of panes) {
    updatePaneStatus(pane);
  }
}

function updatePaneStatus(pane: ScrollPane): void {
  const active = panes[activePaneIndex] === pane ? "*" : " ";
  const maxLeft = Math.max(0, pane.viewport.scrollWidth - pane.viewport.clientWidth);
  const maxTop = Math.max(0, pane.viewport.scrollHeight - pane.viewport.clientHeight);
  pane.title.nodeValue = `${active} ${pane.name}`;
  pane.status.nodeValue = `x ${Math.min(pane.viewport.scrollLeft, maxLeft)}/${maxLeft} y ${Math.min(pane.viewport.scrollTop, maxTop)}/${maxTop} viewport ${pane.viewport.clientWidth}x${pane.viewport.clientHeight}`;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
