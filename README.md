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
- Unicode-aware text wrapping with `overflow-wrap: normal | break-word | anywhere` and
  `word-break: normal | break-all | keep-all | break-word`; both default to `normal`
- Overflow hidden and scroll, with native mouse scrolling
- CSS scroll anchoring, with `overflow-anchor: auto | none`
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
- `IntersectionObserver` notifications based on viewport and scroll-container geometry

It also exposes a non-standard set of border styles, since typical
pixel-based borders can't be rendered in terminals:

- `none`
- `solid`
- `double`
- `heavy`
- `rounded`
- `chunky-rounded`
- `chunky`
- `chunky-best-effort` (attempts chunky-rounded, and chunky as a fallback)
- `pill` (left/right caps only, with Nerd Font rounding and square fallbacks)
- `ascii`

PaintCannon supports the following HTML elements:

- `<div>`
- `<span>`
- `<a>` with `href`, using native OSC 8 hyperlinks in supported terminals
  (for unsupported terminals, links are rendered Markdown-style as
  `[label](url)`)
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
- Focus and blur events (if inside tmux, requires the tmux `focus-events`
  setting to be enabled)
- Transition start and end events
- Scroll events

PaintCannon also supports the `requestAnimationFrame` DOM function to run
animations on a loop synced with paints.

Much like browsers are ways to paper over sticky platform differences between
different operating systems, PaintCannon provides a single application format
that papers over differences in underlying terminal emulators, like:

#### Glyph progressive enhancement

Paintcannon ships a few terminal-specific progressive enhancement queries for
font support:

- `pc.font.hasNerdFontSet(fontSetString): Promise<boolean>`. See [Nerd Font
  detection](paintcannon/README.md#nerd-font-detection) for the supported sets
  and SSH/tmux behavior. The `chunky` borders use Powerline Extended triangles
  when detected.

- `pc.font.hasSymbolsForLegacyComputing(): Promise<boolean>` detects known
  built-in legacy glyph support independently of `NERD_FONT`. `chunky-rounded`
  uses those rounded corners when supported.

Attached clients can change — for example, a `tmux` session running on a server
might have different clients attach and detach — so Paintcannon also publishes
events you can subscribe to when these capabilities change:

- `pc.font.addEventListener("nerdfont-powerline-change", event => { /* event.supported */ })`
  (replace `powerline` with any supported set)
- `pc.font.addEventListener("symbols-for-legacy-computing-change")`

Note that these checks are best-effort and conservative. We use standard env
vars such as checking `NERD_FONT=1`, and also attempt to check the
currently-connected terminal emulator against whitelists of terminal emulators
known to ship fallbacks such as Ghostty, Kitty, WezTerm, VTE-based terminals
like Gnome, and others, and finally use the Rio Glyph Protocol to attempt to
check glyph support, but many older terminals do not support the Glyph Protocol
and don't ship bundled fallbacks, so we may not successfully determine a
terminal supports these glyphs even if the user has configured a font that does
support them. We do however support piercing tmux's veil and finding the true
connected terminal behind tmux, as well as (default) SSH connections, so that
the results aren't false negatives simply due to using tmux or SSH.

Treat these as progressive enhancement hints, and make sure that the
non-enhanced version of your app still is reasonable!

#### OSC 8 clickable link progressive enhancement

PaintCannon will automatically render `<a>` tags in supported terminals using
the OSC 8 clickable link standard, and will fall back to Markdown-style
`[label](url)` rendering in unsupported terminals.

## React support

We also ship `paintcannon-react`, a React reconciler that sits on top of
PaintCannon's DOM API, for fast, React-based terminal rendering with
[Ink](https://github.com/vadimdemedes/ink)-inspired hooks.

## Demos

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
