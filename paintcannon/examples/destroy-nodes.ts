import { PaintCannon, type DivElement } from "../main.ts";

const pc = new PaintCannon({ fps: 12 });

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#020617";
root.style.color = "#e5e7eb";
root.style.gap = 1;
pc.setRoot(root);

const status = pc.createTextNode("destroy() removes native nodes permanently");
const statusRow = pc.createElement("div");
statusRow.style.width = "100%";
statusRow.style.height = 1;
statusRow.style.backgroundColor = "#1e293b";
statusRow.style.color = "#bfdbfe";
statusRow.appendChild(status);
root.appendChild(statusRow);

const list = pc.createElement("div");
list.style.display = "flex";
list.style.flexDirection = "column";
list.style.width = "100%";
list.style.gap = 1;
root.appendChild(list);

const cards: DivElement[] = [];
for (let index = 0; index < 8; index += 1) {
  const card = pc.createElement("div");
  card.style.width = "100%";
  card.style.height = 3;
  card.style.border = "rounded";
  card.style.borderColor = index % 2 === 0 ? "#38bdf8" : "#a78bfa";
  card.style.backgroundColor = index % 2 === 0 ? "#0f172a" : "#111827";
  card.style.color = "#f8fafc";
  card.appendChild(pc.createTextNode(` node ${index + 1} is alive `));
  list.appendChild(card);
  cards.push(card);
}

let frame = 0;
let destroyed = 0;

function tick() {
  frame += 1;

  if (frame % 12 === 0 && cards.length > 0) {
    const card = cards.shift();
    card?.destroy();
    destroyed += 1;
    status.nodeValue = `destroyed ${destroyed}/8 nodes`;
  }

  if (destroyed < 8 || frame < 110) {
    pc.requestAnimationFrame(tick);
  } else {
    status.nodeValue = "all nodes destroyed";
    setTimeout(() => pc.stop(), 500);
  }
}

pc.requestAnimationFrame(tick);
