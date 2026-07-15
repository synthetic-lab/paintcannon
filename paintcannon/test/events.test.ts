import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mock } from "antipattern";
import { PaintCannon, paintCannonDeps, type PaintElement } from "../main.ts";
import {
  createMockNativeBinding,
  keyboardInput,
  keyDown,
  mouseEvent,
  pasteInput,
  resizeEvent,
  type MockNativePaintCannon,
} from "./mock-native.ts";

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

describe("core keyboard events", () => {
  it("drains events only when native code notifies JavaScript", () => {
    vi.useFakeTimers();
    const paintCannon = new PaintCannon({ fps: 60 });
    const mockNative = currentMockNative();
    const keys: string[] = [];

    try {
      paintCannon.addEventListener("keydown", event => keys.push(event.key));
      mockNative.events.push(keyboardInput(keyDown("a")));

      vi.advanceTimersByTime(1_000);
      expect(keys).toEqual([]);
      expect(vi.getTimerCount()).toBe(0);

      mockNative.notifyEvents();
      expect(keys).toEqual(["a"]);
      expect(mockNative.eventNotifications).toBe(1);
    } finally {
      paintCannon.stop();
      vi.useRealTimers();
    }
  });

  it("targets the first root child and bubbles through ancestors before document listeners", () => {
    const { paintCannon, mockNative, root, child } = createPaintTree();
    const events: string[] = [];

    child.addEventListener("keydown", event => {
      events.push(`child:${event.key}:${event.target === child}:${event.currentTarget === child}`);
    });
    root.addEventListener("keydown", event => {
      events.push(`root:${event.key}:${event.target === child}:${event.currentTarget === root}`);
    });
    paintCannon.addEventListener("keydown", event => {
      events.push(`document:${event.key}:${event.target === child}`);
    });

    mockNative.events.push(keyboardInput(keyDown("a")));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["child:a:true:true", "root:a:true:true", "document:a:true"]);
  });

  it("respects stopPropagation for element and document keyboard listeners", () => {
    const { paintCannon, mockNative, root, child } = createPaintTree();
    const events: string[] = [];

    child.addEventListener("keydown", event => {
      events.push("child");
      event.stopPropagation();
    });
    root.addEventListener("keydown", () => {
      events.push("root");
    });
    paintCannon.addEventListener("keydown", () => {
      events.push("document");
    });

    mockNative.events.push(keyboardInput(keyDown("a")));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["child"]);
  });

  it("routes key events to the focused input and lets preventDefault block text insertion", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("input");
    const events: string[] = [];
    root.appendChild(input);

    input.addEventListener("keydown", event => {
      events.push(`input:${event.key}`);
      event.preventDefault();
    });
    root.addEventListener("keydown", () => {
      events.push("root");
    });
    input.focus();
    mockNative.events.push(keyboardInput(keyDown("x")));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["input:x", "root"]);
    expect(input.value).toBe("");
    expect(mockNative.textControls.get(input.id)?.value).toBe("");
  });

  it("applies default text input when keydown is not prevented", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("input");
    const events: string[] = [];
    root.appendChild(input);
    root.addEventListener("change", event => {
      events.push(
        `root:${event.target === input}:${event.currentTarget === root}:${event.target.value}`,
      );
    });

    input.focus();
    mockNative.events.push(keyboardInput(keyDown("x")));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["root:true:true:x"]);
    expect(input.value).toBe("x");
    expect(input.cursorPosition).toBe(1);
    expect(mockNative.textControls.get(input.id)).toMatchObject({
      value: "x",
      cursor: 1,
    });
  });

  it("reapplying the current value preserves the cursor", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("input");
    root.appendChild(input);

    input.focus();
    mockNative.events.push(keyboardInput(keyDown("a")), keyboardInput(keyDown("b")));
    notifyNativeEvents(paintCannon);
    input.cursorPosition = 1;
    input.value = input.value;
    paintCannon.stop();

    expect(input.value).toBe("ab");
    expect(input.cursorPosition).toBe(1);
    expect(mockNative.textControls.get(input.id)).toMatchObject({
      value: "ab",
      cursor: 1,
    });
  });

  it("moves the cursor to the end when setting a value from the previous end", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("input");
    root.appendChild(input);

    input.value = "prefilled";
    paintCannon.stop();

    expect(input.cursorPosition).toBe(9);
    expect(mockNative.textControls.get(input.id)).toMatchObject({
      value: "prefilled",
      cursor: 9,
    });
  });

  it("cycles text control focus with tab and shift-tab", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const first = paintCannon.createElement("input");
    const second = paintCannon.createElement("textarea");
    const events: string[] = [];
    root.appendChild(first);
    root.appendChild(second);

    first.addEventListener("focus", () => events.push("first:focus"));
    first.addEventListener("blur", () => events.push("first:blur"));
    second.addEventListener("focus", () => events.push("second:focus"));
    second.addEventListener("blur", () => events.push("second:blur"));

    mockNative.events.push(keyboardInput(keyDown("Tab", { code: "Tab" })));
    notifyNativeEvents(paintCannon);
    mockNative.events.push(keyboardInput(keyDown("Tab", { code: "Tab" })));
    notifyNativeEvents(paintCannon);
    mockNative.events.push(keyboardInput(keyDown("Tab", { code: "Tab", shiftKey: true })));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      "first:focus",
      "first:blur",
      "second:focus",
      "second:blur",
      "first:focus",
    ]);
    expect(mockNative.textControls.get(first.id)?.focused).toBe(true);
    expect(mockNative.textControls.get(second.id)?.focused).toBe(false);
  });
});

describe("textarea cursor APIs", () => {
  it("returns the native soft-wrapped cursor position", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const textarea = paintCannon.createElement("textarea");
    root.appendChild(textarea);

    expect(textarea.getCursorVisualPosition()).toBeNull();
    mockNative.cursorVisualPositions.set(textarea.id, { row: 3, column: 7 });
    expect(textarea.getCursorVisualPosition()).toEqual({ row: 3, column: 7 });

    expect(textarea.getVisualLineRange(3)).toBeNull();
    mockNative.visualLineRanges.set(`${textarea.id}:3`, { start: 12, end: 18 });
    expect(textarea.getVisualLineRange(3)).toEqual({ start: 12, end: 18 });
    expect(textarea.getVisualLineRange(-1)).toBeNull();
    expect(textarea.getVisualLineRange(1.5)).toBeNull();

    paintCannon.stop();
  });
});

describe("core paste events", () => {
  it("preserves ordering with keyboard events and inserts pasted text by default", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("input");
    const events: string[] = [];
    root.appendChild(input);
    input.focus();
    input.addEventListener("keydown", event => events.push(`key:${event.key}`));
    input.addEventListener("paste", event => {
      events.push(`paste:${event.clipboardData.getData("text/plain")}`);
      expect(event.clipboardData.getData("application/json")).toBe("");
    });

    mockNative.events.push(
      keyboardInput(keyDown("a")),
      pasteInput("BC"),
      keyboardInput(keyDown("d")),
    );
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["key:a", "paste:BC", "key:d"]);
    expect(input.value).toBe("aBCd");
    expect(input.cursorPosition).toBe(4);
  });

  it("bubbles through ancestors before document listeners and emits one change", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("textarea");
    const events: string[] = [];
    root.appendChild(input);
    input.focus();
    input.addEventListener("paste", event => {
      events.push(`input:${event.target === input}:${event.currentTarget === input}`);
    });
    root.addEventListener("paste", event => {
      events.push(`root:${event.target === input}:${event.currentTarget === root}`);
    });
    paintCannon.addEventListener("paste", event => {
      events.push(`document:${event.target === input}`);
    });
    input.addEventListener("change", event => events.push(`change:${event.target.value}`));

    mockNative.events.push(pasteInput("hello\nworld"));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      "input:true:true",
      "root:true:true",
      "document:true",
      "change:hello\nworld",
    ]);
    expect(input.value).toBe("hello\nworld");
  });

  it("lets preventDefault suppress insertion", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement("textarea");
    root.appendChild(input);
    input.focus();
    input.addEventListener("paste", event => event.preventDefault());

    mockNative.events.push(pasteInput("blocked"));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(input.value).toBe("");
  });

  it("exposes terminal image paths as files without exposing or inserting the path text", () => {
    const directory = mkdtempSync(path.join(os.tmpdir(), "paintcannon-event-paste-"));
    try {
      const filePath = path.join(directory, "pasted image.png");
      writeFileSync(filePath, Uint8Array.from([1, 2, 3]));
      const { paintCannon, mockNative, root } = createPaintTree();
      const input = paintCannon.createElement("input");
      root.appendChild(input);
      input.focus();
      input.addEventListener("paste", event => {
        expect(event.clipboardData.getData("text/plain")).toBe("");
        expect(event.clipboardData.types).toEqual(["Files"]);
        expect(event.clipboardData.files[0]).toMatchObject({
          name: "pasted image.png",
          size: 3,
          type: "image/png",
        });
      });

      mockNative.events.push(pasteInput(`'${filePath}'`));
      notifyNativeEvents(paintCannon);
      paintCannon.stop();

      expect(input.value).toBe("");
    } finally {
      rmSync(directory, { force: true, recursive: true });
    }
  });
});

describe("core submit events", () => {
  it("submits the nearest form on input enter and bubbles the submit event", () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const form = paintCannon.createElement("form");
    const input = paintCannon.createElement("input");
    const events: string[] = [];
    root.appendChild(form);
    form.appendChild(input);

    form.addEventListener("submit", event => {
      events.push(
        `form:${event.target === form}:${event.currentTarget === form}:${event.submitter === input}`,
      );
    });
    root.addEventListener("submit", event => {
      events.push(
        `root:${event.target === form}:${event.currentTarget === root}:${event.submitter === input}`,
      );
    });

    input.focus();
    mockNative.events.push(keyboardInput(keyDown("Enter", { code: "Enter" })));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["form:true:true:true", "root:true:true:true"]);
  });

  it("submits a form from a submit button click unless the click default is prevented", () => {
    const { paintCannon, mockNative, root } = createPaintTree({ captureMouse: true });
    const form = paintCannon.createElement("form");
    const button = paintCannon.createElement("button");
    const events: string[] = [];
    root.appendChild(form);
    form.appendChild(button);
    mockNative.targetIdAtPoint = button.id;

    form.addEventListener("submit", event => {
      events.push(`submit:${event.submitter === button}`);
    });
    mockNative.events.push(mouseEvent("click"));
    notifyNativeEvents(paintCannon);

    button.addEventListener("click", event => {
      event.preventDefault();
      events.push("click:prevented");
    });
    mockNative.events.push(mouseEvent("click"));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["submit:true", "click:prevented"]);
  });
});

describe("core mouse events", () => {
  it("bubbles click events and respects stopPropagation", () => {
    const { paintCannon, mockNative, root, child } = createPaintTree({ captureMouse: true });
    const events: string[] = [];
    mockNative.targetIdAtPoint = child.id;

    child.addEventListener("click", event => {
      events.push(`child:${event.target === child}:${event.currentTarget === child}`);
    });
    root.addEventListener("click", event => {
      events.push(`root:${event.target === child}:${event.currentTarget === root}`);
      event.stopPropagation();
    });

    mockNative.events.push(mouseEvent("click"));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["child:true:true", "root:true:true"]);
  });
});

describe("core native scrollbar events", () => {
  it("wraps alternate-screen roots in a private scrollable viewport", () => {
    const paintCannon = new PaintCannon({ alternateScreen: true, fps: 120 });
    const mockNative = currentMockNative();
    const root = paintCannon.createElement("div");

    paintCannon.setRoot(root);
    paintCannon.stop();

    expect(mockNative.viewportId).toBeDefined();
    expect(mockNative.rootId).toBe(mockNative.viewportId);
    expect(mockNative.appendedChildren).toContainEqual({
      parent: mockNative.viewportId,
      child: root.id,
    });
    expect(mockNative.styleMutations).toEqual(
      expect.arrayContaining([
        { id: mockNative.viewportId, property: "width", value: "100%" },
        { id: mockNative.viewportId, property: "height", value: "100%" },
      ]),
    );
  });

  it("uses the alternate-screen viewport as the wheel fallback", () => {
    const paintCannon = new PaintCannon({
      alternateScreen: true,
      captureMouse: true,
      fps: 120,
    });
    const mockNative = currentMockNative();
    const root = paintCannon.createElement("div");
    const child = paintCannon.createElement("div");
    root.appendChild(child);
    paintCannon.setRoot(root);
    const viewportId = mockNative.viewportId;
    if (viewportId === undefined) {
      throw new Error("expected alternate-screen viewport");
    }
    mockNative.targetIdAtPoint = child.id;
    mockNative.scrollMetricsById.set(viewportId, {
      scrollLeft: 0,
      scrollTop: 0,
      scrollWidth: 10,
      scrollHeight: 40,
      clientWidth: 10,
      clientHeight: 10,
    });

    mockNative.events.push(mouseEvent("wheel", { deltaY: 1 }));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(mockNative.scrollMetrics(viewportId).scrollTop).toBe(3);
  });

  it("drags vertical scrollbar thumbs by mapping rail position to scroll offset", () => {
    const { paintCannon, mockNative, child } = createPaintTree({ captureMouse: true });
    const scrollTops: number[] = [];
    child.style.overflowY = "scroll";
    mockNative.scrollMetricsById.set(child.id, {
      scrollLeft: 0,
      scrollTop: 0,
      scrollWidth: 10,
      scrollHeight: 100,
      clientWidth: 10,
      clientHeight: 10,
    });
    mockNative.scrollbarHitAtPoint = {
      targetId: child.id,
      axis: "y",
      railStart: 0,
      railLength: 10,
      thumbStart: 0,
      thumbLength: 1,
      scrollOffset: 0,
      maxScroll: 90,
      clientLength: 10,
      scrollLength: 100,
    };
    child.addEventListener("scroll", event => {
      scrollTops.push(event.scrollTop);
    });

    mockNative.events.push(mouseEvent("mousedown", { y: 0 }), mouseEvent("mousedrag", { y: 5 }));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(mockNative.scrollMetrics(child.id)?.scrollTop).toBe(50);
    expect(scrollTops).toEqual([50]);
  });

  it("pages the scrollbar on rail clicks and suppresses the generated click event", () => {
    const { paintCannon, mockNative, child } = createPaintTree({ captureMouse: true });
    let clicks = 0;
    const scrollTops: number[] = [];
    child.style.overflowY = "scroll";
    mockNative.targetIdAtPoint = child.id;
    mockNative.scrollMetricsById.set(child.id, {
      scrollLeft: 0,
      scrollTop: 0,
      scrollWidth: 10,
      scrollHeight: 100,
      clientWidth: 10,
      clientHeight: 10,
    });
    mockNative.scrollbarHitAtPoint = {
      targetId: child.id,
      axis: "y",
      railStart: 0,
      railLength: 10,
      thumbStart: 0,
      thumbLength: 1,
      scrollOffset: 0,
      maxScroll: 90,
      clientLength: 10,
      scrollLength: 100,
    };
    child.addEventListener("click", () => {
      clicks += 1;
    });
    child.addEventListener("scroll", event => {
      scrollTops.push(event.scrollTop);
    });

    mockNative.events.push(mouseEvent("mousedown", { y: 5 }), mouseEvent("click", { y: 5 }));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(mockNative.scrollMetrics(child.id)?.scrollTop).toBe(10);
    expect(scrollTops).toEqual([10]);
    expect(clicks).toBe(0);
  });
});

describe("core node lifecycle", () => {
  it("cleans up an entire transaction-created subtree when its root is destroyed", () => {
    const paintCannon = new PaintCannon({ captureMouse: true, fps: 120 });
    const mockNative = currentMockNative();
    let root: PaintElement | undefined;
    let child: PaintElement | undefined;
    let grandchild: PaintElement | undefined;

    paintCannon.transaction(() => {
      root = paintCannon.createElement("div");
      child = paintCannon.createElement("div");
      grandchild = paintCannon.createElement("div");
      child.appendChild(grandchild);
      root.appendChild(child);
      paintCannon.setRoot(root);
    });
    if (child === undefined || grandchild === undefined) {
      throw new Error("expected transaction-created subtree");
    }

    let clicks = 0;
    grandchild.addEventListener("click", () => {
      clicks += 1;
    });
    const grandchildId = grandchild.id;
    child.destroy();
    mockNative.targetIdAtPoint = grandchildId;
    mockNative.events.push(mouseEvent("click"));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(clicks).toBe(0);
    expect(mockNative.destroyedNodes).toContain(child.id);
  });
});

describe("core resize events", () => {
  it("dispatches every resize without rendering during native event delivery", () => {
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = currentMockNative();
    const sizes: Array<[number, number]> = [];

    paintCannon.addEventListener("resize", event => {
      sizes.push([event.cols, event.rows]);
    });
    mockNative.events.push(resizeEvent(90, 30), resizeEvent(100, 40));
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(sizes).toEqual([
      [90, 30],
      [100, 40],
    ]);
    expect(mockNative.renderSyncCalls).toBe(0);
  });
});

describe("unified native event ordering", () => {
  it("dispatches mixed native events in queue order", () => {
    const { paintCannon, mockNative, child } = createPaintTree({ captureMouse: true });
    const events: string[] = [];
    mockNative.targetIdAtPoint = child.id;

    paintCannon.addEventListener("keydown", event => events.push(`key:${event.key}`));
    paintCannon.addEventListener("resize", event =>
      events.push(`resize:${event.cols}x${event.rows}`),
    );
    paintCannon.addEventListener("blur", () => events.push("blur"));
    paintCannon.addEventListener("paste", event =>
      events.push(`paste:${event.clipboardData.getData("text/plain")}`),
    );
    child.addEventListener("click", () => events.push("click"));
    child.addEventListener("transitionstart", event =>
      events.push(`transition:${event.propertyName}`),
    );

    mockNative.events.push(keyboardInput(keyDown("a")), resizeEvent(90, 30));
    mockNative.queueFocusEvent("blur");
    mockNative.events.push(
      mouseEvent("click"),
      {
        kind: "transition",
        transition: {
          type: "transitionstart",
          targetId: child.id,
          propertyName: "opacity",
        },
      },
      pasteInput("queued"),
      resizeEvent(100, 40),
    );

    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      "key:a",
      "resize:90x30",
      "blur",
      "click",
      "transition:opacity",
      "paste:queued",
      "resize:100x40",
    ]);
  });
});

describe("core animation lifecycle", () => {
  it("updates the native render loop frame rate", () => {
    const paintCannon = new PaintCannon({ fps: 30 });
    const mockNative = currentMockNative();

    expect(mockNative.fps).toBe(30);
    paintCannon.setFrameRate(90);
    expect(mockNative.fps).toBe(90);

    paintCannon.stop();
  });

  it("runs only requested animation frame callbacks", () => {
    vi.useFakeTimers();
    const paintCannon = new PaintCannon({ fps: 60 });
    let callbackCalls = 0;

    try {
      paintCannon.requestAnimationFrame(() => {
        callbackCalls += 1;
      });

      vi.advanceTimersByTime(17);
      expect(callbackCalls).toBe(1);

      vi.advanceTimersByTime(100);
      expect(callbackCalls).toBe(1);
    } finally {
      paintCannon.stop();
      vi.useRealTimers();
    }
  });

  it("stops an in-flight animation callback cleanly after native Ctrl-C shutdown", () => {
    const paintCannon = new PaintCannon();
    const mockNative = currentMockNative();
    const element = paintCannon.createElement("div");
    let callbackCalls = 0;
    const internals = paintCannon as unknown as {
      animationFrameCallbacks: Map<number, (timestamp: number) => void>;
      runAnimationFrameTick(): void;
    };
    internals.animationFrameCallbacks.set(1, () => {
      callbackCalls += 1;
      element.style.opacity = 0.5;
    });
    mockNative.interruptedByCtrlC = true;
    mockNative.rendererStopped = true;

    try {
      expect(() => internals.runAnimationFrameTick()).not.toThrow();
      expect(mockNative.stopCalls).toBe(1);
      expect(callbackCalls).toBe(0);
    } finally {
      paintCannon.stop();
    }
  });

  it("still surfaces renderer failures not caused by Ctrl-C", () => {
    const paintCannon = new PaintCannon();
    const mockNative = currentMockNative();
    const element = paintCannon.createElement("div");
    const internals = paintCannon as unknown as {
      animationFrameCallbacks: Map<number, (timestamp: number) => void>;
      runAnimationFrameTick(): void;
    };
    internals.animationFrameCallbacks.set(1, () => {
      element.style.opacity = 0.5;
    });
    mockNative.rendererStopped = true;

    try {
      expect(() => internals.runAnimationFrameTick()).toThrow("renderer thread stopped");
      expect(mockNative.stopCalls).toBe(0);
    } finally {
      paintCannon.stop();
    }
  });

  it("handles Ctrl-C racing with an already-running animation callback", () => {
    const paintCannon = new PaintCannon();
    const mockNative = currentMockNative();
    const element = paintCannon.createElement("div");
    const internals = paintCannon as unknown as {
      animationFrameCallbacks: Map<number, (timestamp: number) => void>;
      runAnimationFrameTick(): void;
    };
    internals.animationFrameCallbacks.set(1, () => {
      element.style.opacity = 0.5;
    });
    mockNative.rendererStopped = true;
    mockNative.interruptWhenStyleMutationFails = true;

    try {
      expect(() => internals.runAnimationFrameTick()).not.toThrow();
      expect(mockNative.stopCalls).toBe(1);
    } finally {
      paintCannon.stop();
    }
  });

  it("still surfaces user callback errors during Ctrl-C shutdown", () => {
    const paintCannon = new PaintCannon();
    const mockNative = currentMockNative();
    const internals = paintCannon as unknown as {
      animationFrameCallbacks: Map<number, (timestamp: number) => void>;
      runAnimationFrameTick(): void;
    };
    internals.animationFrameCallbacks.set(1, () => {
      mockNative.interruptedByCtrlC = true;
      throw new Error("user callback failed");
    });

    try {
      expect(() => internals.runAnimationFrameTick()).toThrow("user callback failed");
      expect(mockNative.stopCalls).toBe(0);
    } finally {
      paintCannon.stop();
    }
  });
});

describe("core app focus events", () => {
  it("dispatches an initial blur queued before listener registration", () => {
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = currentMockNative();
    const events: string[] = [];

    mockNative.queueFocusEvent("blur");
    expect(paintCannon.hasFocus).toBe(false);

    paintCannon.addEventListener("blur", event => {
      events.push(`${event.type}:${event.hasFocus}`);
    });
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["blur:false"]);
  });

  it("dispatches terminal focus reports as PaintCannon focus and blur events", () => {
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = currentMockNative();
    const events: string[] = [];

    expect(paintCannon.hasFocus).toBe(true);
    paintCannon.addEventListener("blur", event => {
      events.push(`${event.type}:${event.hasFocus}:${event.target === paintCannon}`);
    });
    paintCannon.addEventListener("focus", event => {
      events.push(`${event.type}:${event.hasFocus}:${event.currentTarget === paintCannon}`);
    });

    mockNative.queueFocusEvent("blur");
    notifyNativeEvents(paintCannon);
    expect(paintCannon.hasFocus).toBe(false);

    mockNative.queueFocusEvent("focus");
    notifyNativeEvents(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["blur:false:true", "focus:true:true"]);
    expect(paintCannon.hasFocus).toBe(true);
  });
});

describe("core style validation", () => {
  it("supports positioned layout and z-index properties", () => {
    const { paintCannon, mockNative, root } = createPaintTree();

    root.style.position = "absolute";
    root.style.top = "10%";
    root.style.right = 2;
    root.style.bottom = "auto";
    root.style.left = -1;
    root.style.zIndex = -3;
    root.style.opacity = 0.5;

    expect(root.style.position).toBe("absolute");
    expect(root.style.top).toBe("10%");
    expect(root.style.right).toBe("2");
    expect(root.style.bottom).toBe("auto");
    expect(root.style.left).toBe("-1");
    expect(root.style.zIndex).toBe("-3");
    expect(root.style.opacity).toBe("0.5");
    expect(mockNative.styleMutations).toEqual(
      expect.arrayContaining([
        { id: root.id, property: "position", value: "absolute" },
        { id: root.id, property: "top", value: "10%" },
        { id: root.id, property: "right", value: "2" },
        { id: root.id, property: "bottom", value: "auto" },
        { id: root.id, property: "left", value: "-1" },
        { id: root.id, property: "z-index", value: "-3" },
        { id: root.id, property: "opacity", value: "0.5" },
      ]),
    );
    paintCannon.stop();
  });

  it("supports width and height constraints and rejects unsupported style keys", () => {
    const { paintCannon, mockNative, root } = createPaintTree();

    root.style.minWidth = 12;
    root.style.maxWidth = "75%";
    root.style.maxHeight = "90%";
    expect(root.style.minWidth).toBe("12");
    expect(root.style.maxWidth).toBe("75%");
    expect(root.style.maxHeight).toBe("90%");
    expect(mockNative.styleMutations).toEqual(
      expect.arrayContaining([
        { id: root.id, property: "min-width", value: "12" },
        { id: root.id, property: "max-width", value: "75%" },
        { id: root.id, property: "max-height", value: "90%" },
      ]),
    );

    expect(root.style.removeProperty("max-width")).toBe("75%");
    expect(root.style.maxWidth).toBe("");
    expect(mockNative.styleMutations).toContainEqual({
      id: root.id,
      property: "max-width",
      value: "",
    });

    const before = mockNative.styleMutations.length;
    expect(() => root.style.setProperty("definitelyNotAProperty" as never, "1")).toThrow(
      /unsupported style property/,
    );
    expect(mockNative.styleMutations).toHaveLength(before);
    paintCannon.stop();
  });

  it("supports terminal text attribute style properties", () => {
    const { paintCannon, mockNative, root } = createPaintTree();

    root.style.fontWeight = "bold";
    root.style.fontStyle = "italic";
    root.style.textDecoration = "underline";
    root.style.textDecorationLine = "line-through";

    expect(root.style.fontWeight).toBe("bold");
    expect(root.style.fontStyle).toBe("italic");
    expect(root.style.textDecoration).toBe("underline");
    expect(root.style.textDecorationLine).toBe("line-through");
    expect(mockNative.styleMutations).toEqual(
      expect.arrayContaining([
        { id: root.id, property: "font-weight", value: "bold" },
        { id: root.id, property: "font-style", value: "italic" },
        { id: root.id, property: "text-decoration", value: "underline" },
        { id: root.id, property: "text-decoration-line", value: "line-through" },
      ]),
    );
    paintCannon.stop();
  });

  it("removes style properties by sending an empty native value", () => {
    const { paintCannon, mockNative, root } = createPaintTree();

    root.style.backgroundColor = "#1e3a5f";
    expect(root.style.backgroundColor).toBe("#1e3a5f");

    const previous = root.style.removeProperty("background-color");

    expect(previous).toBe("#1e3a5f");
    expect(root.style.backgroundColor).toBe("");
    expect(mockNative.styleMutations).toContainEqual({
      id: root.id,
      property: "background-color",
      value: "",
    });
    paintCannon.stop();
  });
});

function createPaintTree(options: { captureMouse?: boolean } = {}): {
  paintCannon: PaintCannon;
  mockNative: MockNativePaintCannon;
  root: PaintElement;
  child: PaintElement;
} {
  const paintCannon = new PaintCannon({ fps: 120, captureMouse: options.captureMouse });
  const root = paintCannon.createElement("div");
  const child = paintCannon.createElement("div");
  paintCannon.setRoot(root);
  root.appendChild(child);
  return {
    paintCannon,
    mockNative: currentMockNative(),
    root,
    child,
  };
}

function currentMockNative(): MockNativePaintCannon {
  const mockNative = mockNativeInstances[mockNativeInstances.length - 1];
  if (mockNative === undefined) {
    throw new Error("expected mock native instance");
  }
  return mockNative;
}

function notifyNativeEvents(_paintCannon: PaintCannon): void {
  currentMockNative().notifyEvents();
}
