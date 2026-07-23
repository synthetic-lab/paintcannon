import { PaintCannon, type CopyEvent } from "../main.ts";

const pc = new PaintCannon({ captureCtrlC: true, captureMouse: true, fps: 30 });

const root = pc.createElement("div");
pc.setRoot(root);
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";

const title = pc.createElement("div");
title.style.display = "flex";
title.style.justifyContent = "center";
title.style.alignItems = "center";
title.style.width = "100%";
title.style.height = "3px";
title.style.backgroundColor = "blue";
title.appendChild(
  pc.createTextNode("drag-select text with your mouse. release to copy. copy toast should appear."),
);

const body = pc.createElement("div");
body.style.display = "flex";
body.style.flexDirection = "column";
body.style.width = "100%";
body.style.height = "100%";
body.style.paddingLeft = "1px";
body.style.paddingRight = "1px";
body.style.backgroundColor = "black";
const lines = [
  "the quick brown fox jumps over the lazy dog",
  "paintcannon renders text you can select and copy",
  "release the mouse after dragging to fire a copy event",
  "a toast appears below when the copy event is dispatched",
];
for (const line of lines) {
  const row = pc.createElement("div");
  row.style.width = "100%";
  row.style.height = "1px";
  row.style.whiteSpace = "nowrap";
  row.appendChild(pc.createTextNode(line));
  body.appendChild(row);
}

const toastHost = pc.createElement("div");
toastHost.style.display = "flex";
toastHost.style.flexGrow = "1";
toastHost.style.width = "100%";
toastHost.style.justifyContent = "center";
toastHost.style.alignItems = "flex-end";

const toast = pc.createElement("div");
toast.style.display = "flex";
toast.style.justifyContent = "center";
toast.style.alignItems = "center";
toast.style.width = "44px";
toast.style.height = "3px";
toast.style.backgroundColor = "green";
toast.style.opacity = "0";
toast.style.transition = "opacity 600ms";
const toastText = pc.createTextNode("copied!");
toast.appendChild(toastText);

toastHost.appendChild(toast);

root.appendChild(title);
root.appendChild(body);
root.appendChild(toastHost);

let toastTimer: NodeJS.Timeout | undefined;

function showToast(text: string, success: boolean) {
  toastText.nodeValue = success
    ? `copied: ${text.length > 16 ? `${text.slice(0, 13)}...` : text}`
    : "copy sent to terminal";
  toast.style.opacity = "1";
  toast.style.backgroundColor = success ? "green" : "yellow";
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.style.opacity = "0";
  }, 5000);
}

pc.addEventListener("copy", (event: CopyEvent) => {
  showToast(event.text, event.success);
});

pc.addEventListener("keydown", event => {
  if (event.key === "q" || event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
    event.preventDefault();
    pc.stop();
    process.exit(0);
  }
});

function paint() {
  pc.requestAnimationFrame(paint);
}

pc.requestAnimationFrame(paint);
