import React from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mock } from "antipattern";
import {
  PaintCannon,
  PaintKeyboardEvent,
  paintCannonDeps,
  type InputElement,
  type PaintElement,
  type PaintResizeEvent,
  type TextAreaElement,
} from "paintcannon";
import { Div, Input, Textarea, render } from "../src/index.ts";
import {
  createMockNativeBinding,
  keyboardInput,
  mouseEvent,
  pasteInput,
  resizeEvent,
  type MockNativePaintCannon,
} from "../../paintcannon/test/mock-native.ts";

let restores: Array<() => void> = [];
let mockNativeInstances: MockNativePaintCannon[] = [];

beforeEach(() => {
  mockNativeInstances = [];
  restores = [
    mock(paintCannonDeps, "loadNativeBinding", () => createMockNativeBinding(mockNativeInstances)),
  ];
});

afterEach(() => {
  for (const restore of restores.reverse()) {
    restore();
  }
  restores = [];
});

describe("keyboard events", () => {
  it("targets root content when no text control is focused", async () => {
    const events: string[] = [];
    const root = render(<Div onKeyDown={event => events.push(event.key)}>root</Div>, { fps: 120 });

    await commit();
    dispatchKey(root.paintCannon, "z");
    root.paintCannon.stop();

    expect(events).toEqual(["z"]);
  });

  it("targets the focused text control and does not notify sibling controls", async () => {
    const events: string[] = [];
    let input: InputElement | undefined;
    let textarea: TextAreaElement | undefined;

    const root = render(
      <Div>
        <Input
          ref={value => {
            input = value;
          }}
          onKeyDown={event => {
            events.push(`input:${event.key}`);
            event.preventDefault();
          }}
        />
        <Textarea
          ref={value => {
            textarea = value;
          }}
          onKeyDown={event => {
            events.push(`textarea:${event.key}`);
            event.preventDefault();
          }}
        />
      </Div>,
      { fps: 120 },
    );

    await commit();
    expect(input).toBeDefined();
    expect(textarea).toBeDefined();

    input?.focus();
    dispatchKey(root.paintCannon, "a");
    textarea?.focus();
    dispatchKey(root.paintCannon, "b");
    root.paintCannon.stop();

    expect(events).toEqual(["input:a", "textarea:b"]);
  });

  it("does not re-apply autoFocus on controlled updates", async () => {
    const events: string[] = [];
    let textarea: TextAreaElement | undefined;
    let update = (): void => {};

    function App(): React.ReactElement {
      const [value, setValue] = React.useState("");
      update = () => {
        setValue(current => `${current}x`);
      };

      return (
        <Div>
          <Input autoFocus value={value} onKeyDown={event => events.push(`input:${event.key}`)} />
          <Textarea
            ref={value => {
              textarea = value;
            }}
            onKeyDown={event => events.push(`textarea:${event.key}`)}
          />
        </Div>
      );
    }

    const root = render(<App />, { fps: 120 });

    await commit();
    textarea?.focus();
    update();
    await commit();
    dispatchKey(root.paintCannon, "a");
    root.paintCannon.stop();

    expect(events).toEqual(["textarea:a"]);
  });

  it("can leave edit mode on blur and defer focus to another input", async () => {
    const events: string[] = [];
    let mainInput: InputElement | undefined;
    let editInput: InputElement | undefined;

    function App(): React.ReactElement {
      const [editing, setEditing] = React.useState(true);

      return (
        <Div>
          <Input
            ref={value => {
              mainInput = value;
            }}
            onKeyDown={event => events.push(`main:${event.key}`)}
          />
          {editing ? (
            <Input
              ref={value => {
                editInput = value;
              }}
              autoFocus
              onBlur={() => {
                setEditing(false);
                queueMicrotask(() => mainInput?.focus());
              }}
              onKeyDown={event => events.push(`edit:${event.key}`)}
            />
          ) : null}
        </Div>
      );
    }

    const root = render(<App />, { fps: 120 });

    await commit();
    expect(mainInput).toBeDefined();
    expect(editInput).toBeDefined();

    editInput?.focus();
    mainInput?.focus();
    await Promise.resolve();
    await commit();
    dispatchKey(root.paintCannon, "a");
    root.paintCannon.stop();

    expect(events).toEqual(["main:a"]);
  });
});

describe("paste events", () => {
  it("forwards clipboard events with typed data to host components", async () => {
    const events: string[] = [];
    let input: InputElement | undefined;
    const root = render(
      <Input
        ref={element => {
          input = element;
        }}
        onPaste={event => events.push(event.clipboardData.getData("text/plain"))}
      />,
      { fps: 120 },
    );

    await commit();
    input?.focus();
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }
    mockNative.events.push(pasteInput("from clipboard"));
    notifyNativeEvents(root.paintCannon);
    root.paintCannon.stop();

    expect(events).toEqual(["from clipboard"]);
    expect(input?.value).toBe("from clipboard");
  });
});

describe("controlled text controls", () => {
  it("supports React-style value plus onChange without manually controlling cursorPosition", async () => {
    const changes: string[] = [];
    let input: InputElement | undefined;

    function App(): React.ReactElement {
      const [value, setValue] = React.useState("");
      return (
        <Input
          ref={element => {
            input = element;
          }}
          autoFocus
          value={value}
          onChange={event => {
            changes.push(event.target.value);
            setValue(event.target.value);
          }}
        />
      );
    }

    const root = render(<App />, { fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    await commit();
    mockNative.events.push(
      keyboardInput({
        type: "keydown",
        key: "a",
        code: "KeyA",
        ctrlKey: false,
        altKey: false,
        metaKey: false,
        shiftKey: false,
        repeat: false,
      }),
    );
    notifyNativeEvents(root.paintCannon);
    await commit();
    root.paintCannon.stop();

    expect(changes).toEqual(["a"]);
    expect(input?.value).toBe("a");
    expect(input?.cursorPosition).toBe(1);
  });
});

describe("resize events", () => {
  it("does not synchronously render during native event delivery", () => {
    const sizes: Array<[number, number]> = [];
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    paintCannon.addEventListener("resize", (event: PaintResizeEvent) => {
      sizes.push([event.cols, event.rows]);
    });
    mockNative.events.push(resizeEvent(100, 40));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(sizes).toEqual([[100, 40]]);
    expect(mockNative.renderSyncCalls).toBe(0);
  });
});

describe("app exit", () => {
  it("flushes the final frame before unmounting", async () => {
    const root = render(<Div>persistent output</Div>, { fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    await commit();
    root.exit();
    await root.waitUntilExit();

    expect(mockNative.renderSyncCalls).toBe(1);
    expect(mockNative.stopCalls).toBe(1);
  });
});

describe("host tree lifecycle", () => {
  it("destroys a large set of removed siblings in one React commit", async () => {
    const view = (count: number): React.ReactElement => (
      <Div>
        {Array.from({ length: count }, (_, index) => (
          <Div key={index}>row {index}</Div>
        ))}
      </Div>
    );
    const root = render(view(200), { fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    await commit();
    const destroysBeforeUpdate = mockNative.destroyedNodes.length;
    root.render(view(1));
    await commit();
    root.paintCannon.stop();

    expect(mockNative.destroyedNodes.length - destroysBeforeUpdate).toBe(199);
  });
});

describe("scroll events", () => {
  it("commits state updates from onScroll before native event delivery returns", async () => {
    let scroller: PaintElement | undefined;
    let renderedTop = -1;

    function App(): React.ReactElement {
      const [scrollTop, setScrollTop] = React.useState(2);
      renderedTop = scrollTop;
      return (
        <Div
          ref={element => {
            scroller = element;
          }}
          style={{ overflowY: "scroll" }}
          onScroll={event => {
            setScrollTop(event.scrollTop);
          }}
        />
      );
    }

    const root = render(<App />, { captureMouse: true, fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    await commit();
    expect(scroller).toBeDefined();
    mockNative.targetIdAtPoint = scroller?.id ?? null;
    mockNative.scrollMetricsById.set(scroller?.id ?? 0, {
      scrollLeft: 0,
      scrollTop: 2,
      scrollWidth: 10,
      scrollHeight: 10,
      clientWidth: 10,
      clientHeight: 4,
    });
    mockNative.events.push(mouseEvent("wheel", { deltaY: -1 }));

    notifyNativeEvents(root.paintCannon);
    root.paintCannon.stop();

    expect(renderedTop).toBe(0);
  });
});

describe("style props", () => {
  it("clears a style property when the next React style value is undefined", async () => {
    let row: PaintElement | undefined;
    const view = (backgroundColor: string | undefined): React.ReactElement => (
      <Div
        ref={element => {
          row = element;
        }}
        style={{ backgroundColor }}
      >
        row
      </Div>
    );
    const root = render(view("#1e3a5f"), { fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error("expected mock native instance");
    }

    await commit();
    expect(row).toBeDefined();
    expect(mockNative.styleMutations).toContainEqual({
      id: row?.id,
      property: "background-color",
      value: "#1e3a5f",
    });

    root.render(view(undefined));
    await commit();
    await commit();
    root.paintCannon.stop();

    expect(mockNative.styleMutations).toContainEqual({
      id: row?.id,
      property: "background-color",
      value: "",
    });
  });
});

function dispatchKey(paintCannon: PaintCannon, key: string): void {
  const anyPaintCannon = paintCannon as unknown as {
    keyboardEventTarget(): PaintElement | undefined;
    dispatchKeyboardEvent(target: PaintElement, event: PaintKeyboardEvent): void;
  };
  const target = anyPaintCannon.keyboardEventTarget();
  if (target === undefined) {
    throw new Error("expected keyboard event target");
  }
  anyPaintCannon.dispatchKeyboardEvent(
    target,
    new PaintKeyboardEvent(
      {
        type: "keydown",
        key,
        code: `Key${key.toUpperCase()}`,
        ctrlKey: false,
        altKey: false,
        metaKey: false,
        shiftKey: false,
        repeat: false,
      },
      target,
    ),
  );
}

function notifyNativeEvents(_paintCannon: PaintCannon): void {
  const mockNative = mockNativeInstances[mockNativeInstances.length - 1];
  if (mockNative === undefined) {
    throw new Error("expected mock native instance");
  }
  mockNative.notifyEvents();
}

async function commit(): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, 20));
}
