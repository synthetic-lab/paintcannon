import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it.each([
  "oversized",
  "stalled",
  "no-color",
  "ansi16",
  "no-tty",
  "term-only",
  "program-only",
] as const)("boots the native renderer and preserves color detection (%s)", mode => {
  const scratch = mkdtempSync(path.join(tmpdir(), "paintcannon-startup-"));
  const log = path.join(scratch, "queries");
  try {
    writeFileSync(
      path.join(scratch, "tmux"),
      `#!/bin/sh
if [ "$1" != info ]; then exit 1; fi
printf 'info\n' >> "$TMUX_TEST_LOG"
${mode === "stalled" ? "exec sleep 60" : "printf '%262144s\\n' ''; printf 'Tc: (flag) true\\n'; printf '%262144s\\n' '' >&2"}
`,
      { mode: 0o755 },
    );
    const result = spawnSync(
      process.execPath,
      [
        "--input-type=module",
        "-e",
        `import { PaintCannon } from "./dist/main.js";
          const pc = new PaintCannon();
          try {
            const root = pc.createElement("div");
            root.style.backgroundColor = "#123456";
            root.appendChild(pc.createTextNode("first frame"));
            pc.setRoot(root);
            pc.renderSync();
          } finally {
            pc.stop();
          }
          console.log("startup completed");`,
      ],
      {
        cwd: fileURLToPath(new URL("..", import.meta.url)),
        env: {
          PATH: `${scratch}${path.delimiter}${process.env.PATH ?? ""}`,
          TERM: mode === "program-only" ? "xterm-256color" : "tmux-256color",
          TERM_PROGRAM: mode === "term-only" ? "" : "tmux",
          TMUX: mode === "term-only" || mode === "program-only" ? "" : "startup-test",
          TMUX_TEST_LOG: log,
          TTY_FORCE: mode === "no-tty" ? "0" : "1",
          NO_COLOR: mode === "no-color" ? "1" : undefined,
          FORCE_COLOR: mode === "ansi16" ? "ansi16" : undefined,
          NERD_FONT: "0",
        },
        encoding: "utf8",
        detached: true,
        timeout: 2000,
        killSignal: "SIGKILL",
      },
    );
    // Also clean up the fake tmux child when testing the broken constructor.
    if (result.error !== undefined) {
      try {
        process.kill(-result.pid, "SIGKILL");
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
      }
    }
    expect(result.error).toBeUndefined();
    expect(result.status, result.stderr).toBe(0);
    expect(result.stdout).toContain("first frame");
    expect(result.stdout).toContain("startup completed");
    expect(readFileSync(log, "utf8")).toBe("info\n");
    if (mode === "no-color" || mode === "no-tty") {
      expect(result.stdout).not.toMatch(/\x1b\[(?:48;|4[0-7]m|10[0-7]m)/);
    } else if (mode === "ansi16") {
      expect(result.stdout).not.toContain("\x1b[48;");
      expect(result.stdout).toMatch(/\x1b\[(?:4[0-7]|10[0-7])m/);
    } else if (mode === "stalled") {
      expect(result.stdout).toMatch(/\x1b\[48;5;\d+m/);
    } else {
      expect(result.stdout).toContain("\x1b[48;2;18;52;86m");
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
