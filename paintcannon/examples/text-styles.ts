import { PaintCannon, type PaintElement, type SpanElement } from "../main.ts";

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement("div");
pc.setRoot(root);
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.justifyContent = "center";
root.style.alignItems = "center";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#020617";
root.style.color = "#e5e7eb";

const panel = pc.createElement("div");
panel.style.display = "flex";
panel.style.flexDirection = "column";
panel.style.width = 60;
panel.style.gap = 1;
panel.style.padding = "1 2";
panel.style.border = "rounded";
panel.style.borderColor = "#38bdf8";
panel.style.backgroundColor = "#0f172a";

const title = pc.createElement("div");
title.style.width = "100%";
title.style.color = "#93c5fd";
title.style.fontWeight = "bold";
title.appendChild(pc.createTextNode("Text style attributes"));
panel.appendChild(title);

panel.appendChild(sampleRow("normal", sample("The quick brown fox")));

const bold = sample("The quick brown fox");
bold.style.fontWeight = "bold";
panel.appendChild(sampleRow("bold", bold));

const italic = sample("The quick brown fox");
italic.style.fontStyle = "italic";
panel.appendChild(sampleRow("italic", italic));

const underlined = sample("The quick brown fox");
underlined.style.textDecoration = "underline";
panel.appendChild(sampleRow("underline", underlined));

const combined = sample("Bold italic underlined");
combined.style.fontWeight = "bold";
combined.style.fontStyle = "italic";
combined.style.textDecoration = "underline";
panel.appendChild(sampleRow("combined", combined));

const inherited = sample("inherited ");
inherited.style.fontWeight = "bold";
inherited.style.fontStyle = "italic";
inherited.style.textDecoration = "underline";
const cleared = pc.createElement("span");
cleared.style.fontWeight = "normal";
cleared.style.fontStyle = "normal";
cleared.style.textDecoration = "none";
cleared.appendChild(pc.createTextNode("then cleared"));
inherited.appendChild(cleared);
panel.appendChild(sampleRow("inherit/reset", inherited));

root.appendChild(panel);

pc.requestAnimationFrame(() => {
  pc.requestAnimationFrame(() => pc.stop());
});

function sampleRow(label: string, value: PaintElement): PaintElement {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.width = "100%";
  row.style.height = 1;
  row.style.gap = 2;

  const name = pc.createElement("span");
  name.style.color = "#94a3b8";
  name.style.width = 14;
  name.appendChild(pc.createTextNode(label));
  row.appendChild(name);

  row.appendChild(value);
  return row;
}

function sample(text: string): SpanElement {
  const span = pc.createElement("span");
  span.style.color = "#f8fafc";
  span.appendChild(pc.createTextNode(text));
  return span;
}
