import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it.each([false, true])("renders native link fallbacks through NAPI (transaction: %s)", batched => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `
        import { PaintCannon } from "./dist/main.js";
        const pc = new PaintCannon();
        try {
          const build = () => {
            const root = pc.createElement("div");
            root.style.width = 60;
            pc.setRoot(root);
            const link = pc.createElement("a");
            link.href = "https://example.com/docs";
            link.appendChild(pc.createTextNode("Documentation"));
            root.appendChild(link);
          };
          if (${batched}) pc.transaction(build);
          else build();
          pc.renderSync();
        } finally {
          pc.stop();
        }
      `,
    ],
    {
      cwd: fileURLToPath(new URL("..", import.meta.url)),
      encoding: "utf8",
      timeout: 5000,
      // Even a known emulator must use fallback when stdout is a pipe.
      env: { ...process.env, TMUX: "", TMUX_PANE: "", TERM_PROGRAM: "ghostty" },
    },
  );
  expect(result.error).toBeUndefined();
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain("[Documentation](https://example.com/docs)");
  expect(result.stdout).not.toContain("\x1b]8;");
});
