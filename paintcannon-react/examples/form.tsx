import React, { useState } from "react";
import { Button, Div, Form, Input, Span, Textarea, render, useApp } from "../src/index.ts";

function FormDemo(): React.ReactElement {
  const { exit } = useApp();
  const [submits, setSubmits] = useState(0);
  const [lastSubmitter, setLastSubmitter] = useState("none");

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
      onKeyDown={event => {
        if (event.key === "Escape" || (event.ctrlKey && event.code === "KeyC")) {
          event.preventDefault();
          exit();
        }
      }}
    >
      <Span style={{ color: "#38bdf8" }}>paintcannon-react forms</Span>
      <Form
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 1,
          width: 54,
          padding: "1 2",
          border: "rounded",
          borderColor: "#334155",
          backgroundColor: "#0f172a",
        }}
        onSubmit={event => {
          event.preventDefault();
          setSubmits(value => value + 1);
          setLastSubmitter(`node ${event.submitter.id}`);
        }}
      >
        <Input
          autoFocus
          placeholder="Enter submits the form"
          style={{
            height: 3,
            border: "rounded",
            borderColor: "#475569",
            backgroundColor: "#020617",
            color: "#f8fafc",
            placeholderColor: "#64748b",
          }}
        />
        <Textarea
          placeholder="Enter inserts a newline here"
          style={{
            minHeight: 4,
            border: "rounded",
            borderColor: "#475569",
            backgroundColor: "#020617",
            color: "#f8fafc",
            placeholderColor: "#64748b",
          }}
        />
        <Button
          type="submit"
          style={{
            alignSelf: "flex-start",
            padding: "0 2",
            border: "chunky-rounded",
            borderColor: "#fb923c",
            backgroundColor: "#7c2d12",
            color: "#f8fafc",
            cursor: "pointer",
          }}
        >
          Submit
        </Button>
      </Form>
      <Div style={{ color: "#cbd5e1" }}>
        submits={submits} submitter={lastSubmitter}
      </Div>
      <Div style={{ color: "#64748b" }}>Tab changes focus. Escape or Ctrl-C exits.</Div>
    </Div>
  );
}

const root = render(<FormDemo />, {
  alternateScreen: true,
  captureCtrlC: true,
  captureMouse: true,
});

root.waitUntilExit().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
