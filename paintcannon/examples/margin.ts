import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.width = "100%";
root.style.height = "100%";
root.style.padding = "1 2";
root.style.backgroundColor = "black";
pc.setRoot(root);

const title = pc.createElement("div");
title.style.height = "3";
title.style.borderBottom = "solid";
title.style.borderColor = "cyan";
title.appendChild(pc.createTextNode("Margin demo: shorthand and per-side margins"));

const row = pc.createElement("div");
row.style.display = "flex";
row.style.flexDirection = "row";
row.style.width = "100%";
row.style.height = "10";
row.style.backgroundColor = "#202020";

const first = box("margin: 1 2", "blue");
first.style.margin = "1 2";

const second = box("marginLeft: 5", "green");
second.style.marginTop = "2";
second.style.marginLeft = "5";

const third = box("margin: 0 0 2 3", "magenta");
third.style.margin = "0 0 2 3";

row.appendChild(first);
row.appendChild(second);
row.appendChild(third);

const nested = pc.createElement("div");
nested.style.width = "62";
nested.style.height = "10";
nested.style.marginTop = "2";
nested.style.border = "solid";
nested.style.borderColor = "yellow";
nested.style.backgroundColor = "#303030";

const nestedChild = box("child margin: 1 4", "red");
nestedChild.style.width = "28";
nestedChild.style.height = "4";
nestedChild.style.margin = "1 4";
nested.appendChild(nestedChild);

const centered = box("margin-left/right: auto", "cyan");
centered.style.width = "26";
centered.style.height = "5";
centered.style.marginTop = "2";
centered.style.marginLeft = "auto";
centered.style.marginRight = "auto";

root.appendChild(title);
root.appendChild(row);
root.appendChild(nested);
root.appendChild(centered);

let frames = 0;
function tick() {
  frames += 1;
  if (frames >= 90) {
    pc.stop();
    return;
  }
  pc.requestAnimationFrame(tick);
}

tick();

function box(label: string, background: string) {
  const element = pc.createElement("div");
  element.style.width = "16";
  element.style.height = "5";
  element.style.border = "rounded";
  element.style.borderColor = "white";
  element.style.backgroundColor = background;
  element.style.padding = "1";
  element.appendChild(pc.createTextNode(label));
  return element;
}
