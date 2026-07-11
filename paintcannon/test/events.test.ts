import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mock } from "antipattern";
import { PaintCannon, paintCannonDeps, type PaintElement } from "../main.ts";
import {
  createMockNativeBinding,
  keyDown,
  mouseEvent,
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

    mockNative.keyboardEvents.push(keyDown("a"));
    runKeyboardEventPump(paintCannon);
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

    mockNative.keyboardEvents.push(keyDown("a"));
    runKeyboardEventPump(paintCannon);
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
    mockNative.keyboardEvents.push(keyDown("x"));
    runKeyboardEventPump(paintCannon);
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
    mockNative.keyboardEvents.push(keyDown("x"));
    runKeyboardEventPump(paintCannon);
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
    mockNative.keyboardEvents.push(keyDown("a"), keyDown("b"));
    runKeyboardEventPump(paintCannon);
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

    mockNative.keyboardEvents.push(keyDown("Tab", { code: "Tab" }));
    runKeyboardEventPump(paintCannon);
    mockNative.keyboardEvents.push(keyDown("Tab", { code: "Tab" }));
    runKeyboardEventPump(paintCannon);
    mockNative.keyboardEvents.push(keyDown("Tab", { code: "Tab", shiftKey: true }));
    runKeyboardEventPump(paintCannon);
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
    mockNative.keyboardEvents.push(keyDown("Enter", { code: "Enter" }));
    runKeyboardEventPump(paintCannon);
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
    mockNative.mouseEvents.push(mouseEvent("click"));
    runKeyboardEventPump(paintCannon);

    button.addEventListener("click", event => {
      event.preventDefault();
      events.push("click:prevented");
    });
    mockNative.mouseEvents.push(mouseEvent("click"));
    runKeyboardEventPump(paintCannon);
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

    mockNative.mouseEvents.push(mouseEvent("click"));
    runKeyboardEventPump(paintCannon);
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

    mockNative.mouseEvents.push(mouseEvent("wheel", { deltaY: 1 }));
    runKeyboardEventPump(paintCannon);
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

    mockNative.mouseEvents.push(
      mouseEvent("mousedown", { y: 0 }),
      mouseEvent("mousedrag", { y: 5 }),
    );
    runKeyboardEventPump(paintCannon);
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

    mockNative.mouseEvents.push(mouseEvent("mousedown", { y: 5 }), mouseEvent("click", { y: 5 }));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(mockNative.scrollMetrics(child.id)?.scrollTop).toBe(10);
    expect(scrollTops).toEqual([10]);
    expect(clicks).toBe(0);
  });
});

describe("core resize events", () => {
  it("dispatches the latest resize and uses the normal render path", () => {
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = currentMockNative();
    const sizes: Array<[number, number]> = [];

    paintCannon.addEventListener("resize", event => {
      sizes.push([event.cols, event.rows]);
    });
    mockNative.resizeEvents.push({ cols: 90, rows: 30 }, { cols: 100, rows: 40 });
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(sizes).toEqual([[100, 40]]);
    expect(mockNative.renderCalls).toBe(1);
    expect(mockNative.renderSyncCalls).toBe(0);
  });
});

describe("core app focus events", () => {
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

    mockNative.focusEvents.push({ type: "blur" });
    runKeyboardEventPump(paintCannon);
    expect(paintCannon.hasFocus).toBe(false);

    mockNative.focusEvents.push({ type: "focus" });
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(["blur:false:true", "focus:true:true"]);
    expect(paintCannon.hasFocus).toBe(true);
    expect(mockNative.renderCalls).toBe(2);
  });
});

describe("core style validation", () => {
  it("supports maxHeight and rejects unsupported style keys before native calls", () => {
    const { paintCannon, mockNative, root } = createPaintTree();

    root.style.maxHeight = "90%";
    expect(root.style.maxHeight).toBe("90%");
    expect(mockNative.styleMutations).toContainEqual({
      id: root.id,
      property: "max-height",
      value: "90%",
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
    root.style.textDecorationLine = "none";

    expect(root.style.fontWeight).toBe("bold");
    expect(root.style.fontStyle).toBe("italic");
    expect(root.style.textDecoration).toBe("underline");
    expect(root.style.textDecorationLine).toBe("none");
    expect(mockNative.styleMutations).toEqual(
      expect.arrayContaining([
        { id: root.id, property: "font-weight", value: "bold" },
        { id: root.id, property: "font-style", value: "italic" },
        { id: root.id, property: "text-decoration", value: "underline" },
        { id: root.id, property: "text-decoration-line", value: "none" },
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

function runKeyboardEventPump(paintCannon: PaintCannon): void {
  (paintCannon as unknown as { runKeyboardEventPump(): void }).runKeyboardEventPump();
}
