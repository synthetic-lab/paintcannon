import { PaintCannon } from "../main.ts";

async function main(): Promise<void> {
  const pc = new PaintCannon({ alternateScreen: true });
  const powerline = await pc.font.hasNerdFontSet("powerline");

  pc.transaction(() => {
    const root = pc.createElement("div");
    root.style.display = "flex";
    root.style.flexDirection = "column";
    root.style.padding = 1;
    root.style.gap = 1;
    root.style.width = "100%";
    root.style.height = "100%";
    root.style.backgroundColor = "#18181b";
    root.style.color = "#f4f4f5";
    pc.setRoot(root);

    const title = pc.createElement("div");
    title.style.fontWeight = "bold";
    title.appendChild(pc.createTextNode("paintcannon / powerline"));
    root.appendChild(title);

    const mode = pc.createElement("div");
    mode.style.color = "#a1a1aa";
    mode.appendChild(pc.createTextNode(powerline ? "NERD_FONT=1" : "NERD_FONT=0"));
    root.appendChild(mode);

    const terminal = pc.createElement("div");
    terminal.style.flexGrow = 1;
    terminal.style.whiteSpace = "pre-wrap";
    terminal.appendChild(
      pc.createTextNode("$ git status --short\n M src/main.ts\n?? notes.md\n\n$ "),
    );
    root.appendChild(terminal);

    const bar = pc.createElement("div");
    bar.style.display = "flex";
    bar.style.flexWrap = "wrap";
    bar.style.flexShrink = 0;
    root.appendChild(bar);

    const segments = [
      { text: " NORMAL ", background: "#86efac", color: "#14532d" },
      { text: " paintcannon ", background: "#3f3f46", color: "#fafafa" },
      {
        text: powerline ? " \ue0a0 main " : " branch:main ",
        background: "#c4b5fd",
        color: "#2e1065",
      },
      { text: " +2 ~1 ", background: "#67e8f9", color: "#164e63" },
    ];
    for (const [index, segment] of segments.entries()) {
      const group = pc.createElement("div");
      group.style.display = "flex";
      group.style.flexShrink = 0;
      bar.appendChild(group);

      const label = pc.createElement("div");
      label.style.backgroundColor = segment.background;
      label.style.color = segment.color;
      label.style.whiteSpace = "pre";
      label.appendChild(pc.createTextNode(segment.text));
      group.appendChild(label);

      const separator = pc.createElement("div");
      separator.style.color = segment.background;
      separator.style.backgroundColor = segments[index + 1]?.background ?? "#18181b";
      separator.appendChild(pc.createTextNode(powerline ? "\ue0b0" : "|"));
      group.appendChild(separator);
    }
  });

  pc.addEventListener("keydown", event => {
    if (event.key === "q" || event.key === "Escape") pc.stop();
  });
}

await main();
