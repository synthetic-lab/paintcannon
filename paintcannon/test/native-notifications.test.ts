import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

it.each(["success", "no-close", "failure", "ssh", "missing", "stalled"] as const)(
  "native notification fallback (%s)",
  mode => {
    const scratch = mkdtempSync(path.join(tmpdir(), "paintcannon-notification-"));
    const log = path.join(scratch, "arguments");
    const mac = process.platform === "darwin";
    try {
      if (mode !== "missing")
        writeFileSync(
          path.join(scratch, mac ? "osascript" : "notify-send"),
          `#!/bin/sh
printf '%s\\n' "$@" >> "$NOTIFICATION_LOG"
${mode === "stalled" ? "exec sleep 30" : mode === "failure" ? "exit 1" : mac ? "exit 0" : "printf '42\\n'"}
`,
          { mode: 0o755 },
        );
      if (mode !== "no-close")
        writeFileSync(
          path.join(scratch, "gdbus"),
          `#!/bin/sh\nprintf '%s\\n' "$@" >> "$NOTIFICATION_LOG"\n`,
          { mode: 0o755 },
        );
      const result = spawnSync(
        process.execPath,
        [
          "--input-type=module",
          "-e",
          `
      import assert from 'node:assert/strict';
      import { PaintCannon } from './dist/main.js';
      const pc = new PaintCannon();
      try {
        assert.equal(pc.Notification.permission, 'default');
        assert.equal(await pc.Notification.requestPermission(), ${JSON.stringify(mode === "ssh" || mode === "missing" ? "denied" : "granted")});
        assert.equal(await pc.Notification.supportsClose(), ${!mac && mode !== "no-close" && mode !== "ssh" && mode !== "missing"});
        const n = new pc.Notification('--untrusted "title"', {body: '<b>body</b>; $HOME', tag: 'test'});
        await new Promise((resolve, reject) => {
          const timer = setTimeout(() => reject(new Error('notification event missing')), 4000);
          n.addEventListener(${JSON.stringify(mode === "success" || mode === "no-close" ? "show" : "error")}, event => {
            clearTimeout(timer);
            assert.equal(event.target, n);
            resolve();
          });
        });
        if (${mode === "success"}) {
          const lifecycle = [];
          n.onclose = () => lifecycle.push('old closed');
          const replacement = new pc.Notification('replacement', {tag: 'test'});
          await new Promise(resolve => replacement.onshow = resolve);
          lifecycle.push('new shown');
          assert.deepEqual(lifecycle, ['old closed', 'new shown']);
          await new Promise(resolve => {
            replacement.onclose = resolve;
            replacement.close();
          });
        }
        console.log('notification completed');
      } finally { pc.stop(); }
    `,
        ],
        {
          cwd: fileURLToPath(new URL("..", import.meta.url)),
          env: {
            PATH:
              mode === "missing" || mode === "no-close"
                ? scratch
                : `${scratch}${path.delimiter}${process.env.PATH ?? ""}`,
            NOTIFICATION_LOG: log,
            TERM: "dumb",
            DISPLAY: ":paintcannon-test",
            DBUS_SESSION_BUS_ADDRESS: "unix:path=/paintcannon-test/no-bus",
            SSH_CONNECTION: mode === "ssh" ? "remote" : undefined,
            NERD_FONT: "0",
          },
          detached: true,
          timeout: 6000,
          killSignal: "SIGKILL",
          encoding: "utf8",
        },
      );
      if (result.error?.code === "ETIMEDOUT" && result.pid > 0) {
        try {
          process.kill(-result.pid, "SIGKILL");
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
        }
      }
      expect(result.error).toBeUndefined();
      expect(result.status, result.stderr).toBe(0);
      expect(result.stdout).toContain("notification completed");
      if (mode !== "ssh" && mode !== "missing") {
        const args = readFileSync(log, "utf8");
        expect(args).toContain('--\n--untrusted "title"\n');
        expect(args).toContain(mac ? "<b>body</b>; $HOME" : "&lt;b&gt;body&lt;/b&gt;; $HOME");
        if (!mac && mode === "success") {
          expect(args).toContain("--replace-id=42");
          expect(args).toContain("org.freedesktop.Notifications.CloseNotification\n42\n");
        }
      } else {
        expect(() => readFileSync(log)).toThrow();
      }
    } finally {
      rmSync(scratch, { recursive: true, force: true });
    }
  },
);
