import {
  PaintCannon,
  type InputElement,
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
  pc.createTextNode("Focus/blur events: click fields or press Tab / Shift-Tab. Escape exits."),
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

root.appendChild(title);
root.appendChild(name.row);
root.appendChild(command.row);
root.appendChild(notes.row);
root.appendChild(status);
root.appendChild(log);

name.control.focus();
pc.render();

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

type FocusableControl = InputElement | TextAreaElement;

function wireFocusEvents(label: string, control: FocusableControl): void {
  control.addEventListener("focus", event => {
    styleFocused(event.currentTarget);
    record(`${label}: focus`, event);
  });
  control.addEventListener("blur", event => {
    styleBlurred(event.currentTarget);
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

function record(message: string, event: PaintFocusEvent): void {
  events.unshift(`${new Date().toLocaleTimeString()} ${message} target=${event.target.id}`);
  events.length = Math.min(events.length, logLines.length);
  statusText.nodeValue = `last event: ${event.type} on node ${event.target.id}`;

  for (let index = 0; index < logLines.length; index += 1) {
    logLines[index].nodeValue = events[index] ?? "";
  }
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
