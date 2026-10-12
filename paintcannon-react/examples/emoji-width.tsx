import { useState } from "react";
import { Div, Span, Textarea, render, useApp } from "../src/index.ts";

const alternateScreen = !process.argv.includes("--primary-screen");

function EmojiWidthDemo() {
  const { exit } = useApp();
  const [value, setValue] = useState("⚠🐙❤️👩‍💻👍🏽");
  const [message, setMessage] = useState("🐙🐙🐙🐙🐙🐙");

  return (
    <Div
      style={{
        display: "flex",
        flexDirection: "column",
        whiteSpace: "pre-wrap",
        width: "100%",
        height: "100%",
        padding: 1,
        backgroundColor: "#0f172a",
        color: "#e2e8f0",
      }}
      onKeyDown={event => {
        if (event.key === "Escape") exit();
      }}
    >
      <Span>
        {alternateScreen ? "Alternate screen" : "Normal screen"}. Paste emojis. Enter submits.
        Escape exits.
      </Span>
      <Div
        style={{
          display: "flex",
          flexDirection: "column",
          flexGrow: 1,
          flexShrink: 1,
          flexBasis: 0,
          minWidth: 0,
          minHeight: 0,
          overflowY: "scroll",
          scrollbarGutter: "stable",
          overflowWrap: "anywhere",
        }}
      >
        <Div style={{ display: "flex", marginTop: 1, marginBottom: 1 }}>
          <Div style={{ display: "flex", marginRight: 1, flexShrink: 0 }}>
            <Span>▶</Span>
          </Div>
          <Div style={{ display: "flex", flexDirection: "column" }}>
            {message.split("\n").map((line, index) => (
              <Div key={index} style={{ display: "flex" }}>
                <Span>{line}</Span>
              </Div>
            ))}
          </Div>
        </Div>
        <Div style={{ display: "flex", border: "rounded", borderColor: "gray" }}>
          <Span>{value}</Span>
        </Div>
        <Span style={{ color: "gray" }}>
          {Array.from(
            value,
            character => `U+${character.codePointAt(0)?.toString(16).toUpperCase()}`,
          ).join(" ")}
        </Span>
      </Div>
      <Div
        style={{
          display: "flex",
          width: "100%",
          minWidth: 0,
          paddingLeft: 1,
          paddingRight: 1,
          border: "rounded",
          borderColor: "green",
        }}
      >
        <Div
          style={{
            display: "flex",
            flexDirection: "column",
            flexGrow: 1,
            flexShrink: 1,
            flexBasis: 0,
            minWidth: 0,
          }}
        >
          <Textarea
            autoFocus
            value={value}
            onChange={event => setValue(event.target.value)}
            onKeyDown={event => {
              if (event.key === "Enter") {
                event.preventDefault();
                setMessage(value);
                setValue("");
              }
            }}
            style={{
              display: "flex",
              width: "100%",
              minWidth: 0,
              minHeight: 1,
              flexGrow: 1,
              whiteSpace: "pre-wrap",
              overflowY: "visible",
            }}
          />
        </Div>
      </Div>
    </Div>
  );
}

const root = render(<EmojiWidthDemo />, {
  alternateScreen,
  captureMouse: true,
  captureCtrlC: false,
});

await root.waitUntilExit();
