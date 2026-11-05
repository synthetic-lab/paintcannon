import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { PaintCannon } from "../dist/main.js";

const pc = new PaintCannon({ alternateScreen: true });
const richPasteExpected = process.env.PC_CLIPBOARD_FOCUS_EVENTS !== "off";
try {
  assert.equal(await pc.supportsRichPaste(), richPasteExpected);
  const input = pc.createElement("input");
  pc.setRoot(input);
  input.focus();
  const order = [];
  let file;
  const finished = new Promise(resolve => {
    input.addEventListener("paste", event => {
      order.push("paste");
      assert.equal(
        event.clipboardData.getData("text/plain"),
        richPasteExpected ? "" : "plain text",
      );
      assert.equal(event.clipboardData.files.length, richPasteExpected ? 1 : 0);
      file = event.clipboardData.files[0];
    });
    input.addEventListener("keydown", event => {
      order.push(event.key);
      if (event.key === "b") setImmediate(resolve);
    });
  });
  process.stdout.write("CLIPBOARD_READY\n");
  await finished;
  assert.deepEqual(order, ["a", "paste", "b"]);
  assert.equal(input.value, richPasteExpected ? "ab" : "aplain textb");
  if (richPasteExpected) {
    assert.equal(file.name, "clipboard.png");
    assert.equal(file.type, "image/png");
    assert.equal(
      createHash("sha256")
        .update(await file.bytes())
        .digest("hex"),
      process.env.PC_CLIPBOARD_EXPECTED_SHA256,
    );
  }
} finally {
  pc.stop();
}
process.stdout.write("CLIPBOARD_TMUX_OK\n");
