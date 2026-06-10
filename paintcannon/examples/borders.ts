import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const title = pc.createElement("div");
title.style.height = "3";
title.style.borderBottom = "solid";
title.style.borderColor = "cyan";
title.appendChild(
  pc.createTextNode(
    "Borders: solid, double, heavy, rounded, chunky-rounded, ascii, and mixed sides",
  ),
);

const grid = pc.createElement("div");
grid.style.display = "grid";
grid.style.gridTemplateColumns = "1fr 1fr 1fr";
grid.style.gridAutoRows = "5";
grid.style.gap = "1";
grid.style.width = "100%";

root.appendChild(title);
root.appendChild(grid);

pc.transaction(() => {
  grid.appendChild(box("solid", "solid", "green"));
  grid.appendChild(box("double", "double", "yellow"));
  grid.appendChild(box("heavy", "heavy", "red"));
  grid.appendChild(box("rounded", "rounded", "cyan"));
  grid.appendChild(box("chunky-rounded", "chunky-rounded", "green"));
  grid.appendChild(box("ascii", "ascii", "white"));

  const mixed = pc.createElement("div");
  mixed.style.borderTop = "solid";
  mixed.style.borderRight = "double";
  mixed.style.borderBottom = "heavy";
  mixed.style.borderLeft = "rounded";
  mixed.style.borderColor = "magenta";
  mixed.style.backgroundColor = "blue";
  mixed.appendChild(pc.createTextNode("mixed per-side borders"));
  grid.appendChild(mixed);
});

function box(label: string, border: string, color: string) {
  const element = pc.createElement("div");
  element.style.border = border;
  element.style.borderColor = color;
  element.style.backgroundColor = "blue";
  element.appendChild(pc.createTextNode(label));
  return element;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
