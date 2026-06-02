# paintcannon-react

A React reconciler for `paintcannon`, built for fast terminal UIs.

`paintcannon-react` lets you render React components into PaintCannon's Rust-backed terminal
renderer. It is intended as a faster, no-flicker alternative for terminal React interfaces
that would otherwise use Ink.

![A cannon shooting paint](https://raw.githubusercontent.com/synthetic-lab/paintcannon/refs/heads/main/paintcannon-shot.png)

## Host Components

PaintCannon host components use capitalized names to make it clear that they are not browser DOM
elements:

- `Div`
- `Span`
- `Input`
- `Textarea`
- `Button`
- `Form`

These components map to PaintCannon's DOM-like core API and support typed style props, bubbling
events, forms, focus handling, controlled inputs, and mouse interactions.

## Installation

`paintcannon` and `react` are peer dependencies, so install them alongside the reconciler:

```bash
npm install paintcannon paintcannon-react react
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

## Hooks

`paintcannon-react` includes:

- `useApp()` for exiting the rendered app from inside React.
- `useAnimation()` for requestAnimationFrame-driven animations that share a single render cycle.

## Core Runtime

This package depends on `paintcannon`, which provides the NAPI-RS native renderer and DOM-like API.
