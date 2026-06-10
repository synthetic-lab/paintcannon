import { chmodSync, mkdirSync, writeFileSync } from "node:fs";

const hookPath = new URL("../.husky/_/reference-transaction", import.meta.url);

mkdirSync(new URL("../.husky/_/", import.meta.url), { recursive: true });
writeFileSync(hookPath, '#!/usr/bin/env sh\n. "$(dirname "$0")/h"\n');
chmodSync(hookPath, 0o755);
