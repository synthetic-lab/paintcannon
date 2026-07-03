import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type PackageJson = {
  name: string;
  version: string;
  peerDependencies?: Record<string, string>;
};

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const args = process.argv.slice(2);
const versionArg = args.shift();

if (versionArg === undefined || versionArg === "--help" || versionArg === "-h") {
  printUsage();
  process.exit(versionArg === undefined ? 1 : 0);
}

if (args.length > 0) {
  fail(`Unexpected argument: ${args[0]}`);
}

const paintcannonPath = "paintcannon/package.json";
const reactPath = "paintcannon-react/package.json";
const lockPath = "package-lock.json";

const version = parseVersion(versionArg);
const paintcannonTag = `paintcannon@${version}`;
const reactTag = `paintcannon-react@${version}`;

const branch = commandOutput("git", ["branch", "--show-current"]);
if (branch !== "main") {
  fail(`Releases must be created from main. Current branch: ${branch}`);
}

if (commandOutput("git", ["status", "--porcelain"]) !== "") {
  fail("Release requires a clean worktree.");
}

ensureTagDoesNotExist(paintcannonTag);
ensureTagDoesNotExist(reactTag);

const paintcannonPackage = readPackage(paintcannonPath);
const reactPackage = readPackage(reactPath);

paintcannonPackage.version = version;
reactPackage.version = version;
reactPackage.peerDependencies ??= {};
reactPackage.peerDependencies.paintcannon = `^${version}`;

writePackage(paintcannonPath, paintcannonPackage);
writePackage(reactPath, reactPackage);

run("npm", ["install"]);
run("npm", ["run", "release:check"]);
run("git", ["add", paintcannonPath, reactPath, lockPath]);
run("git", ["commit", "-m", version]);
run("git", ["tag", paintcannonTag], {
  PAINTCANNON_RELEASE_CHECK_DONE: "1",
});
run("git", ["tag", reactTag], {
  PAINTCANNON_RELEASE_CHECK_DONE: "1",
});
run("git", ["push", "origin", branch, paintcannonTag, reactTag]);

function readPackage(path: string): PackageJson {
  return JSON.parse(readFileSync(resolve(root, path), "utf8")) as PackageJson;
}

function writePackage(path: string, contents: PackageJson): void {
  writeFileSync(resolve(root, path), `${JSON.stringify(contents, null, 2)}\n`);
}

function parseVersion(version: string): string {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
    fail(`Invalid version: ${version}`);
  }

  return version;
}

function ensureTagDoesNotExist(tag: string): void {
  const result = spawnSync("git", ["rev-parse", "--quiet", "--verify", `refs/tags/${tag}`], {
    cwd: root,
    encoding: "utf8",
  });
  if (result.status === 0) {
    fail(`Tag already exists: ${tag}`);
  }
}

function commandOutput(command: string, args: string[]): string {
  const result = spawnSync(command, args, {
    cwd: root,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    process.stderr.write(result.stderr);
    process.stderr.write(result.stdout);
    fail(`Command failed: ${command} ${args.join(" ")}`);
  }

  return result.stdout.trim();
}

function run(command: string, args: string[], env: Record<string, string> = {}): void {
  const result = spawnSync(command, args, {
    cwd: root,
    env: { ...process.env, ...env },
    stdio: "inherit",
  });

  if (result.status !== 0) {
    fail(`Command failed: ${command} ${args.join(" ")}`);
  }
}

function fail(message: string): never {
  console.error(message);
  process.exit(1);
}

function printUsage(): void {
  console.log(`Usage: npm run release -- <version>

Examples:
  npm run release -- 0.0.13`);
}
