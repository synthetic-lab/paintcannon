import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  fps: 60,
});

const root = pc.createElement("div");
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.alignItems = "center";
root.style.justifyContent = "center";
root.style.gap = 1;
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#09090b";
root.style.color = "#f4f4f5";
pc.setRoot(root);

root.appendChild(pc.createTextNode("Group opacity over overlapping terminal cells"));

const stage = pc.createElement("div");
stage.style.position = "relative";
stage.style.width = 64;
stage.style.height = 16;
stage.style.border = "rounded";
stage.style.borderColor = "#52525b";
stage.style.backgroundColor = "#18181b";
stage.style.overflow = "hidden";
root.appendChild(stage);

const lower = pc.createElement("div");
lower.style.position = "absolute";
lower.style.left = 5;
lower.style.top = 3;
lower.style.width = 38;
lower.style.height = 8;
lower.style.display = "flex";
lower.style.alignItems = "center";
lower.style.justifyContent = "center";
lower.style.backgroundColor = "#2563eb";
lower.style.color = "white";
lower.appendChild(pc.createTextNode("LOWER LAYER"));
stage.appendChild(lower);

const upper = pc.createElement("div");
upper.style.position = "absolute";
upper.style.left = 22;
upper.style.top = 6;
upper.style.width = 36;
upper.style.height = 7;
upper.style.display = "flex";
upper.style.flexDirection = "column";
upper.style.alignItems = "center";
upper.style.justifyContent = "center";
upper.style.backgroundColor = "#f97316";
upper.style.color = "#fff7ed";
upper.appendChild(pc.createTextNode("UPPER GROUP"));
upper.appendChild(pc.createTextNode("text and background share one opacity"));
stage.appendChild(upper);

const status = pc.createTextNode("");
root.appendChild(status);
root.appendChild(pc.createTextNode("Ctrl-C exits"));

const start = performance.now();

function tick(timestamp: number): void {
  const opacity = 0.5 + Math.sin((timestamp - start) / 900) * 0.45;
  upper.style.opacity = opacity;
  status.nodeValue = `opacity: ${opacity.toFixed(2)}`;
  pc.requestAnimationFrame(tick);
}

pc.requestAnimationFrame(tick);
