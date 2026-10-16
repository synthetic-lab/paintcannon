# paintcannon

Very fast, Rust-based terminal rendering for JavaScript and TypeScript.

`paintcannon` exposes a small DOM-like API over NAPI-RS bindings and renders to terminal
backends from Rust. Think of it as a tiny browser that renders to a VT-style terminal
instead of a GUI window.

![A cannon shooting paint](https://raw.githubusercontent.com/synthetic-lab/paintcannon/refs/heads/main/paintcannon-shot.png)

## Features

PaintCannon supports the following CSS layout and paint features:

- Flexbox
- Grid
- Block layout
- Inline layout
- Relative and absolute positioning with CSS stacking contexts and `z-index`
- Margins and padding, including auto margins
- Width and height constraints with `min-width`, `max-width`, `min-height`, and `max-height`,
  including percentage values
- Unicode-aware text wrapping with `overflow-wrap: normal | break-word | anywhere` and
  `word-break: normal | break-all | keep-all | break-word`; both default to `normal`
- Overflow hidden and scroll, with native mouse scrolling
- CSS scroll anchoring, with `overflow-anchor: auto | none`
- `visibility: hidden`, which preserves layout space while suppressing paint and hit testing
- CSS `opacity`, composited once for an element and its descendants as a stacking-context group
- 24-bit RGB and CSS named background, border, text, placeholder, and selection colors with
  256-color and 16-color fallbacks
- CSS transitions for color and opacity properties
- Mouse pointer styling in supported terminals
- Terminal focus detection via `PaintCannon.hasFocus` and app-level `focus`/`blur` events
- `IntersectionObserver` notifications based on terminal viewport and scroll-container geometry

PaintCannon also exposes terminal-specific border styles:

- `none`
- `solid`
- `double`
- `heavy`
- `rounded`
- `chunky-rounded`
- `chunky`
- `chunky-best-effort`
- `pill`
- `ascii`

Positioned elements support `position: static | relative | absolute`, the `top`, `right`,
`bottom`, and `left` inset properties, and integer or `auto` `z-index` values. Absolute elements
use the nearest non-static ancestor as their containing block and do not contribute to normal flow.
Positioned inline elements retain their inline flow position while their painted fragments move.

`overflow-wrap` and `word-break` are inherited. PaintCannon follows CSS intrinsic sizing semantics:
`overflow-wrap: anywhere` contributes emergency break opportunities to min-content sizing, while
`overflow-wrap: break-word` does not. The language-aware `word-break` values `manual` and
`auto-phrase` are not currently supported.

Scroll anchoring is enabled by default. When layout changes move visible content inside a vertically
scrolled container, PaintCannon adjusts `scrollTop` to keep the selected anchor stable and dispatches
a `scroll` event. Set `overflow-anchor: none` on a scroll container to disable anchoring, or on a
descendant to exclude that subtree from anchor selection. Explicit scrolling selects a new anchor;
containers at `scrollTop === 0` are not anchored. This follows the
[CSS Scroll Anchoring specification](https://drafts.csswg.org/css-scroll-anchoring/) using terminal
cells as the layout unit.

Opacity accepts numbers or percentages and is clamped to the CSS range from `0` to `1`. PaintCannon
queries supporting terminals for their default foreground and background colors so translucent
content can blend correctly when a color resolves to the terminal default. Run
`npm run demo:opacity` to see group opacity over overlapping text and backgrounds.

## Elements

The core DOM subset supports:

- `div`
- `span`
- `a` with `href`
- `input` with `type: "text"`
- `textarea`
- `button`
- `form`
- `img` via ANSI, ASCII, and half-block rendering
- text nodes

Textareas expose the non-standard `getCursorVisualPosition()` method for terminal editors. It
returns the cursor's zero-based `{row, column}` in the soft-wrapped text, or `null` when layout is
not available. The row is relative to the complete textarea value, not its currently scrolled
viewport. Pass that row to `getVisualLineRange(row)` to get its `{start, end}` character offsets;
`end` is exclusive, explicit newline characters are excluded, and both offsets use the same units
as `cursorPosition`.

## Links

Create an inline anchor with a destination and ordinary child nodes:

```ts
const link = pc.createElement("a");
link.href = "https://example.com/docs";
link.style.textDecorationLine = "underline";
link.appendChild(pc.createTextNode("Documentation"));
root.appendChild(link);
```

`AnchorElement.href` is readable and writable. Use absolute URLs: PaintCannon
has no document base URL to resolve relative destinations against.

In supported terminals, the label is a native OSC 8 hyperlink. Otherwise,
PaintCannon renders `[Documentation](https://example.com/docs)`. This fallback
is part of layout, so it wraps, clips, scrolls and copies like other text.
Spaces, Unicode and control characters in destinations are percent-encoded in
the rendered URL.

Support is based on the [OSC 8 adoption
list](https://github.com/Alhadis/OSC8-Adoption/). Unknown terminals use the
fallback, as do terminals whose hyperlinks are disabled by default (such as
Konsole). There is no universal OSC 8 capability query; terminal identity
cannot reveal every user setting or older emulator version. PaintCannon does
not launch a browser process itself. Its ordinary click events still bubble,
but `preventDefault()` cannot cancel a gesture handled by the terminal
emulator, since the gesture is handled by the emulator itself rather than
PaintCannon.

Inside tmux, **every attached client viewing the pane's session** must have a
supported outer terminal and tmux's `hyperlinks` terminal feature enabled.
tmux 3.4 or newer is required. If tmux has not enabled the feature automatically,
add `set -as terminal-features 'xterm-ghostty:hyperlinks'` to your tmux configuration
for Ghostty (use the matching terminal pattern for other emulators). Detach and
reattach existing clients if the feature is not applied immediately.

Opening a link uses your terminal's gesture, commonly Ctrl-click or Cmd-click.
With mouse capture enabled, Shift may also be required to let the terminal
handle the click. With `captureMouse: true`, hovering a native link
automatically shows opening instructions for the detected terminal's default
link-opening bindings.

Shortcuts can be OS-specific; for example, on Linux a terminal emulator may
require Ctrl-Shift-Click and on macOS require Command-Shift-Click. Local
clients detect the host OS and render the OS-specific shortcut. When the client
OS cannot be determined (for example, over SSH), PaintCannon shows generic
hints when necessary; for example, we might render: `Ctrl-Shift-Click to open
(Cmd-Shift-Click on macOS)`.

## Events

PaintCannon supports bubbling events with `stopPropagation()` and `preventDefault()`:

- Click events
- Mouse enter and leave events
- Keyboard events
- Text and image paste events
- Input change events
- Form submit events
- Focus and blur events
- Transition start and end events
- Scroll events
- Resize events
- App-level terminal focus and blur events
- App-level clipboard write notification events

Pasting text, or pasting or dropping one or more image files, dispatches a bubbling `paste` event
targeted at the focused element. Text pastes are available from
`clipboardData.getData("text/plain")` and have an empty `clipboardData.files`. Image pastes have an
empty text value and expose files through `clipboardData.files`, so image paths are not visible to
the application or inserted into a focused text control.

Each image is a `PaintFile` object with `name`, `type`, `size`, `lastModified`, `arrayBuffer()`,
`bytes()`, `text()`, and `stream()`.

By default, text is inserted into the focused input or textarea. Image files are not inserted. Call
`preventDefault()` when taking over paste handling yourself:

```ts
input.addEventListener("paste", async event => {
  if (event.clipboardData.files.length === 0) {
    return;
  }

  event.preventDefault();
  for (const file of event.clipboardData.files) {
    console.log(file.name, file.type, file.size, await file.bytes());
  }
});
```

PaintCannon detects PNG, JPEG, WebP, and GIF files. For a text-only paste,
`clipboardData.files` is empty.

Run `npm run demo:paste-images` from the workspace root to paste or drag PNG files into a live
PaintCannon image renderer.

When `captureMouse` is enabled, releasing a dragged text selection requests a clipboard copy and
dispatches an app-level `clipboardWrite` event on the `PaintCannon` instance. The event does not
bubble and cannot cancel the copy. Its `text` property contains the selected text, and `success`
reports whether PaintCannon successfully handed the copy request to the local clipboard command or
terminal output:

```ts
pc.addEventListener("clipboardWrite", event => {
  if (event.success) {
    showToast("Text copied");
  }
});
```

For OSC 52 terminal copies, `success` confirms that the escape sequence was written and flushed;
it cannot confirm that the terminal accepted the request or changed the system clipboard.
Applications using `paintcannon-react` can subscribe through the `paintCannon` property returned by
`render()`.

## Intersection Observer

PaintCannon exports the standard core `IntersectionObserver` API. Rectangles use terminal cells as
their coordinate unit, and observations are calculated from the committed Rust layout, including
scroll offsets, ancestor overflow clipping, borders, explicit roots, root margins, and threshold
crossings. In `rootMargin`, one `px` corresponds to one terminal cell; percentages resolve against
the root width, as they do in CSS:

```ts
import { IntersectionObserver } from "paintcannon";

const observer = new IntersectionObserver(
  entries => {
    for (const entry of entries) {
      entry.target.style.opacity = entry.isIntersecting ? "1" : "0.4";
    }
  },
  { root: viewport, rootMargin: "1px 0px", threshold: [0, 0.5, 1] },
);

observer.observe(row);
```

The implementation supports `observe()`, `unobserve()`, `disconnect()`, and `takeRecords()`, plus
the standard `root`, `rootMargin`, and `thresholds` properties. As on the web, callbacks are queued
asynchronously and an initial entry is delivered for every observed target.

The Rust renderer owns a fixed-cadence render loop, configured with the `fps`
constructor option (60 by default). It only runs layout for layout-dirty
changes, only paints dirty frames or active transitions, and only writes
terminal bytes when the frame diff is non-empty. `requestAnimationFrame()` and
`cancelAnimationFrame()` schedule JS updates at the configured cadence, while
`renderSync()` is available when shutdown or another explicit barrier must wait
for the current state to reach the terminal.

Terminal focus detection is separate from element focus. PaintCannon enables xterm focus reporting
from Rust, listens for terminal focus gain/loss reports, and exposes them through the
`PaintCannon` instance:

```ts
pc.addEventListener("blur", () => {
  root.style.backgroundColor = "#27272a";
});

pc.addEventListener("focus", () => {
  root.style.backgroundColor = "#0f172a";
});

console.log(pc.hasFocus);
```

Inside tmux, enable focus reporting in tmux first:

```tmux
set -g focus-events on
```

## Nerd Font Detection

Use `pc.font.hasNerdFontSet()` to choose icons or plain-text fallbacks:

```ts
const branchIcon = (await pc.font.hasNerdFontSet("powerline")) ? "\ue0a0" : "branch";
```

The check is asynchronous and conservative: `false` means support could not be
confirmed. Accepted sets are `"powerline"`, `"powerline-extended"`,
`"font-awesome"`, `"devicons"`, and `"material-design"` (also exported as the
`NerdFontSet` type). Check the specific sets your application uses. Detection
samples representative characters for each set independently; an unsupported
set does not disable another set. A stopped instance returns `false`.

Rust refreshes support every two seconds. Listen for changes to switch your
icons or fallbacks without polling from JavaScript:

```ts
import type { PaintFontChangeEvent } from "paintcannon";

const onPowerlineChange = (event: PaintFontChangeEvent) => {
  branchLabel.textContent = event.supported ? "\ue0a0 main" : "branch: main";
};
pc.font.addEventListener("nerdfont-powerline-change", onPowerlineChange);
// Read the initial value as well; listeners only receive changes.
branchLabel.textContent = (await pc.font.hasNerdFontSet("powerline"))
  ? "\ue0a0 main"
  : "branch: main";

// When this UI is removed:
pc.font.removeEventListener("nerdfont-powerline-change", onPowerlineChange);
```

Event names are `nerdfont-${set}-change` for each accepted set, plus
`symbols-for-legacy-computing-change`. Events contain `type`, `supported`,
`target`, and `currentTarget` (both targets are `pc.font`). Repeated unchanged
checks do not emit events. Support starts unconfirmed (`false`), so initial
confirmation can emit a change to `true`. `pc.stop()` removes all listeners
and stops detection. In-flight glyph queries can add their probe timeout to
the refresh latency; concurrent API calls share one probe per refresh.

Detection tries these sources in order:

1. `NERD_FONT=0` forces all sets to `false`; `NERD_FONT=1` forces them to `true`,
   without queries. Other set values (including empty) also enable the override.
   Unset the variable to use automatic detection.
2. Many popular terminal emulators bundle Nerd Font symbols; we attempt to
   detect them when the terminal can be identified. Detected terminals are:
   - Ghostty
   - Wezterm
   - Rio
   - Kitty
   - Alacritty
   - Foot
   - Apple Terminal
   - Iterm2
   - Konsole
   - VTE-based terminals such as Gnome
   - XtermJS
   - Xterm
   - Rxvt
   - VSCode
     Detection checks `TERM_PROGRAM` and falls back to emulator-specific `TERM`
     values such as `xterm-ghostty` and `xterm-kitty`. [OpenSSH forwards `TERM`
     when allocating a PTY](https://man.openbsd.org/ssh_config#SendEnv), but does
     not automatically forward `TERM_PROGRAM`. Generic values such as
     `xterm-256color` cannot identify an emulator. Inside tmux, detection reads
     **live attached clients**, not the global/session environment saved when
     the server or pane started. It checks each client's `TERM` and terminal
     version-report identity, plus `TERM_PROGRAM` and fallback markers when
     tmux supports per-client environment queries (added after tmux 3.7).
3. The [Rio Glyph
   Protocol](https://github.com/raphamorim/rio/blob/main/specs/glyph-protocol.md),
   only for sets not confirmed by the terminal table and only when the terminal
   is known to support APC framing (`supports_apc`). This works over SSH; tmux
   requires `set -g allow-passthrough on` and a single attached client.

With tmux, **every attached terminal client whose session contains the pane**
must support a capability. Attaching a less capable client downgrades it;
detaching that client can restore it. Control-mode clients are excluded.
Glyph replies have no client identity, so one client's positive response
cannot prove support for multiple clients. Unidentified clients therefore
fail closed in that case. With no clients, automatic detection also returns
`false`. The explicit `NERD_FONT` override still takes precedence.

Reattaching does not update the pane process's inherited environment or reliably
update tmux's global environment. Live client detection avoids those stale
values. On older tmux versions, generic `TERM` plus an absent/unrecognized
terminal version report may remain unidentified. See [tmux formats and environment
handling](https://man.openbsd.org/tmux).

PaintCannon does not inspect installed fonts or parse terminal configuration.
Unknown support returns `false`; users can set `NERD_FONT=1` when they have
configured a Nerd Font in a terminal without built-in symbols or protocol support.

To verify tmux transport manually, run `bash scripts/test-font-probe-tmux.sh`
from the repository root. It uses an isolated tmux server and a simulated
terminal, cleans up on exit, and requires Python 3. It is not part of CI and
does not test your actual terminal's glyph coverage.

After building, `bash scripts/test-font-changes-tmux.sh` tests live attach,
detach, multiple clients, JavaScript change events, and actual border cells
using simulated terminal identities on an isolated tmux server. This is also
opt-in, requires Python 3, and cleans up its clients/server on exit.

`chunky` uses Powerline Extended triangles when that set is detected.
`chunky-rounded` uses Symbols for Legacy Computing corners
(U+1FB41, U+1FB4C, U+1FB52, U+1FB5D) when supported. Both fall back to filled
square corners and retain filled edges. Detection changes only repaint;
it does not change layout. `npm run demo:borders` shows both styles.

`chunky-best-effort` chooses rounded legacy corners first, Powerline Extended
triangles second, and filled square corners last. It uses the same one-cell
border layout in all three cases. The borders demo also shows this style.

`pill` adds only a one-cell left and right border, so a one-line label stays one
row tall. It uses the Powerline Extended left/right semicircles (U+E0B6/U+E0B4),
or filled square cells when unsupported. For a solid pill, set `borderColor`
to match `backgroundColor`:

```ts
button.style.border = "pill";
button.style.borderColor = "#38bdf8";
button.style.backgroundColor = "#38bdf8";
button.style.color = "black";
```

`borderLeft` and `borderRight` can also use `pill` independently. `pill` on
`borderTop` or `borderBottom` reserves no space and draws no border. Wrapped
content keeps the same side caps on each row.

### Symbols for Legacy Computing

```ts
const supported = await pc.font.hasSymbolsForLegacyComputing();
```

This `Promise<boolean>` reads the current detection pass, checking known terminal built-in support first,
including the smooth mosaic corners used by `chunky-rounded`. It is independent
of `NERD_FONT`: Nerd Fonts do not guarantee those characters. Windows Terminal
1.25.2733.0 and newer includes these corners in its
[built-in glyph renderer](https://github.com/microsoft/terminal/discussions/20758);
both its terminal identity and `WT_SESSION` are recognized. If the table does
not recognize the terminal, environment markers such as `VTE_VERSION` can
establish legacy glyph support without identifying a particular application.
Recognized terminals use their table entry; these fallback markers do not
override it. If support remains unconfirmed, the Glyph Protocol probes a
sextant and all four border corners. Unconfirmed support and stopped instances
return `false`.

The [Rust capability table](src/glyph_detection/terminals.rs) links to the
upstream implementations. As with Nerd Font detection, tmux's outer
`TERM_PROGRAM` and `TERM` are checked; no local font or terminal configuration is inspected.

## Usage

```ts
import { PaintCannon } from "paintcannon";

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 60,
});

const root = pc.createElement("div");
pc.setRoot(root);

root.style.display = "flex";
root.style.width = "100%";
root.style.height = "100%";
root.style.alignItems = "center";
root.style.justifyContent = "center";
root.style.backgroundColor = "#020617";
root.style.color = "#e2e8f0";

const button = pc.createElement("button");
button.style.border = "chunky-rounded";
button.style.borderColor = "#fb923c";
button.style.backgroundColor = "#0f172a";
button.style.padding = "1 2";
button.style.cursor = "pointer";

const label = pc.createTextNode("Click me");
button.appendChild(label);

let count = 0;
button.addEventListener("click", () => {
  count += 1;
  label.nodeValue = `Clicked ${count} times`;
});

root.appendChild(button);
```

## React

For React rendering on top of this DOM API, use `paintcannon-react`.
