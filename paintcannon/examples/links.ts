import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({ alternateScreen: true, captureMouse: true });

pc.transaction(() => {
  const root = pc.createElement("div");
  root.style.padding = "1 2";
  root.style.width = "100%";
  root.style.overflowWrap = "anywhere";
  pc.setRoot(root);

  const heading = pc.createElement("div");
  heading.style.fontWeight = "bold";
  heading.style.marginBottom = 1;
  heading.appendChild(pc.createTextNode("PaintCannon links"));
  root.appendChild(heading);

  for (const [label, href, detail] of [
    ["PaintCannon", "https://github.com/synthetic-lab/paintcannon", "Terminal UI rendering"],
    ["OSC 8 adoption", "https://github.com/Alhadis/OSC8-Adoption/", "Terminal hyperlink support"],
    ["Taffy", "https://github.com/DioxusLabs/taffy", "Layout engine"],
  ]) {
    const row = pc.createElement("div");
    row.style.marginBottom = 1;
    const anchor = pc.createElement("a");
    anchor.href = href;
    anchor.style.color = "#60a5fa";
    anchor.style.textDecorationLine = "underline";
    anchor.appendChild(pc.createTextNode(label));
    row.appendChild(anchor);
    row.appendChild(pc.createTextNode(" - " + detail));
    root.appendChild(row);
  }
});

pc.addEventListener("keydown", event => {
  if (event.key === "q") pc.stop();
});
