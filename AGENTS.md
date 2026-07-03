# PaintCannon Agent Notes

This file applies to the whole repository.

## Release Process

PaintCannon is an npm workspaces monorepo with two public packages:

- `paintcannon`
- `paintcannon-react`

Release tags are package-scoped. `paintcannon@<version>` publishes `paintcannon` and its NAPI-RS native packages. `paintcannon-react@<version>` publishes `paintcannon-react`.

To release a new version:

1. Update package versions:
   - Set `paintcannon/package.json` `version` to `<version>`.
   - Set `paintcannon-react/package.json` `version` to `<version>`.
   - Set `paintcannon-react/package.json` `peerDependencies.paintcannon` to `^<version>`.
2. Run `npm install` from the repo root so `package-lock.json` is updated.
3. Run `npm run release:check` from the repo root before tagging.
4. Commit the version bump, including both package files and `package-lock.json`.
   - Preferred commit message: `<version>`.
5. Create both release tags on that commit:
   - `git tag paintcannon@<version>`
   - `git tag paintcannon-react@<version>`
6. Push the commit and both tags:
   - `git push origin HEAD`
   - `git push origin paintcannon@<version> paintcannon-react@<version>`

The Husky `reference-transaction` hook validates release tags against package versions and runs `npm run release:check` when package release tags are created at `HEAD`. Do not rely on the hook as the only check; run `npm run release:check` manually before tagging.

Do not manually edit generated NAPI package directories. The release workflow creates them with `npm run create-npm-dirs --workspace paintcannon`, downloads native artifacts, runs `npm run artifacts --workspace paintcannon`, and publishes through GitHub Actions using the `release` environment.
