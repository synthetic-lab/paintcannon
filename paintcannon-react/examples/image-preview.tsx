import React from "react";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Div, Img, Span, render, useApp } from "../src/index.ts";

function ImagePreviewDemo({ source }: { source: string }): React.ReactElement {
  const { exit } = useApp();

  return (
    <Div
      autoFocus
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexWrap: "wrap",
        alignContent: "flex-start",
        gap: 2,
        padding: 1,
        color: "#e5e7eb",
        backgroundColor: "#28272f",
      }}
      onKeyDown={event => {
        if (event.key === "Escape" || (event.ctrlKey && event.key === "c")) {
          event.preventDefault();
          exit();
        }
      }}
    >
      <Div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Span>Automatic</Span>
        <Img src={source} style={{ width: 24, height: 10, imageRendering: "auto" }} />
      </Div>
      <Div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Span>Half-block</Span>
        <Img src={source} style={{ width: 24, height: 10, imageRendering: "half-block" }} />
      </Div>
      <Div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Span>ASCII</Span>
        <Img src={source} style={{ width: 24, height: 10, imageRendering: "ascii" }} />
      </Div>
    </Div>
  );
}

const filename =
  process.argv[2] ?? fileURLToPath(new URL("../../../paintcannon-shot.png", import.meta.url));
const bytes = await readFile(filename);
const source = `data:image/png;base64,${bytes.toString("base64")}`;
await render(<ImagePreviewDemo source={source} />, {
  alternateScreen: true,
  captureMouse: true,
}).waitUntilExit();
