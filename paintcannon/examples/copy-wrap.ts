import { PaintCannon, type TextNode } from "../main.ts";

function main(): void {
  const pc = new PaintCannon({
    alternateScreen: true,
    captureMouse: true,
    captureCtrlC: true,
  });
  const root = pc.createElement("div");
  root.style.display = "flex";
  root.style.flexDirection = "column";
  root.style.gap = 1;
  root.style.padding = "1 2";
  root.style.width = "100%";
  root.style.height = "100%";
  root.style.overflowY = "scroll";
  root.style.whiteSpace = "pre-wrap";
  root.style.overflowWrap = "anywhere";
  root.style.color = "#e5e7eb";

  function addLine(text: string, color: string): TextNode {
    const row = pc.createElement("div");
    row.style.flexShrink = 0;
    row.style.color = color;
    const node = pc.createTextNode(text);
    row.appendChild(node);
    root.appendChild(row);
    return node;
  }

  addLine("Copy wrapped text", "#93c5fd");
  addLine("Drag across the sample, release to copy, then paste into this demo.", "#e5e7eb");
  addLine("r: plain text   c: blockquote   t: textarea", "#94a3b8");
  addLine("p: paragraph   b: real line breaks   u: unbroken text   k: code", "#94a3b8");
  addLine("n: narrow   w: wide   f: full width   q / Esc / Ctrl-C: quit", "#94a3b8");

  const samples = {
    p: "This is one continuous paragraph. Shrink the terminal or choose a narrower quote, then copy across several wrapped rows. Pasting should preserve the original spaces without adding any newline characters.",
    b: "This first paragraph wraps when the quote is narrow.\n\nThis second paragraph starts after a real blank line. That blank line should survive copying.",
    u: "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ",
    k: 'function example() {\n  const message = "This line is long enough to wrap in a narrow view.";\n\n  return message;\n}',
  };
  const sample = pc.createElement("div");
  sample.style.display = "flex";
  sample.style.flexShrink = 0;
  sample.style.paddingLeft = 2;
  sample.style.width = 40;
  sample.style.maxWidth = "100%";
  const marker = pc.createElement("span");
  marker.style.flexShrink = 0;
  marker.style.color = "gray";
  marker.appendChild(pc.createTextNode("│ "));
  const content = pc.createElement("span");
  content.style.fontStyle = "italic";
  const sampleText = pc.createTextNode(samples.p);
  content.appendChild(sampleText);
  const textarea = pc.createElement("textarea");
  textarea.value = samples.p;
  textarea.style.width = "100%";
  textarea.style.minWidth = 0;
  sample.appendChild(marker);
  sample.appendChild(content);
  root.appendChild(sample);

  addLine("Escaped output below shows actual newlines as \\n.", "#94a3b8");
  const copied = addLine("Last copy: (select some sample text)", "#86efac");
  const pasted = addLine(
    "Last paste: (paste here using your terminal's paste shortcut)",
    "#86efac",
  );

  pc.addEventListener("clipboardWrite", event => {
    copied.nodeValue = `${event.success ? "Last copy" : "Copy request failed"}: ${JSON.stringify(event.text)}`;
  });
  pc.addEventListener("paste", event => {
    event.preventDefault();
    pasted.nodeValue = `Last paste: ${JSON.stringify(event.clipboardData.getData("text/plain"))}`;
  });
  pc.addEventListener("keydown", event => {
    event.preventDefault();
    if (event.key === "q" || event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
      pc.stop();
      return;
    }
    switch (event.key) {
      case "p":
      case "b":
      case "u":
      case "k":
        sampleText.nodeValue = samples[event.key];
        textarea.value = samples[event.key];
        break;
      case "r":
      case "c":
      case "t":
        for (const node of [marker, content, textarea]) node.detach();
        if (event.key === "c") sample.appendChild(marker);
        sample.appendChild(event.key === "t" ? textarea : content);
        content.style.fontStyle = event.key === "c" ? "italic" : "normal";
        sample.style.paddingLeft = event.key === "c" ? 2 : 0;
        break;
      case "n":
        sample.style.width = 24;
        break;
      case "w":
        sample.style.width = 64;
        break;
      case "f":
        sample.style.width = "100%";
        break;
      default:
        return;
    }
  });
  pc.setRoot(root);
}

main();
