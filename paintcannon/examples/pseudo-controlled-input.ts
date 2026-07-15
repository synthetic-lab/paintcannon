import { PaintCannon, TextAreaElement, type InputElement, type KeyboardEvent } from "../main.ts";

type TextControl = InputElement | TextAreaElement;

interface ControlledField {
  input: TextControl;
  value: string;
  cursor: number;
  updates: number;
  stats: ReturnType<PaintCannon["createTextNode"]>;
}

const pc = new PaintCannon({
  alternateScreen: true,
  captureCtrlC: true,
  fps: 60,
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
title.appendChild(pc.createTextNode("core pseudo-controlled input demo"));

const subtitle = pc.createElement("div");
subtitle.style.color = "#94a3b8";
subtitle.appendChild(
  pc.createTextNode(
    "Userland prevents default, computes value, then sets value/cursor manually. Escape exits.",
  ),
);

const inputField = field("input", "type here", false);
const textareaField = field("textarea", "multi-line typing", true);

root.appendChild(title);
root.appendChild(subtitle);
root.appendChild(inputField.row);
root.appendChild(textareaField.row);

inputField.control.focus();

pc.addEventListener("keydown", event => {
  if (event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
    event.preventDefault();
    pc.stop();
    process.exit(0);
  }
});

function field(labelText: string, placeholder: string, multiline: boolean) {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.alignItems = "center";
  row.style.gap = 2;

  const label = pc.createElement("div");
  label.style.width = 10;
  label.style.color = "#cbd5e1";
  label.appendChild(pc.createTextNode(labelText));

  const input = pc.createElement(multiline ? "textarea" : "input");
  input.placeholder = placeholder;
  input.style.width = 48;
  input.style.minHeight = multiline ? 5 : 3;
  input.style.backgroundColor = "#020617";
  input.style.color = "#f8fafc";
  input.style.placeholderColor = "#64748b";
  input.style.border = "rounded";
  input.style.borderColor = "#64748b";

  const statsBox = pc.createElement("div");
  statsBox.style.width = 18;
  statsBox.style.color = "#a7f3d0";
  const stats = pc.createTextNode("updates=0");
  statsBox.appendChild(stats);

  const controlled: ControlledField = {
    input,
    value: "",
    cursor: 0,
    updates: 0,
    stats,
  };

  input.addEventListener("keydown", event => {
    if (handleControlledKey(controlled, event)) {
      event.preventDefault();
    }
  });

  row.appendChild(label);
  row.appendChild(input);
  row.appendChild(statsBox);
  return { row, control: input };
}

function handleControlledKey(field: ControlledField, event: KeyboardEvent): boolean {
  if (event.altKey || event.metaKey || event.type !== "keydown") {
    return false;
  }

  const next = editText(field.value, field.cursor, event, field.input instanceof TextAreaElement);
  if (next === undefined) {
    return false;
  }

  field.value = next.value;
  field.cursor = next.cursor;
  field.updates += 1;
  field.input.value = field.value;
  field.input.cursorPosition = field.cursor;
  field.stats.nodeValue = `updates=${field.updates} chars=${Array.from(field.value).length}`;
  return true;
}

function editText(
  value: string,
  cursor: number,
  event: KeyboardEvent,
  multiline: boolean,
): { value: string; cursor: number } | undefined {
  const chars = Array.from(value);

  if (event.ctrlKey) {
    switch (event.code) {
      case "KeyA":
        return { value, cursor: 0 };
      case "KeyE":
        return { value, cursor: chars.length };
      case "KeyB":
        return { value, cursor: clampCursor(cursor - 1, chars.length) };
      case "KeyF":
        return { value, cursor: clampCursor(cursor + 1, chars.length) };
      default:
        return undefined;
    }
  }

  switch (event.key) {
    case "Backspace":
      if (cursor === 0) {
        return { value, cursor };
      }
      chars.splice(cursor - 1, 1);
      return { value: chars.join(""), cursor: cursor - 1 };
    case "Delete":
      if (cursor >= chars.length) {
        return { value, cursor };
      }
      chars.splice(cursor, 1);
      return { value: chars.join(""), cursor };
    case "ArrowLeft":
      return { value, cursor: clampCursor(cursor - 1, chars.length) };
    case "ArrowRight":
      return { value, cursor: clampCursor(cursor + 1, chars.length) };
    case "Home":
      return { value, cursor: 0 };
    case "End":
      return { value, cursor: chars.length };
    case "Enter":
      return multiline ? insert(chars, cursor, "\n") : undefined;
    default:
      return event.key.length === 1 ? insert(chars, cursor, event.key) : undefined;
  }
}

function insert(chars: string[], cursor: number, text: string): { value: string; cursor: number } {
  const inserted = Array.from(text);
  chars.splice(cursor, 0, ...inserted);
  return {
    value: chars.join(""),
    cursor: cursor + inserted.length,
  };
}

function clampCursor(cursor: number, length: number): number {
  return Math.max(0, Math.min(length, cursor));
}

function tick(): void {
  pc.requestAnimationFrame(tick);
}

tick();
