import { PaintCannon, type DivElement, type TextNode } from "../main.ts";

type Variant = "relative" | "explore" | "static" | "padded";

function main(): void {
  const pc = new PaintCannon({
    alternateScreen: true,
    captureMouse: true,
    captureCtrlC: true,
    fps: 30,
  });

  const root = pc.createElement("div");
  root.style.display = "flex";
  root.style.flexDirection = "column";
  root.style.gap = 1;
  root.style.padding = "1 2";
  root.style.width = "100%";
  root.style.height = "100%";
  root.style.backgroundColor = "#111827";
  root.style.color = "#e5e7eb";
  root.style.whiteSpace = "pre";
  root.style.selectionBackgroundColor = "#ff00ff";

  root.appendChild(line(pc.createTextNode("Selection across repaints"), "#93c5fd"));
  root.appendChild(
    line(
      pc.createTextNode('Drag-select "charlie", then hold still for several updates.'),
      "#e5e7eb",
    ),
  );
  root.appendChild(
    line(
      pc.createTextNode('Release while "Exploring..." is shown. Check the copied text.'),
      "#e5e7eb",
    ),
  );
  root.appendChild(
    line(pc.createTextNode("r: relative   e: heading above   s: static   p: padded"), "#94a3b8"),
  );
  root.appendChild(
    line(
      pc.createTextNode("Space: pause/resume   n: change label   q / Esc / Ctrl-C: quit"),
      "#94a3b8",
    ),
  );

  const modeText = pc.createTextNode("");
  root.appendChild(line(modeText, "#93c5fd"));

  const stage = pc.createElement("div");
  stage.style.display = "flex";
  stage.style.flexDirection = "column";
  stage.style.width = "100%";
  stage.style.height = 2;
  stage.style.flexShrink = 0;

  const message = line(
    pc.createTextNode("alpha bravo charlie delta echo foxtrot golf hotel"),
    "#e5e7eb",
  );
  message.style.display = "flex";
  const changingText = pc.createTextNode("");
  const changing = line(changingText, "#facc15");
  stage.appendChild(message);
  stage.appendChild(changing);
  root.appendChild(stage);

  const animationText = pc.createTextNode("Animation: running (label changes every 0.6 seconds)");
  root.appendChild(line(animationText, "#94a3b8"));

  const copiedText = pc.createTextNode("Last copy: (release a selection)");
  root.appendChild(line(copiedText, "#86efac"));

  let variant: Variant = "relative";
  let expanded = false;
  let paused = false;
  let lastChange = performance.now();

  function line(text: TextNode, color: string): DivElement {
    const row = pc.createElement("div");
    row.style.width = "100%";
    row.style.height = 1;
    row.style.flexShrink = 0;
    row.style.color = color;
    row.appendChild(text);
    return row;
  }

  function updateLabel(): void {
    const text = expanded ? "Exploring..." : "Exploring";
    changingText.nodeValue = variant === "padded" ? text.padEnd(12, " ") : text;
  }

  function setVariant(next: Variant): void {
    variant = next;
    expanded = false;
    lastChange = performance.now();
    message.style.position = next === "relative" ? "relative" : "static";
    if (next === "explore" || next === "padded") {
      stage.insertBefore(changing, message);
    } else {
      stage.insertBefore(message, changing);
    }
    const descriptions = {
      relative: "Relative message, changing text below (reproduction)",
      explore: "Static message, changing heading above (reproduction)",
      static: "Static message, changing text below (control)",
      padded: "Static message, constant-length heading above (control)",
    };
    modeText.nodeValue = descriptions[next];
    updateLabel();
  }

  pc.addEventListener("clipboardWrite", event => {
    copiedText.nodeValue = `Last copy: ${JSON.stringify(event.text)}`;
  });

  pc.addEventListener("keydown", event => {
    if (event.key === "q" || event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
      event.preventDefault();
      pc.stop();
      return;
    }
    switch (event.key) {
      case "r":
        setVariant("relative");
        break;
      case "e":
        setVariant("explore");
        break;
      case "s":
        setVariant("static");
        break;
      case "p":
        setVariant("padded");
        break;
      case " ":
        paused = !paused;
        lastChange = performance.now();
        animationText.nodeValue = `Animation: ${paused ? "paused " : "running"} (label changes every 0.6 seconds)`;
        break;
      case "n":
        expanded = !expanded;
        lastChange = performance.now();
        updateLabel();
        break;
      default:
        return;
    }
    event.preventDefault();
  });

  function tick(timestamp: number): void {
    if (!paused && timestamp - lastChange >= 600) {
      expanded = !expanded;
      lastChange = timestamp;
      updateLabel();
    }
    pc.requestAnimationFrame(tick);
  }

  setVariant("relative");
  pc.setRoot(root);
  pc.requestAnimationFrame(tick);
}

main();
