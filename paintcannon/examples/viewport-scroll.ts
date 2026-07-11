import { PaintCannon, type KeyboardEvent } from "../main.ts";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 30,
});

const root = pc.createElement("div");
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#111827";
root.style.color = "#f9fafb";
root.style.scrollbarColor = "#38bdf8 #111827";

const content = pc.createElement("div");
content.style.display = "flex";
content.style.flexDirection = "column";
content.style.width = "100%";

for (let index = 1; index <= 80; index += 1) {
  const row = pc.createElement("div");
  row.style.width = "100%";
  row.style.height = 1;
  row.style.backgroundColor = index % 2 === 0 ? "#1f2937" : "#111827";
  row.style.color = index % 5 === 0 ? "#fbbf24" : "#d1d5db";
  row.appendChild(
    pc.createTextNode(
      `${String(index).padStart(2, "0")}  Automatic alternate-screen viewport scrolling`,
    ),
  );
  content.appendChild(row);
}

root.appendChild(content);
pc.setRoot(root);

pc.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.key === "q" || event.key === "Escape") {
    event.preventDefault();
    pc.stop();
    process.exit(0);
  }
});

pc.render();
