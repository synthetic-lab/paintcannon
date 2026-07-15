# paintcannon-react

A React reconciler for `paintcannon`, built for fast terminal UIs.

`paintcannon-react` lets you render React components into PaintCannon's
Rust-backed terminal renderer. It is intended as a faster, no-flicker
alternative for terminal React interfaces that would otherwise use Ink.

![A cannon shooting paint](https://raw.githubusercontent.com/synthetic-lab/paintcannon/refs/heads/main/paintcannon-shot.png)

## Installation

`paintcannon` and `react` are peer dependencies, so install them alongside the
reconciler:

```bash
npm install --save paintcannon paintcannon-react react
```

## Usage

```tsx
import React, { useState } from "react";
import { Button, Div, Span, render } from "paintcannon-react";

function Counter(): React.ReactElement {
  const [count, setCount] = useState(0);

  return (
    <Div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 1,
        backgroundColor: "#020617",
        color: "#e2e8f0",
      }}
    >
      <Span style={{ color: "#38bdf8" }}>paintcannon-react</Span>
      <Button
        style={{
          border: "chunky-rounded",
          borderColor: "#fb923c",
          backgroundColor: count % 2 === 0 ? "#0f172a" : "#7c2d12",
          color: "#f8fafc",
          padding: "1 2",
          cursor: "pointer",
        }}
        onClick={() => {
          setCount(value => value + 1);
        }}
      >
        Clicked {count} times
      </Button>
    </Div>
  );
}

const app = render(<Counter />, {
  alternateScreen: true,
  captureMouse: true,
});

await app.waitUntilExit();
```

## API differences from Ink

PaintCannon-React isn't 100% drop-in compatible with Ink: the primary
difference is that PaintCannon expects you to use PaintCannon's native
`<input>` and `<textarea>` components for input, rather than relying on custom
JS reimplementations of input handling on top of React. Although this is
_different_ thank Ink, this comes with a significant performance win: typing in
PaintCannon is much lower latency as a result.

PaintCannon is also somewhat less restrictive thank Ink: you aren't required to
wrap text content in `<Text>` nodes. Text content Just Works inside PaintCannon
components, just like with a regular browser.

## Host Components

PaintCannon exposes the following host components that mirror a subset of the
DOM API:

- `Div`
- `Span`
- `Input`
- `Textarea`
- `Button`
- `Form`

These components map to PaintCannon's DOM-like core API and support typed style
props, bubbling events, forms, focus handling, controlled inputs, and mouse
interactions. The subset of the React-DOM props they support is as follows:

Host component `onFocus` and `onBlur` are element focus events. Terminal
window/pane focus is exposed by the underlying PaintCannon instance as
`paintCannon.hasFocus` and app-level `focus`/`blur` events. The root returned
from `render()` includes that instance:

```tsx
const app = render(<App />, { alternateScreen: true });

app.paintCannon.addEventListener("blur", () => {
  // Dim or pause the app while the terminal is unfocused.
});
```

Inside tmux, terminal focus reporting requires `set -g focus-events on`.

### Common props:

All host components accept:

- `children?: React.ReactNode`
- `style?: CSSStyleProperties`
- `ref?: React.Ref<Element>`

All host components accept these event props:

- `onKeyDown`
- `onKeyUp`
- `onPaste`
- `onClick`
- `onMouseEnter`
- `onMouseLeave`
- `onMouseMove`
- `onFocus`
- `onBlur`
- `onSubmit`
- `onChange`
- `onTransitionStart`
- `onTransitionEnd`
- `onScroll`

`onPaste` receives the same `PaintClipboardEvent` as core, including detected terminal image paths
through `event.clipboardData.files`.

`style` accepts the following CSS property names. Kebab-case and camelCase are
both supported:

- `display`
- `position`
- `top`
- `right`
- `bottom`
- `left`
- `z-index` / `zIndex`
- `visibility` (accepts `"visible"` or `"hidden"`; hidden elements keep their
  layout space but do not paint or receive hit tests)
- `opacity` (accepts a number or percentage and applies to the component subtree as a group)
- `overflow`
- `overflow-x` / `overflowX`
- `overflow-y` / `overflowY`
- `scrollbar-color` / `scrollbarColor`
- `scrollbar-gutter` / `scrollbarGutter`
- `image-rendering` / `imageRendering`
- `flex-direction` / `flexDirection`
- `flex-wrap` / `flexWrap`
- `flex-flow` / `flexFlow`
- `flex-basis` / `flexBasis`
- `flex-grow` / `flexGrow`
- `flex-shrink` / `flexShrink`
- `flex`
- `justify-content` / `justifyContent`
- `align-items` / `alignItems`
- `align-self` / `alignSelf`
- `align-content` / `alignContent`
- `justify-items` / `justifyItems`
- `justify-self` / `justifySelf`
- `gap`
- `row-gap` / `rowGap`
- `column-gap` / `columnGap`
- `padding`
- `padding-top` / `paddingTop`
- `padding-right` / `paddingRight`
- `padding-bottom` / `paddingBottom`
- `padding-left` / `paddingLeft`
- `margin`
- `margin-top` / `marginTop`
- `margin-right` / `marginRight`
- `margin-bottom` / `marginBottom`
- `margin-left` / `marginLeft`
- `width`
- `height`
- `min-width` / `minWidth`
- `max-width` / `maxWidth`
- `min-height` / `minHeight`
- `max-height` / `maxHeight`
- `white-space` / `whiteSpace`
- `border`
- `border-top` / `borderTop`
- `border-right` / `borderRight`
- `border-bottom` / `borderBottom`
- `border-left` / `borderLeft`
- `border-color` / `borderColor`
- `color`
- `placeholder-color` / `placeholderColor`
- `transition`
- `background`
- `background-color` / `backgroundColor`
- `selection-background-color` / `selectionBackgroundColor`
- `cursor`
- `grid-template-columns` / `gridTemplateColumns`
- `grid-template-rows` / `gridTemplateRows`
- `grid-auto-columns` / `gridAutoColumns`
- `grid-auto-rows` / `gridAutoRows`
- `grid-auto-flow` / `gridAutoFlow`
- `grid-column` / `gridColumn`
- `grid-row` / `gridRow`
- `grid-column-start` / `gridColumnStart`
- `grid-column-end` / `gridColumnEnd`
- `grid-row-start` / `gridRowStart`
- `grid-row-end` / `gridRowEnd`
- `font-style`/`fontStyle` (accepts `"italic"` or `"normal"`)
- `font-weight`/`fontWeight` (accepts `"bold"` or `"normal"`)
- `text-decoration`/`textDecoration` (accepts `"underline"`, `"line-through"`, or `"none"`)

### Per-component props:

- `Div`: shared props, plus `scrollLeft?: number` and `scrollTop?: number`.
- `Span`: shared props, plus `scrollLeft?: number` and `scrollTop?: number`.
- `Form`: shared props, plus `scrollLeft?: number` and `scrollTop?: number`.
- `Button`: shared props, plus `type?: "submit" | "button"`,
  `scrollLeft?: number`, and `scrollTop?: number`.
- `Input`: shared props, plus `type?: "text"`, `value?: string`,
  `placeholder?: string`, `cursorPosition?: number`, and
  `autoFocus?: boolean`.
- `Textarea`: shared props, plus `value?: string`, `placeholder?: string`,
  `cursorPosition?: number`, `autoFocus?: boolean`, `scrollLeft?: number`, and
  `scrollTop?: number`. `Textarea` does not accept a `type` prop.

## Hooks

`paintcannon-react` includes:

- `useApp()` for exiting the rendered app from inside React.
- `useAnimation()` for requestAnimationFrame-driven animations that share a
  single render cycle.

## Core Runtime

This package depends on `paintcannon`, which provides the NAPI-RS native
renderer and DOM-like API. React commits are sent as PaintCannon transactions;
the Rust-owned render loop presents dirty state at the configured `fps`.
