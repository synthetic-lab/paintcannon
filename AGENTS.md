# PaintCannon Agent Notes

This file applies to the whole repository.

## Release Process

PaintCannon is an npm workspaces monorepo with two public packages:

- `paintcannon`
- `paintcannon-react`

Release tags are package-scoped. `paintcannon@<version>` publishes `paintcannon` and its NAPI-RS native packages. `paintcannon-react@<version>` publishes `paintcannon-react`.

Use the release script from a clean `main` checkout:

- `npm run release -- patch`
- `npm run release -- minor`
- `npm run release -- major`
- `npm run release -- <version>`

The script does the full release. It updates both package versions, updates `paintcannon-react`'s `paintcannon` peer dependency, runs `npm install`, runs `npm run release:check`, commits the version bump, creates both package tags, and pushes the commit plus tags.

Do not manually perform the version bump/tag/push sequence unless the release script is broken. If the script is broken, fix the script first when practical.

The script requires:

- current branch is `main`
- worktree is clean
- both workspace versions match before release
- target release tags do not already exist

Generated NAPI package directories should not be edited manually. The GitHub release workflow creates them with `npm run create-npm-dirs --workspace paintcannon`, downloads native artifacts, runs `npm run artifacts --workspace paintcannon`, and publishes through GitHub Actions using the `release` environment.
