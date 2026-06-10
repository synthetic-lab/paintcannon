import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 60,
});

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

const status = pc.createTextNode("hover a button");
root.appendChild(status);
root.appendChild(button("background + text", "blue", "white", "cyan", "black"));
root.appendChild(button("border + fill", "#402080", "white", "#20d0ff", "black"));

function button(label: string, base: string, baseText: string, hover: string, hoverText: string) {
  const element = pc.createElement("div");
  element.style.width = "28";
  element.style.height = "5";
  element.style.border = "chunky-rounded";
  element.style.backgroundColor = base;
  element.style.borderColor = base;
  element.style.color = baseText;
  element.style.cursor = "pointer";
  element.style.transition = "background-color 220ms, border-color 220ms, color 220ms";
  element.appendChild(pc.createTextNode(`  ${label}  `));

  element.addEventListener("mouseenter", () => {
    element.style.backgroundColor = hover;
    element.style.borderColor = hover;
    element.style.color = hoverText;
  });

  element.addEventListener("mouseleave", () => {
    element.style.backgroundColor = base;
    element.style.borderColor = base;
    element.style.color = baseText;
  });

  element.addEventListener("transitionstart", event => {
    status.nodeValue = `${label}: ${event.propertyName} started`;
  });

  element.addEventListener("transitionend", event => {
    status.nodeValue = `${label}: ${event.propertyName} ended`;
  });

  return element;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
