import { PaintCannon } from "../main.ts";

const pc = new PaintCannon({ alternateScreen: true, captureMouse: true });

pc.transaction(() => {
  const root = pc.createElement("div");
  root.style.display = "grid";
  root.style.gridTemplateColumns = "1fr 1fr 1fr";
  root.style.gridTemplateRows = "1fr 1fr 1fr";
  root.style.width = "100%";
  root.style.height = "100%";
  root.style.overflow = "hidden";
  root.style.backgroundColor = "#181818";
  pc.setRoot(root);

  for (const [vertical, alignItems] of [
    ["Top", "flex-start"],
    ["Middle", "center"],
    ["Bottom", "flex-end"],
  ] as const) {
    for (const [horizontal, justifyContent] of [
      ["left", "flex-start"],
      ["center", "center"],
      ["right", "flex-end"],
    ] as const) {
      const cell = pc.createElement("div");
      cell.style.display = "flex";
      cell.style.minWidth = 0;
      cell.style.minHeight = 0;
      cell.style.alignItems = alignItems;
      cell.style.justifyContent = justifyContent;
      root.appendChild(cell);

      const link = pc.createElement("a");
      link.href = "https://example.com";
      link.style.color = "#67e8f9";
      link.style.textDecorationLine = "underline";
      link.style.maxWidth = "100%";
      link.style.overflowWrap = "anywhere";
      link.appendChild(pc.createTextNode(`${vertical} ${horizontal}`));
      cell.appendChild(link);
    }
  }
});

pc.addEventListener("keydown", event => {
  if (event.key === "q") pc.stop();
});
