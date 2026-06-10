import { readFileSync } from "node:fs";

const paintcannonPackage = JSON.parse(
  readFileSync(new URL("../paintcannon/package.json", import.meta.url), "utf8"),
);

const targetSettings = {
  "x86_64-apple-darwin": {
    host: "macos-26-intel",
    build: "npm run build:native --workspace paintcannon -- --target x86_64-apple-darwin",
  },
  "aarch64-apple-darwin": {
    host: "macos-latest",
    build: "npm run build:native --workspace paintcannon -- --target aarch64-apple-darwin",
  },
  "x86_64-pc-windows-msvc": {
    host: "windows-latest",
    build: "npm run build:native --workspace paintcannon -- --target x86_64-pc-windows-msvc",
  },
  "aarch64-pc-windows-msvc": {
    host: "windows-11-arm",
    build: "npm run build:native --workspace paintcannon -- --target aarch64-pc-windows-msvc",
  },
  "x86_64-unknown-linux-gnu": {
    host: "ubuntu-latest",
    build: "npm run build:native --workspace paintcannon -- --target x86_64-unknown-linux-gnu",
  },
  "aarch64-unknown-linux-gnu": {
    host: "ubuntu-latest",
    build:
      "npm run build:native --workspace paintcannon -- --target aarch64-unknown-linux-gnu --use-napi-cross",
  },
  "x86_64-unknown-linux-musl": {
    host: "ubuntu-latest",
    build: "npm run build:native --workspace paintcannon -- --target x86_64-unknown-linux-musl -x",
    zig: true,
  },
  "aarch64-unknown-linux-musl": {
    host: "ubuntu-latest",
    build: "npm run build:native --workspace paintcannon -- --target aarch64-unknown-linux-musl -x",
    zig: true,
  },
};

const targets = paintcannonPackage.napi?.targets;
if (!Array.isArray(targets)) {
  throw new Error("paintcannon/package.json must define napi.targets");
}

const include = targets.map(target => {
  const settings = targetSettings[target];
  if (settings === undefined) {
    throw new Error(`No native build settings defined for NAPI-RS target ${target}`);
  }

  return { target, ...settings };
});

process.stdout.write(JSON.stringify({ include }));
