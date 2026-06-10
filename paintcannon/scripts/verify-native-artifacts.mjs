import fs from "node:fs";
import path from "node:path";

const packageRoot = path.resolve(import.meta.dirname, "..");
const packageJson = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"));
const binaryName = packageJson.napi?.binaryName ?? "index";

const targetArtifacts = {
  "x86_64-apple-darwin": ["darwin-x64", [`${binaryName}.darwin-x64.node`]],
  "aarch64-apple-darwin": ["darwin-arm64", [`${binaryName}.darwin-arm64.node`]],
  "x86_64-pc-windows-msvc": ["win32-x64-msvc", [`${binaryName}.win32-x64-msvc.node`]],
  "aarch64-pc-windows-msvc": ["win32-arm64-msvc", [`${binaryName}.win32-arm64-msvc.node`]],
  "x86_64-unknown-linux-gnu": ["linux-x64-gnu", [`${binaryName}.linux-x64-gnu.node`]],
  "aarch64-unknown-linux-gnu": ["linux-arm64-gnu", [`${binaryName}.linux-arm64-gnu.node`]],
  "x86_64-unknown-linux-musl": ["linux-x64-musl", [`${binaryName}.linux-x64-musl.node`]],
  "aarch64-unknown-linux-musl": ["linux-arm64-musl", [`${binaryName}.linux-arm64-musl.node`]],
};

const missing = [];
for (const target of packageJson.napi?.targets ?? []) {
  const artifact = targetArtifacts[target];
  if (artifact === undefined) {
    missing.push(`No verifier mapping for NAPI target ${target}`);
    continue;
  }

  const [dir, files] = artifact;
  for (const file of files) {
    const filename = path.join(packageRoot, "npm", dir, file);
    if (!fs.existsSync(filename)) {
      missing.push(path.relative(packageRoot, filename));
    }
  }
}

if (missing.length > 0) {
  throw new Error(
    `Missing native publish artifacts:\n${missing.map(item => `  - ${item}`).join("\n")}`,
  );
}
