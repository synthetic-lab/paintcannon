import { PaintCannon, type PaintElement } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
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

root.appendChild(pc.createTextNode("Hover a panel to fade it in. Click it to raise its z-index."));

const stage = pc.createElement("div");
stage.style.position = "relative";
stage.style.width = 72;
stage.style.height = 20;
stage.style.border = "rounded";
stage.style.borderColor = "#52525b";
stage.style.backgroundColor = "#18181b";
stage.style.overflow = "hidden";
root.appendChild(stage);

const status = pc.createTextNode("Each panel starts at opacity 0.45");
root.appendChild(status);
root.appendChild(pc.createTextNode("Ctrl-C exits"));

let nextZIndex = 4;

stage.appendChild(panel("BLUE / z-index 1", "#2563eb", 4, 2, 1));
stage.appendChild(panel("PINK / z-index 2", "#db2777", 20, 6, 2));
stage.appendChild(panel("AMBER / z-index 3", "#d97706", 35, 10, 3));

function panel(
  label: string,
  color: string,
  left: number,
  top: number,
  initialZIndex: number,
): PaintElement {
  const element = pc.createElement("div");
  element.style.position = "absolute";
  element.style.left = left;
  element.style.top = top;
  element.style.zIndex = initialZIndex;
  element.style.display = "flex";
  element.style.flexDirection = "column";
  element.style.alignItems = "center";
  element.style.justifyContent = "center";
  element.style.width = 33;
  element.style.height = 8;
  element.style.border = "chunky-rounded";
  element.style.borderColor = color;
  element.style.backgroundColor = color;
  element.style.color = "white";
  element.style.cursor = "pointer";
  element.style.opacity = 0.45;
  element.style.transition = "opacity 650ms";
  element.appendChild(pc.createTextNode(label));
  element.appendChild(pc.createTextNode("opacity transitions as one group"));

  element.addEventListener("mouseenter", () => {
    element.style.opacity = 0.95;
    status.nodeValue = `${label}: transitioning to opacity 0.95`;
  });

  element.addEventListener("mouseleave", () => {
    element.style.opacity = 0.45;
    status.nodeValue = `${label}: transitioning to opacity 0.45`;
  });

  element.addEventListener("click", event => {
    event.stopPropagation();
    const zIndex = nextZIndex++;
    element.style.zIndex = zIndex;
    status.nodeValue = `${label}: raised to z-index ${zIndex}`;
  });

  return element;
}

pc.render();
