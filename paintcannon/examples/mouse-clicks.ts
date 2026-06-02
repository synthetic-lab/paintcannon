import { PaintCannon } from "../index.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

let clicks = 0;
let hovered = "nothing";

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const status = pc.createTextNode("Hover or click a colored box. Ctrl-C exits.");

const row = pc.createElement("div");
row.style.display = "flex";
row.style.flexDirection = "row";
row.style.gap = "2px";
row.style.width = "46px";
row.style.height = "3px";

const red = button("red", "red");
const green = button("green", "green");
const blue = button("blue", "blue");

row.appendChild(red);
row.appendChild(green);
row.appendChild(blue);

root.appendChild(status);
root.appendChild(row);

root.addEventListener("click", event => {
  status.nodeValue = `root saw ${event.target === event.currentTarget ? "root" : "child"} click at ${event.clientX},${event.clientY}`;
});

red.addEventListener("click", event => {
  clicks += 1;
  event.stopPropagation();
  status.nodeValue = `red handled click #${clicks}; propagation stopped`;
});
red.addEventListener("mouseenter", () => {
  hovered = "red";
  status.nodeValue = `hovering ${hovered}`;
});
red.addEventListener("mouseleave", () => {
  hovered = "nothing";
  status.nodeValue = `hovering ${hovered}`;
});

green.addEventListener("click", event => {
  clicks += 1;
  event.preventDefault();
  status.nodeValue = `green handled click #${clicks}; defaultPrevented=${event.defaultPrevented}`;
});
green.addEventListener("mouseenter", () => {
  hovered = "green";
  status.nodeValue = `hovering ${hovered}`;
});
green.addEventListener("mouseleave", () => {
  hovered = "nothing";
  status.nodeValue = `hovering ${hovered}`;
});

blue.addEventListener("click", () => {
  clicks += 1;
  status.nodeValue = `blue handled click #${clicks}`;
});
blue.addEventListener("mouseenter", () => {
  hovered = "blue";
  status.nodeValue = `hovering ${hovered}`;
});
blue.addEventListener("mouseleave", () => {
  hovered = "nothing";
  status.nodeValue = `hovering ${hovered}`;
});

function button(label: string, color: string) {
  const element = pc.createElement("div");
  element.style.display = "flex";
  element.style.justifyContent = "center";
  element.style.alignItems = "center";
  element.style.width = "14px";
  element.style.height = "3px";
  element.style.backgroundColor = color;
  element.appendChild(pc.createTextNode(label));
  return element;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
