import { PaintCannon } from "../index.ts";

const pc = new PaintCannon({ fps: 10 });

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#020617";
root.style.color = "#e5e7eb";
root.style.gap = 1;
pc.setRoot(root);

const status = pc.createTextNode("detach() keeps the native node alive");
const header = pc.createElement("div");
header.style.width = "100%";
header.style.height = 1;
header.style.backgroundColor = "#1e293b";
header.style.color = "#bfdbfe";
header.appendChild(status);
root.appendChild(header);

const columns = pc.createElement("div");
columns.style.display = "flex";
columns.style.flexDirection = "row";
columns.style.width = "100%";
columns.style.flex = "1 1 0";
root.appendChild(columns);

const left = pc.createElement("div");
left.style.display = "flex";
left.style.flexDirection = "column";
left.style.flex = "1 1 0";
left.style.height = "100%";
left.style.border = "rounded";
left.style.borderColor = "#38bdf8";
left.style.backgroundColor = "#0f172a";
left.style.gap = 1;

const right = pc.createElement("div");
right.style.display = "flex";
right.style.flexDirection = "column";
right.style.flex = "1 1 0";
right.style.height = "100%";
right.style.border = "rounded";
right.style.borderColor = "#a78bfa";
right.style.backgroundColor = "#111827";
right.style.gap = 1;

columns.appendChild(left);
columns.appendChild(right);

const leftLabel = pc.createElement("div");
leftLabel.style.width = "100%";
leftLabel.style.height = 1;
leftLabel.style.color = "#7dd3fc";
leftLabel.appendChild(pc.createTextNode(" left parent "));
left.appendChild(leftLabel);

const rightLabel = pc.createElement("div");
rightLabel.style.width = "100%";
rightLabel.style.height = 1;
rightLabel.style.color = "#c4b5fd";
rightLabel.appendChild(pc.createTextNode(" right parent "));
right.appendChild(rightLabel);

const moving = pc.createElement("div");
moving.style.width = "100%";
moving.style.height = 5;
moving.style.border = "chunky-rounded";
moving.style.borderColor = "#22c55e";
moving.style.backgroundColor = "#16a34a";
moving.style.color = "#052e16";
moving.appendChild(pc.createTextNode(" same node, different parent "));
left.appendChild(moving);

let inLeft = true;
let moves = 0;
let frame = 0;

function tick() {
  frame += 1;

  if (frame % 10 === 0) {
    if (inLeft) {
      left.detachChild(moving);
      right.appendChild(moving);
    } else {
      right.detachChild(moving);
      left.appendChild(moving);
    }
    inLeft = !inLeft;
    moves += 1;
    status.nodeValue = `detached and reattached ${moves} times`;
  }

  pc.render();

  if (moves < 10) {
    pc.requestAnimationFrame(tick);
  } else {
    status.nodeValue = "node survived every detach";
    pc.render();
    setTimeout(() => pc.stop(), 700);
  }
}

pc.requestAnimationFrame(tick);
