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
- Margins and padding, including auto margins
- Overflow hidden and scroll, with native mouse scrolling
- `visibility: hidden`, which preserves layout space while suppressing paint and hit testing
- 24-bit RGB background, border, text, placeholder, and selection coloring with 256-color and
  16-color fallbacks
- CSS transitions for color properties
- Mouse pointer styling in supported terminals

PaintCannon also exposes terminal-specific border styles:

- `none`
- `solid`
- `double`
- `heavy`
- `rounded`
- `chunky-rounded`
- `ascii`

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

## Events

PaintCannon supports bubbling events with `stopPropagation()` and `preventDefault()`:

- Click events
- Mouse enter and leave events
- Keyboard events
- Input change events
- Form submit events
- Focus and blur events
- Transition start and end events
- Scroll events
- Resize events

It also exposes `requestAnimationFrame()` and `cancelAnimationFrame()` so UI code can synchronize
with PaintCannon's render loop.

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
