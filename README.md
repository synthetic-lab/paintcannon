Very fast, Rust-based terminal rendering for JavaScript and TypeScript.
PaintCannon exposes a DOM API subset via NAPI-RS bindings to JS and renders to
terminal backends very quickly. Think of PaintCannon as a tiny browser that
renders to a terminal backend instead of a GUI in your window manager.

![A cannon shooting paint](https://raw.githubusercontent.com/synthetic-lab/paintcannon/refs/heads/main/paintcannon-shot.png)

PaintCannon supports the following CSS:

- FlexBox
- Grid
- Block
- Inline
- Relative and absolute positioning with CSS stacking contexts and `z-index`
- Margins and padding, including auto margins
- Width and height constraints with `min-width`, `max-width`, `min-height`, and `max-height`,
  including percentage values
- Overflow hidden and scroll, with native mouse scrolling
- `visibility: hidden`, which preserves layout space while suppressing paint
  and hit testing
- CSS `opacity`, composited once for an element and its descendants as a stacking-context group
- `scrollbar-color` and `scrollbar-gutter` styling
- 24-bit RGB and CSS named background, border, text, placeholder, and selection colors with
  256-color and 16-color fallbacks
- CSS transitions for color and opacity properties
- Mouse pointer styling (in supported terminals using the kitty protocol)
- Terminal focus detection via `PaintCannon.hasFocus` and app-level
  `focus`/`blur` events

It also exposes a non-standard set of border styles, since typical
pixel-based borders can't be rendered in terminals:

- `none`
- `solid`
- `double`
- `heavy`
- `rounded`
- `chunky-rounded`
- `ascii`

PaintCannon supports the following HTML elements:

- `<div>`
- `<span>`
- `<input type="text">`
- `<textarea>`
- `<button>`
- `<form>`
- `<img>` (via ANSI/ASCII rendering)

And the following DOM events and event handlers, with bubbling,
`stopPropagation`, and `preventDefault` support:

- Click events
- Mouse enter and leave events
- Keyboard events
- Text and image paste events
- Input change events
- Form submit events
- Focus and blur events
- Transition start and end events
- Scroll events

PaintCannon also exposes a `requestAnimationFrame` function to hook into its
paint timing, much like browsers do.

Terminal focus detection uses xterm focus reporting under the hood. PaintCannon
enables the terminal protocol from Rust and exposes it as app-level
`focus`/`blur` events plus `pc.hasFocus`. Inside tmux, focus events work when
tmux has `focus-events` enabled:

```tmux
set -g focus-events on
```

We also ship `paintcannon-react`, a React reconciler that sits on top of
PaintCannon's DOM API, for fast, React-based terminal rendering with
[Ink](https://github.com/vadimdemedes/ink)-inspired hooks.

To check out the main demo, clone this repo and then:

```bash
npm run build:debug
npm run demo:todo
```

This builds the main PaintCannon API bindings, and runs the React-based TODO
app in your terminal with scrollbars, clickable buttons, keyboard shortcuts,
and more. If you want even more speed:

```bash
npm run build # release build
node ./paintcannon-react/dist/examples/todo.js
```

## Maintenance

NAPI-RS native targets are declared in `paintcannon/package.json`'s
`napi.targets`; update that list to add or remove platform builds. CI and
release jobs read the same target list when building native artifacts.

Local package release tag creation is guarded by Git's `reference-transaction`
hook, since Git does not have a `pre-tag` hook. The hook validates the
package-scoped tag name, then runs `npm run release:check` before the local tag
ref is committed. Releases use package-scoped tags: `paintcannon@0.0.1`
publishes only `paintcannon`, while `paintcannon-react@0.0.1` publishes only
`paintcannon-react`.
