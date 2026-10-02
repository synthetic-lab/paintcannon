import { PaintCannon } from "../main.ts";

const samples = [
  ["Cell ruler", "010203040506"],
  ["Bare warning", "⚠⚠⚠⚠⚠⚠"],
  ["Emoji warning", "⚠️⚠️⚠️⚠️⚠️⚠️"],
  ["Text warning", "⚠︎⚠︎⚠︎⚠︎⚠︎⚠︎"],
  ["Octopus", "🐙🐙🐙🐙🐙🐙"],
  ["Mixed", "⚠🐙❤️👩‍💻👍🏽"],
  ["Joined", "❤️‍🔥🏳️‍🌈👨‍👩‍👧‍👦"],
  ["Flags/keycaps", "🇺🇸🇧🇬1️⃣#️⃣"],
];

if (process.argv.includes("--raw")) {
  for (const [label, text] of samples) {
    process.stdout.write(`${label.padEnd(16)}${text}\n`);
  }
  process.exit(0);
}

const pc = new PaintCannon({
  alternateScreen: !process.argv.includes("--primary-screen"),
  captureCtrlC: false,
  fps: 30,
});

const root = pc.createElement("div");
pc.setRoot(root);
root.style.display = "flex";
root.style.flexDirection = "column";
root.style.padding = "1 2";
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "#111827";
root.style.color = "#f9fafb";

for (const [label, text] of samples) {
  const row = pc.createElement("div");
  row.style.display = "flex";
  row.style.flexDirection = "row";
  row.style.alignItems = "center";
  row.style.height = 3;
  row.style.flexShrink = 0;

  const name = pc.createElement("span");
  name.style.width = 16;
  name.style.color = "#93c5fd";
  name.appendChild(pc.createTextNode(label));

  const sample = pc.createElement("div");
  sample.style.display = "flex";
  sample.style.border = "rounded";
  sample.style.borderColor = "#93c5fd";
  sample.style.padding = "0 1";
  sample.style.flexShrink = 0;
  sample.appendChild(pc.createTextNode(text));

  row.appendChild(name);
  row.appendChild(sample);
  root.appendChild(row);
}

pc.addEventListener("keydown", event => {
  if (event.key === "Escape") {
    pc.stop();
    process.exit(0);
  }
});

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
