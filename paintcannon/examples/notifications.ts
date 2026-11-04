import { PaintCannon, type Notification } from "../main.ts";

const pc = new PaintCannon({ alternateScreen: true, captureMouse: true });
const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.gap = 1;
root.style.padding = 2;
root.style.width = "100%";
root.style.height = "100%";
root.style.color = "white";
root.style.backgroundColor = "#18181b";
pc.setRoot(root);

const heading = pc.createElement("div");
heading.style.fontWeight = "bold";
heading.appendChild(pc.createTextNode("Desktop notifications"));
root.appendChild(heading);
const status = pc.createTextNode("");
root.appendChild(status);
const actions = pc.createElement("div");
actions.style.display = "flex";
actions.style.flexWrap = "wrap";
actions.style.gap = 2;
root.appendChild(actions);
const history = pc.createElement("div");
history.style.whiteSpace = "pre-wrap";
history.style.overflowY = "scroll";
history.style.flexGrow = 1;
const log = pc.createTextNode("");
history.appendChild(log);
root.appendChild(history);
const entries: string[] = [];
let latest: Notification | undefined;
let number = 0;
let delayed: ReturnType<typeof setTimeout> | undefined;

function record(message: string): void {
  entries.unshift(message);
  entries.length = Math.min(entries.length, 12);
  log.nodeValue = entries.join("\n");
}

function send(): void {
  number += 1;
  latest = new pc.Notification(`PaintCannon notification ${number}`, {
    body: "Your terminal application has finished its work.",
    tag: "paintcannon-demo",
  });
  for (const type of ["show", "click", "close", "error"] as const) {
    latest.addEventListener(type, event =>
      record(
        `${event.target.title}: ${event.type}${event.error ? ` (${event.error.message})` : ""}`,
      ),
    );
  }
}

function button(label: string, action: () => void): void {
  const element = pc.createElement("button");
  element.type = "button";
  element.style.border = "pill";
  element.style.padding = "0 1";
  element.style.backgroundColor = "#155e75";
  element.style.borderColor = "#155e75";
  element.style.cursor = "pointer";
  element.appendChild(pc.createTextNode(label));
  element.addEventListener("mouseenter", () => {
    element.style.backgroundColor = "#0e7490";
    element.style.borderColor = "#0e7490";
  });
  element.addEventListener("mouseleave", () => {
    element.style.backgroundColor = "#155e75";
    element.style.borderColor = "#155e75";
  });
  element.addEventListener("click", action);
  actions.appendChild(element);
}

button("Send now", send);
button("Send in 3 seconds", () => {
  clearTimeout(delayed);
  record("Notification scheduled");
  delayed = setTimeout(send, 3000);
});
pc.addEventListener("keydown", event => {
  if (event.key === "q" || event.key === "Escape") {
    clearTimeout(delayed);
    pc.stop();
  }
});
status.nodeValue = `Permission: ${await pc.Notification.requestPermission()}`;
if (await pc.Notification.supportsClose()) {
  button("Close latest", () => latest?.close());
}
