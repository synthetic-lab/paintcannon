import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it("accepts the pill border through NAPI without adding vertical border space", () => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `
        import { PaintCannon } from "./dist/main.js";
        const pc = new PaintCannon();
        let metrics;
        try {
          const pill = pc.createElement("div");
          pc.transaction(() => {
            pill.style.width = 7;
            pill.style.border = "pill";
            pill.appendChild(pc.createTextNode("hello"));
            pc.setRoot(pill);
          });
          pc.renderSync();
          metrics = { width: pill.clientWidth, height: pill.clientHeight };
        } finally {
          pc.stop();
        }
        console.log(JSON.stringify(metrics));
      `,
    ],
    {
      cwd: fileURLToPath(new URL("..", import.meta.url)),
      encoding: "utf8",
      timeout: 5000,
      env: { ...process.env, TMUX: "", TMUX_PANE: "", NERD_FONT: "0" },
    },
  );
  expect(result.error).toBeUndefined();
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain("hello");
  expect(result.stdout).toContain('{"width":5,"height":1}');
});
