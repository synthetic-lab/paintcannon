import { PaintCannon } from "../dist/main.js";

let completed = false;
process.on("exit", () => {
  if (!completed) {
    console.error("active renderer did not keep the Node.js event loop alive");
    process.exitCode = 1;
  }
});

const paintCannon = new PaintCannon();
const verification = setTimeout(() => {
  completed = true;
  paintCannon.stop();
}, 100);
verification.unref();
