import { readFileSync } from "node:fs";

const workspace = process.argv[2];
const tag = process.argv[3] ?? process.env.GITHUB_REF_NAME;

if (workspace === undefined) {
  throw new Error("usage: node scripts/verify-release-tag.mjs <workspace> [tag]");
}

if (tag === undefined) {
  throw new Error("release tag is required");
}

const packageJson = JSON.parse(
  readFileSync(new URL(`../${workspace}/package.json`, import.meta.url), "utf8"),
);
const expectedTag = `${packageJson.name}@${packageJson.version}`;

if (tag !== expectedTag) {
  throw new Error(`release tag ${tag} does not match ${workspace} package version ${expectedTag}`);
}

console.log(`Release tag ${tag} matches ${workspace}`);
