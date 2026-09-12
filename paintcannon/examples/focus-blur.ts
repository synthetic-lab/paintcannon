import {
  PaintCannon,
  type InputElement,
  type PaintKeyboardEvent,
  type PaintElement,
  type PaintFocusEvent,
  type TextAreaElement,
} from "../main.ts";

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
title.appendChild(
  pc.createTextNode("Focus and bubbling: Tab / Shift-Tab for fields; click a menu to focus it."),
);

const status = pc.createElement("div");
status.style.width = 62;
status.style.height = 1;
status.style.color = "#cbd5e1";
const statusText = pc.createTextNode("waiting for focus events");
status.appendChild(statusText);

const log = pc.createElement("div");
log.style.display = "flex";
log.style.flexDirection = "column";
log.style.width = 62;
log.style.height = 6;
log.style.padding = "1 2";
log.style.border = "rounded";
log.style.borderColor = "#334155";
log.style.backgroundColor = "#020617";
log.style.color = "#cbd5e1";

const logLines = Array.from({ length: 4 }, () => {
  const line = pc.createElement("div");
  line.style.height = 1;
  const text = pc.createTextNode("");
  line.appendChild(text);
  log.appendChild(line);
  return text;
});
const events: string[] = [];

const name = field("Name", "Ada");
const command = field("Command", "paint fast");
const notes = area("Notes", "blur/focus events work for textarea too");
const genericFocusDemo = createGenericFocusDemo();

root.appendChild(title);
root.appendChild(name.row);
root.appendChild(command.row);
root.appendChild(notes.row);
root.appendChild(genericFocusDemo.modal);
root.appendChild(status);
root.appendChild(log);

name.control.focus();

pc.addEventListener("keydown", event => {
  if (event.key === "Escape") {
    pc.stop();
    process.exit(0);
  }
});

function field(labelText: string, value: string) {
  const row = rowShell(labelText);

  const control = pc.createElement("input");
  control.type = "text";
  control.value = value;
  control.cursorToEnd();
  control.style.width = 36;
  control.style.height = 3;
  styleBlurred(control);
  wireFocusEvents(labelText, control);

  row.appendChild(control);
  return { row, control };
}

function area(labelText: string, value: string) {
  const row = rowShell(labelText);

  const control = pc.createElement("textarea");
  control.value = value;
  control.cursorToEnd();
  control.style.width = 36;
  control.style.minHeight = 4;
  styleBlurred(control);
  wireFocusEvents(labelText, control);

  row.appendChild(control);
  return { row, control };
}

function rowShell(labelText: string) {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.alignItems = "center";
  row.style.gap = 2;

  const label = pc.createElement("div");
  label.style.width = 10;
  label.style.color = "#cbd5e1";
  label.appendChild(pc.createTextNode(labelText));
  row.appendChild(label);

  return row;
}

function createGenericFocusDemo() {
  const modal = pc.createElement("div");
  modal.style.display = "flex";
  modal.style.flexDirection = "column";
  modal.style.width = 62;
  modal.style.padding = "1 2";
  modal.style.border = "rounded";
  modal.style.borderColor = "#a78bfa";
  modal.style.backgroundColor = "#1e1b4b";
  modal.appendChild(pc.createTextNode("GENERIC DIV MENU (these rows are regular, focusable divs)"));

  const instructions = pc.createElement("div");
  instructions.style.display = "flex";
  instructions.style.flexDirection = "column";
  instructions.style.width = "100%";
  instructions.style.color = "#c4b5fd";
  const instructionLine1 = pc.createElement("div");
  instructionLine1.style.width = "100%";
  instructionLine1.appendChild(
    pc.createTextNode("Click MAIN MENU to focus it. Tab cycles through the text fields."),
  );
  const instructionLine2 = pc.createElement("div");
  instructionLine2.style.width = "100%";
  instructionLine2.appendChild(
    pc.createTextNode("Enter on MAIN MENU focuses SUBMENU. Escape returns; Escape again exits."),
  );
  instructions.appendChild(instructionLine1);
  instructions.appendChild(instructionLine2);
  modal.appendChild(instructions);

  const menu = pc.createElement("div");
  menu.style.width = "100%";
  styleGenericFocus(menu, "#7dd3fc");

  const menuLabel = pc.createElement("div");
  menuLabel.style.width = "100%";
  menuLabel.appendChild(pc.createTextNode("MAIN MENU   [click here]   Escape = exit"));
  menu.appendChild(menuLabel);

  const submenu = pc.createElement("div");
  submenu.style.width = "100%";
  submenu.appendChild(pc.createTextNode("SUBMENU     [click here]   Escape = return"));
  styleGenericFocus(submenu, "#fbbf24");

  // The submenu is intentionally nested inside the main menu. Its Escape handler
  // can stop bubbling; otherwise the event continues through menu and modal.
  menu.appendChild(submenu);
  modal.appendChild(menu);
  menuLabel.addEventListener("click", () => menu.focus());
  submenu.addEventListener("click", () => submenu.focus());
  menu.addEventListener("keydown", event => {
    if (event.key === "Enter") {
      submenu.focus();
      record("Main menu: opened submenu", event);
    }
  });
  submenu.addEventListener("keydown", event => {
    if (event.key === "Escape") {
      event.stopPropagation();
      menu.focus();
      record("Submenu: Escape returned to main menu", event);
    }
  });

  return { modal, menu, submenu };
}

function styleGenericFocus(element: PaintElement, color: string): void {
  // Let the text establish the row height; padding keeps the focus border visible.
  element.style.padding = "0";
  element.style.color = "#cbd5e1";
  element.style.border = "rounded";
  element.style.borderColor = "#475569";
  element.addEventListener("focus", event => {
    element.style.borderColor = color;
    element.style.color = "#f8fafc";
    record(`div ${element.id}: focus`, event);
  });
  element.addEventListener("blur", event => {
    element.style.borderColor = "#475569";
    element.style.color = "#cbd5e1";
    record(`div ${element.id}: blur`, event);
  });
}

type FocusableControl = InputElement | TextAreaElement;

function wireFocusEvents(label: string, control: FocusableControl): void {
  control.addEventListener("focus", event => {
    styleFocused(control);
    record(`${label}: focus`, event);
  });
  control.addEventListener("blur", event => {
    styleBlurred(control);
    record(`${label}: blur`, event);
  });
}

function styleFocused(control: FocusableControl): void {
  control.style.backgroundColor = "#1e293b";
  control.style.color = "#f8fafc";
  control.style.placeholderColor = "#64748b";
  control.style.border = "rounded";
  control.style.borderColor = "#38bdf8";
}

function styleBlurred(control: FocusableControl): void {
  control.style.backgroundColor = "#020617";
  control.style.color = "#e2e8f0";
  control.style.placeholderColor = "#64748b";
  control.style.border = "rounded";
  control.style.borderColor = "#475569";
}

function record(message: string, event: PaintFocusEvent | PaintKeyboardEvent): void {
  events.unshift(
    `${new Date().toLocaleTimeString()} ${message} target=${event.target?.id ?? "none"}`,
  );
  events.length = Math.min(events.length, logLines.length);
  statusText.nodeValue = `last event: ${event.type} on node ${event.target?.id ?? "none"}`;

  for (let index = 0; index < logLines.length; index += 1) {
    logLines[index].nodeValue = events[index] ?? "";
  }
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
