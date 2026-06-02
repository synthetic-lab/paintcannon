import { PaintCannon } from "../index.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

let count = 0;
let hovering = false;
let flashToken = 0;

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.gap = "1";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const label = pc.createTextNode("Count: 0");

const button = pc.createElement("div");
button.style.display = "flex";
button.style.justifyContent = "center";
button.style.alignItems = "center";
button.style.width = "24";
button.style.height = "5";
button.style.border = "chunky-rounded";
button.style.borderColor = "blue";
button.style.backgroundColor = "blue";
button.style.color = "white";
button.style.cursor = "pointer";
button.style.transition = "background-color 100ms, border-color 100ms, color 100ms";
button.appendChild(pc.createTextNode("Increment"));

button.addEventListener("mouseenter", () => {
  hovering = true;
  applyButtonColors();
});

button.addEventListener("mouseleave", () => {
  hovering = false;
  applyButtonColors();
});

button.addEventListener("click", () => {
  count += 1;
  label.nodeValue = `Count: ${count}`;
  flashButton();
});

function applyButtonColors() {
  button.style.backgroundColor = hovering ? "cyan" : "blue";
  button.style.borderColor = hovering ? "cyan" : "blue";
  button.style.color = hovering ? "black" : "white";
}

function flashButton() {
  const token = ++flashToken;
  button.style.backgroundColor = "#f97316";
  button.style.borderColor = "#f97316";
  button.style.color = "white";

  setTimeout(() => {
    if (token === flashToken) {
      applyButtonColors();
    }
  }, 150);
}

root.appendChild(label);
root.appendChild(button);

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
