import { PaintCannon } from "../index.ts";

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.gap = "2px";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";
pc.setRoot(root);

const hidden = box("overflow:hidden", "blue", "hidden");
hidden.appendChild(
  pc.createTextNode(
    "This line is intentionally much wider than the blue box and should be clipped.",
  ),
);

const visible = box("overflow:visible", "green", "visible");
visible.appendChild(
  pc.createTextNode(
    "This line is intentionally much wider than the green box and should spill out.",
  ),
);

root.appendChild(hidden);
root.appendChild(visible);

pc.requestAnimationFrame(() => {
  pc.requestAnimationFrame(() => pc.stop());
});

function box(label: string, color: string, overflow: "hidden" | "visible") {
  const element = pc.createElement("div");
  element.style.width = "28px";
  element.style.height = "3px";
  element.style.backgroundColor = color;
  element.style.overflow = overflow;
  element.appendChild(pc.createTextNode(label));
  return element;
}
