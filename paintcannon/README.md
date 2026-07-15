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
- Overflow hidden and scroll, with native mouse scrolling
- `visibility: hidden`, which preserves layout space while suppressing paint and hit testing
- CSS `opacity`, composited once for an element and its descendants as a stacking-context group
- 24-bit RGB and CSS named background, border, text, placeholder, and selection colors with
  256-color and 16-color fallbacks
- CSS transitions for color properties
- Mouse pointer styling in supported terminals
- Terminal focus detection via `PaintCannon.hasFocus` and app-level `focus`/`blur` events

PaintCannon also exposes terminal-specific border styles:

- `none`
- `solid`
- `double`
- `heavy`
- `rounded`
- `chunky-rounded`
- `ascii`

Positioned elements support `position: static | relative | absolute`, the `top`, `right`,
`bottom`, and `left` inset properties, and integer or `auto` `z-index` values. Absolute elements
use the nearest non-static ancestor as their containing block and do not contribute to normal flow.
Positioned inline elements retain their inline flow position while their painted fragments move.

Opacity accepts numbers or percentages and is clamped to the CSS range from `0` to `1`. PaintCannon
queries supporting terminals for their default foreground and background colors so translucent
content can blend correctly when a color resolves to the terminal default. Run
`npm run demo:opacity` to see group opacity over overlapping text and backgrounds.

## Elements

The core DOM subset supports:

- `div`
- `span`
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

It also exposes `requestAnimationFrame()` and `cancelAnimationFrame()` so UI code can synchronize
with PaintCannon's render loop.

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
