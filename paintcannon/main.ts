import { performance } from "node:perf_hooks";
import { registry } from "antipattern";
import { PaintCannon as NativePaintCannonClass } from "#paintcannon-native";
import { filesFromTerminalPaste, type PaintFileList } from "./terminal-files.ts";
import type {
  BatchCommand as NativeBatchCommand,
  BatchIdMapping as NativeBatchIdMapping,
  CursorVisualPosition as NativeCursorVisualPosition,
  IntersectionEvent as NativeIntersectionEvent,
  IntersectionRect as NativeIntersectionRect,
  KeyboardEvent as NativeKeyboardEvent,
  NativeEvent as NativeQueuedEvent,
  PaintCannon as NativePaintCannon,
  ScrollbarHit as NativeScrollbarHit,
  ScrollMetrics as NativeScrollMetrics,
  TerminalFocusEvent,
  TerminalMouseEvent,
  TerminalResizeEvent,
  TerminalSize,
  TransitionEvent as NativeTransitionEvent,
  VisualLineRange as NativeVisualLineRange,
} from "#paintcannon-native";

export type { PaintFile, PaintFileList, PaintFileType } from "./terminal-files.ts";

export type {
  BatchCommand as NativeBatchCommand,
  BatchIdMapping as NativeBatchIdMapping,
  ClickEvent as NativeClickEvent,
  KeyboardEvent as NativeKeyboardEvent,
  NativeEvent,
  PaintCannon as NativePaintCannon,
  ScrollbarHit as NativeScrollbarHit,
  ScrollMetrics as NativeScrollMetrics,
  TerminalFocusEvent,
  TerminalMouseEvent,
  TerminalResizeEvent,
  TerminalSize,
  TransitionEvent as NativeTransitionEvent,
} from "#paintcannon-native";

const nativeBinding: NativeBinding = {
  PaintCannon: NativePaintCannonClass as NativeBinding["PaintCannon"],
};

export interface PaintCannonOptions {
  fps?: number;
  syntheticKeyupDelayMs?: number;
  forceCompatMode?: boolean;
  captureCtrlC?: boolean;
  captureCtrlZ?: boolean;
  alternateScreen?: boolean;
  captureMouse?: boolean;
}

export type AnimationFrameCallback = (timestamp: number) => void;
export type CursorVisualPosition = NativeCursorVisualPosition;
export type VisualLineRange = NativeVisualLineRange;
export const KEYBOARD_EVENT_TYPES = ["keydown", "keyup"] as const;
export const CLIPBOARD_EVENT_TYPES = ["paste"] as const;
export const CLIPBOARD_WRITE_EVENT_TYPES = ["clipboardWrite"] as const;
export const PAINT_CANNON_FOCUS_EVENT_TYPES = ["focus", "blur"] as const;
export type KeyboardEventType = (typeof KEYBOARD_EVENT_TYPES)[number];
export type ClipboardEventType = (typeof CLIPBOARD_EVENT_TYPES)[number];
export type ClipboardWriteEventType = (typeof CLIPBOARD_WRITE_EVENT_TYPES)[number];
export type PaintCannonFocusEventType = (typeof PAINT_CANNON_FOCUS_EVENT_TYPES)[number];
export type PaintCannonEventType =
  | KeyboardEventType
  | ClipboardEventType
  | ClipboardWriteEventType
  | PaintCannonFocusEventType
  | "resize";
export type KeyboardEventListener = (event: PaintKeyboardEvent) => void;
export type ClipboardEventListener = (event: PaintClipboardEvent) => void;
export type ClipboardWriteEventListener = (event: PaintClipboardWriteEvent) => void;
export type PaintCannonFocusEventListener = (event: PaintCannonFocusEvent) => void;
export type ResizeEventListener = (event: PaintResizeEvent) => void;

export type IntersectionObserverCallback = (
  entries: IntersectionObserverEntry[],
  observer: IntersectionObserver,
) => void;

export type IntersectionObserverInit = {
  root?: PaintElement | null;
  rootMargin?: string;
  threshold?: number | number[];
};

const syncIntersectionTarget = Symbol("syncIntersectionTarget");
const removeIntersectionTarget = Symbol("removeIntersectionTarget");
const enqueueIntersectionEntries = Symbol("enqueueIntersectionEntries");
const stopIntersectionObserver = Symbol("stopIntersectionObserver");
const registerIntersectionObserver = Symbol("registerIntersectionObserver");
const observeNativeIntersection = Symbol("observeNativeIntersection");
const unobserveNativeIntersection = Symbol("unobserveNativeIntersection");
const disconnectNativeIntersectionObserver = Symbol("disconnectNativeIntersectionObserver");
export const MOUSE_ELEMENT_EVENT_TYPES = [
  "click",
  "mouseenter",
  "mouseleave",
  "mousemove",
] as const;
export const FOCUS_ELEMENT_EVENT_TYPES = ["focus", "blur"] as const;
export const FORM_ELEMENT_EVENT_TYPES = ["submit"] as const;
export const CHANGE_ELEMENT_EVENT_TYPES = ["change"] as const;
export const TRANSITION_ELEMENT_EVENT_TYPES = ["transitionstart", "transitionend"] as const;
const TRANSITION_ELEMENT_EVENT_TYPE_SET = new Set<string>(TRANSITION_ELEMENT_EVENT_TYPES);
export const ELEMENT_EVENT_TYPES = [
  ...KEYBOARD_EVENT_TYPES,
  ...CLIPBOARD_EVENT_TYPES,
  ...MOUSE_ELEMENT_EVENT_TYPES,
  ...FOCUS_ELEMENT_EVENT_TYPES,
  ...FORM_ELEMENT_EVENT_TYPES,
  ...CHANGE_ELEMENT_EVENT_TYPES,
  ...TRANSITION_ELEMENT_EVENT_TYPES,
  "scroll",
] as const;
export type MouseElementEventType = (typeof MOUSE_ELEMENT_EVENT_TYPES)[number];
export type FocusElementEventType = (typeof FOCUS_ELEMENT_EVENT_TYPES)[number];
export type FormElementEventType = (typeof FORM_ELEMENT_EVENT_TYPES)[number];
export type ChangeElementEventType = (typeof CHANGE_ELEMENT_EVENT_TYPES)[number];
export type TransitionElementEventType = (typeof TRANSITION_ELEMENT_EVENT_TYPES)[number];
export type ElementEventType = (typeof ELEMENT_EVENT_TYPES)[number];
export type MouseEventListener = (event: PaintMouseEvent) => void;
export type FocusEventListener = (event: PaintFocusEvent) => void;
export type SubmitEventListener = (event: PaintSubmitEvent) => void;
export type ChangeEventListener = (event: PaintChangeEvent) => void;
export type ScrollEventListener = (event: PaintScrollEvent) => void;
export type TransitionEventListener = (event: PaintTransitionEvent) => void;
type ElementEventListenerFunction =
  | KeyboardEventListener
  | ClipboardEventListener
  | MouseEventListener
  | FocusEventListener
  | SubmitEventListener
  | ChangeEventListener
  | ScrollEventListener
  | TransitionEventListener;
type ElementEventListenerTuple = readonly [ElementEventType, ElementEventListenerFunction];
type ElementEventListenerTuplesFor<
  TTypes extends readonly ElementEventType[],
  TListener extends ElementEventListenerFunction,
> = {
  [Index in keyof TTypes]: TTypes[Index] extends ElementEventType
    ? readonly [TTypes[Index], TListener]
    : never;
}[number];
type KeyboardElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof KEYBOARD_EVENT_TYPES,
  KeyboardEventListener
>;
type ClipboardElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof CLIPBOARD_EVENT_TYPES,
  ClipboardEventListener
>;
type MouseElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof MOUSE_ELEMENT_EVENT_TYPES,
  MouseEventListener
>;
type FocusElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof FOCUS_ELEMENT_EVENT_TYPES,
  FocusEventListener
>;
type FormElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof FORM_ELEMENT_EVENT_TYPES,
  SubmitEventListener
>;
type ChangeElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof CHANGE_ELEMENT_EVENT_TYPES,
  ChangeEventListener
>;
type TransitionElementEventListenerTuple = ElementEventListenerTuplesFor<
  typeof TRANSITION_ELEMENT_EVENT_TYPES,
  TransitionEventListener
>;
type ScrollElementEventListenerTuple = readonly ["scroll", ScrollEventListener];
type ExactElementEventListenerTuple<TEvents extends ElementEventListenerTuple> = [
  Exclude<ElementEventType, TEvents[0]>,
  Exclude<TEvents[0], ElementEventType>,
] extends [never, never]
  ? TEvents
  : never;
type AllElementEventListenerTuple = ExactElementEventListenerTuple<
  | KeyboardElementEventListenerTuple
  | ClipboardElementEventListenerTuple
  | MouseElementEventListenerTuple
  | FocusElementEventListenerTuple
  | FormElementEventListenerTuple
  | ChangeElementEventListenerTuple
  | ScrollElementEventListenerTuple
  | TransitionElementEventListenerTuple
>;
type BasicElementEventListenerTuple =
  | KeyboardElementEventListenerTuple
  | ClipboardElementEventListenerTuple
  | MouseElementEventListenerTuple
  | FocusElementEventListenerTuple
  | ChangeElementEventListenerTuple
  | TransitionElementEventListenerTuple;
type ContainerElementEventListenerTuple = AllElementEventListenerTuple;
type TextAreaElementEventListenerTuple =
  | BasicElementEventListenerTuple
  | ScrollElementEventListenerTuple;
export type ElementEventListenerMap = {
  [TType in AllElementEventListenerTuple[0]]: Extract<
    AllElementEventListenerTuple,
    readonly [TType, ElementEventListenerFunction]
  >[1];
};
export type ElementEventListenerFor<T extends ElementEventType> = ElementEventListenerMap[T];
type ElementEventListener = ElementEventListenerFor<ElementEventType>;
type EventListenerForTuple<
  TEvents extends ElementEventListenerTuple,
  TType extends ElementEventType,
> = Extract<TEvents, readonly [TType, ElementEventListenerFunction]>[1];
export type ClickEventListener = MouseEventListener;
export type ImageRendering = "ascii" | "half-block";
export type CSSVisibility = "visible" | "hidden";
export type CSSPosition = "static" | "relative" | "absolute";
export type CSSWhiteSpace = "normal" | "nowrap" | "pre" | "pre-wrap" | "pre-line";
export type CSSOverflowWrap = "normal" | "break-word" | "anywhere";
export type CSSOverflowAnchor = "auto" | "none";
export type CSSWordBreak = "normal" | "break-all" | "keep-all" | "break-word";
export type CSSFontWeight = "normal" | "bold";
export type CSSFontStyle = "normal" | "italic";
export type CSSTextDecoration = "none" | "underline" | "line-through";
export type CSSStyleValue = string | number;
export type CSSCursor =
  | "auto"
  | "alias"
  | "cell"
  | "copy"
  | "crosshair"
  | "default"
  | "e-resize"
  | "ew-resize"
  | "grab"
  | "grabbing"
  | "help"
  | "move"
  | "n-resize"
  | "ne-resize"
  | "nesw-resize"
  | "no-drop"
  | "not-allowed"
  | "ns-resize"
  | "nw-resize"
  | "nwse-resize"
  | "pointer"
  | "progress"
  | "s-resize"
  | "se-resize"
  | "sw-resize"
  | "text"
  | "vertical-text"
  | "w-resize"
  | "wait"
  | "zoom-in"
  | "zoom-out";

function isTransitionElementEventType(type: string): type is TransitionElementEventType {
  return TRANSITION_ELEMENT_EVENT_TYPE_SET.has(type);
}

export class PaintKeyboardEvent {
  readonly type: KeyboardEventType;
  readonly target: PaintElement | undefined;
  currentTarget: PaintElement | undefined;
  readonly key: string;
  readonly code: string;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly repeat: boolean;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: NativeKeyboardEvent, target?: PaintElement) {
    this.type = event.type as KeyboardEventType;
    this.target = target;
    this.currentTarget = target;
    this.key = event.key;
    this.code = event.code;
    this.ctrlKey = event.ctrlKey;
    this.altKey = event.altKey;
    this.metaKey = event.metaKey;
    this.shiftKey = event.shiftKey;
    this.repeat = event.repeat;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export type KeyboardEvent = PaintKeyboardEvent;

export class PaintDataTransfer {
  readonly types: readonly string[];
  readonly files: PaintFileList;
  private readonly text: string;

  constructor(text: string) {
    this.files = filesFromTerminalPaste(text);
    this.text = this.files.length > 0 ? "" : text;
    this.types = Object.freeze(this.files.length > 0 ? ["Files"] : ["text/plain"]);
  }

  getData(format: string): string {
    if (this.files.length > 0) {
      return "";
    }
    const normalized = format.toLowerCase();
    return normalized === "text" || normalized === "text/plain" ? this.text : "";
  }
}

export class PaintClipboardEvent {
  readonly type: ClipboardEventType = "paste";
  readonly target: PaintElement | undefined;
  currentTarget: PaintElement | undefined;
  readonly clipboardData: PaintDataTransfer;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(data: string, target?: PaintElement) {
    this.target = target;
    this.currentTarget = target;
    this.clipboardData = new PaintDataTransfer(data);
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export type ClipboardEvent = PaintClipboardEvent;
export class PaintClipboardWriteEvent {
  readonly type: "clipboardWrite" = "clipboardWrite";
  readonly text: string;
  readonly success: boolean;

  constructor(text: string, success: boolean) {
    this.text = text;
    this.success = success;
  }
}

export type ClipboardWriteEvent = PaintClipboardWriteEvent;
export type DataTransfer = PaintDataTransfer;
export type { PaintFile as File, PaintFileList as FileList } from "./terminal-files.ts";

export type NativeBinding = {
  PaintCannon: new (
    ...args: ConstructorParameters<typeof NativePaintCannonClass>
  ) => NativePaintCannon;
};

type TextControlElement = InputElement | TextAreaElement;
type ScrollbarAxis = "x" | "y";
type ActiveScrollbarDrag = {
  target: PaintElement;
  axis: ScrollbarAxis;
  dragOffset: number;
  railStart: number;
  railLength: number;
  thumbLength: number;
  maxScroll: number;
};
export const PAINT_ELEMENT_TAG_NAMES = [
  "div",
  "span",
  "form",
  "button",
  "img",
  "input",
  "textarea",
] as const;
export type PaintElementTagName = (typeof PAINT_ELEMENT_TAG_NAMES)[number];
const PAINT_ELEMENT_TAG_NAME_SET = new Set<string>(PAINT_ELEMENT_TAG_NAMES);
type ExactPaintElementTagMap<T extends { [Tag in PaintElementTagName]: object }> =
  Exclude<keyof T, PaintElementTagName> extends never ? T : never;
export type PaintElementByTagName = ExactPaintElementTagMap<{
  div: DivElement;
  span: SpanElement;
  form: FormElement;
  button: ButtonElement;
  img: ImageElement;
  input: InputElement;
  textarea: TextAreaElement;
}>;
export type PaintElement = PaintElementByTagName[PaintElementTagName];
export type PaintNode = PaintElement | TextNode;

export const paintCannonDeps = registry({
  loadNativeBinding,
});

const livePaintCannons = new Set<PaintCannon>();
let processCleanupInstalled = false;
let handlingFatalError = false;
const cleanupSignalHandlers = new Map<NodeJS.Signals, NodeJS.SignalsListener>();

function registerLivePaintCannon(paintCannon: PaintCannon): void {
  livePaintCannons.add(paintCannon);
  installProcessCleanupHandlers();
}

function cleanupLivePaintCannons(): void {
  for (const paintCannon of Array.from(livePaintCannons)) {
    try {
      paintCannon.releaseForProcessExit();
    } catch {
      // Process cleanup must be best-effort; throwing here would hide the real crash.
    }
  }
}

function installProcessCleanupHandlers(): void {
  if (processCleanupInstalled) {
    return;
  }

  processCleanupInstalled = true;
  process.once("exit", cleanupLivePaintCannons);

  for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
    const handler: NodeJS.SignalsListener = () => {
      if (handlingFatalError) {
        return;
      }

      handlingFatalError = true;
      cleanupLivePaintCannons();
      const installedHandler = cleanupSignalHandlers.get(signal);
      if (installedHandler !== undefined) {
        process.off(signal, installedHandler);
        cleanupSignalHandlers.delete(signal);
      }
      process.kill(process.pid, signal);
    };

    cleanupSignalHandlers.set(signal, handler);
    process.on(signal, handler);
  }
}

export class DOMRectReadOnly {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly left: number;

  constructor(x = 0, y = 0, width = 0, height = 0) {
    this.x = x;
    this.y = y;
    this.width = width;
    this.height = height;
    this.top = y;
    this.right = x + width;
    this.bottom = y + height;
    this.left = x;
  }

  toJSON(): Record<"x" | "y" | "width" | "height" | "top" | "right" | "bottom" | "left", number> {
    return {
      x: this.x,
      y: this.y,
      width: this.width,
      height: this.height,
      top: this.top,
      right: this.right,
      bottom: this.bottom,
      left: this.left,
    };
  }
}

export class IntersectionObserverEntry {
  readonly time: number;
  readonly rootBounds: DOMRectReadOnly | null;
  readonly boundingClientRect: DOMRectReadOnly;
  readonly intersectionRect: DOMRectReadOnly;
  readonly isIntersecting: boolean;
  readonly intersectionRatio: number;
  readonly target: PaintElement;

  constructor(entry: IntersectionObserverEntryInit) {
    this.time = entry.time;
    this.rootBounds = entry.rootBounds;
    this.boundingClientRect = entry.boundingClientRect;
    this.intersectionRect = entry.intersectionRect;
    this.isIntersecting = entry.isIntersecting;
    this.intersectionRatio = entry.intersectionRatio;
    this.target = entry.target;
  }
}

export type IntersectionObserverEntryInit = {
  time: number;
  rootBounds: DOMRectReadOnly | null;
  boundingClientRect: DOMRectReadOnly;
  intersectionRect: DOMRectReadOnly;
  isIntersecting: boolean;
  intersectionRatio: number;
  target: PaintElement;
};

export class IntersectionObserver {
  readonly root: PaintElement | null;
  readonly rootMargin: string;
  readonly thresholds: readonly number[];
  private readonly callback: IntersectionObserverCallback;
  private readonly rootMarginValues: string[];
  private readonly targets = new Set<PaintElement>();
  private readonly records: IntersectionObserverEntry[] = [];
  private ownerDocument: PaintCannon | undefined;
  private observerId: number | undefined;
  private callbackScheduled = false;

  constructor(callback: IntersectionObserverCallback, options: IntersectionObserverInit = {}) {
    if (typeof callback !== "function") {
      throw new TypeError("IntersectionObserver callback must be a function");
    }
    this.callback = callback;
    this.root = options.root ?? null;
    if (this.root !== null) {
      assertElement(this.root);
    }
    this.rootMarginValues = parseRootMargin(options.rootMargin ?? "0px");
    this.rootMargin = this.rootMarginValues.join(" ");
    this.thresholds = Object.freeze(normalizeThresholds(options.threshold));
    if (this.root !== null) {
      this.attach(this.root.ownerDocument);
    }
  }

  observe(target: PaintElement): void {
    assertElement(target);
    this.attach(target.ownerDocument);
    if (this.targets.has(target)) {
      return;
    }
    this.targets.add(target);
    this[syncIntersectionTarget](target);
  }

  unobserve(target: PaintElement): void {
    assertElement(target);
    if (!this.targets.delete(target)) {
      return;
    }
    if (this.ownerDocument === target.ownerDocument && this.observerId !== undefined) {
      this.ownerDocument[unobserveNativeIntersection](this.observerId, target);
    }
  }

  disconnect(): void {
    this.targets.clear();
    this.records.length = 0;
    if (this.ownerDocument !== undefined && this.observerId !== undefined) {
      this.ownerDocument[disconnectNativeIntersectionObserver](this.observerId);
    }
  }

  takeRecords(): IntersectionObserverEntry[] {
    return this.records.splice(0);
  }

  [syncIntersectionTarget](target: PaintElement): void {
    if (
      !this.targets.has(target) ||
      this.ownerDocument === undefined ||
      this.observerId === undefined
    ) {
      return;
    }
    this.ownerDocument[observeNativeIntersection](
      this,
      this.observerId,
      target,
      this.root,
      this.rootMarginValues,
      this.thresholds,
    );
  }

  [removeIntersectionTarget](target: PaintElement): boolean {
    return this.targets.delete(target);
  }

  [enqueueIntersectionEntries](entries: IntersectionObserverEntry[]): void {
    this.records.push(...entries.filter(entry => this.targets.has(entry.target)));
    if (this.records.length === 0) {
      return;
    }
    if (this.callbackScheduled) {
      return;
    }
    this.callbackScheduled = true;
    queueMicrotask(() => {
      this.callbackScheduled = false;
      const records = this.takeRecords();
      if (records.length > 0 && this.ownerDocument !== undefined) {
        this.callback(records, this);
      }
    });
  }

  [stopIntersectionObserver](): void {
    this.targets.clear();
    this.records.length = 0;
    this.ownerDocument = undefined;
    this.observerId = undefined;
  }

  private attach(ownerDocument: PaintCannon): void {
    if (this.ownerDocument !== undefined && this.ownerDocument !== ownerDocument) {
      throw new Error(
        "an IntersectionObserver cannot observe elements from different PaintCannons",
      );
    }
    if (this.ownerDocument === undefined) {
      this.ownerDocument = ownerDocument;
      this.observerId = ownerDocument[registerIntersectionObserver](this);
    }
  }
}

export class PaintCannon {
  private readonly binding: NativePaintCannon;
  private animationFrameIntervalMs: number;
  private stopped = false;
  private nextAnimationFrameId = 1;
  private nextIntersectionObserverId = 1;
  private animationFrameTimer: NodeJS.Timeout | undefined;
  private suspendedByPaintCannon = false;
  private transactionDepth = 0;
  private nextTemporaryId = -1;
  private batchCommands: NativeBatchCommand[] = [];
  private batchNodes = new Map<number, PaintNodeBase>();
  private readonly captureCtrlZ: boolean;
  private readonly captureMouse: boolean;
  private readonly animationFrameCallbacks = new Map<number, AnimationFrameCallback>();
  private readonly textControls = new Set<TextControlElement>();
  private focusedElement: PaintElement | undefined;

  private get focusedTextControl(): TextControlElement | undefined {
    return isTextControl(this.focusedElement) ? this.focusedElement : undefined;
  }
  private readonly keyboardEventListeners: Record<KeyboardEventType, Set<KeyboardEventListener>> = {
    keydown: new Set(),
    keyup: new Set(),
  };
  private readonly clipboardEventListeners = new Set<ClipboardEventListener>();
  private readonly clipboardWriteEventListeners = new Set<ClipboardWriteEventListener>();
  private readonly focusEventListeners: Record<
    PaintCannonFocusEventType,
    Set<PaintCannonFocusEventListener>
  > = {
    focus: new Set(),
    blur: new Set(),
  };
  private readonly resizeEventListeners = new Set<ResizeEventListener>();
  private readonly elements = new Map<number, PaintElement>();
  private readonly parents = new Map<number, PaintElement>();
  private readonly children = new Map<number, Set<number>>();
  private readonly elementEventListeners = new Map<
    number,
    Partial<Record<ElementEventType, Set<ElementEventListener>>>
  >();
  private readonly scrollMetrics = new Map<number, NativeScrollMetrics>();
  private readonly intersectionObservers = new Map<number, IntersectionObserver>();
  private readonly pendingIntersectionTargets = new Map<IntersectionObserver, Set<PaintElement>>();
  private scrollbarDrag: ActiveScrollbarDrag | undefined;
  private suppressNextScrollbarClick = false;
  private readonly elementFactories: {
    [Tag in PaintElementTagName]: () => PaintElementByTagName[Tag];
  } = {
    div: () => this.createDivElement(),
    span: () => this.createSpanElement(),
    form: () => this.createFormElement(),
    button: () => this.createButtonElement(),
    img: () => this.createImageElement(),
    input: () => this.createInputElement(),
    textarea: () => this.createTextAreaElement(),
  };
  private hoveredElement: PaintElement | undefined;
  private rootElement: PaintElement | undefined;
  private readonly viewportElement: DivElement | undefined;
  private readonly handleSigcont = () => {
    if (!this.suspendedByPaintCannon || this.stopped) {
      return;
    }

    this.suspendedByPaintCannon = false;
    this.binding.captureTerminal();
    this.binding.invalidateFrame();
  };

  constructor(options: PaintCannonOptions = {}) {
    const binding = paintCannonDeps.loadNativeBinding();
    const alternateScreen = options.alternateScreen ?? false;
    const fps = options.fps ?? 60;
    this.animationFrameIntervalMs = fpsToInterval(fps);
    this.binding = new binding.PaintCannon(
      options.forceCompatMode ?? false,
      alternateScreen,
      options.captureMouse ?? false,
      options.captureCtrlC ?? false,
      fps,
      () => this.handleNativeEventNotification(),
    );
    this.captureCtrlZ = options.captureCtrlZ ?? false;
    this.captureMouse = options.captureMouse ?? false;
    if (alternateScreen) {
      const viewport = this.createDivElement();
      viewport.style.width = "100%";
      viewport.style.height = "100%";
      this.binding.setViewport(viewport.id);
      this.setNativeRoot(viewport.id);
      this.viewportElement = viewport;
    } else {
      this.viewportElement = undefined;
    }
    registerLivePaintCannon(this);
    process.on("SIGCONT", this.handleSigcont);
    if (options.syntheticKeyupDelayMs !== undefined) {
      this.setSyntheticKeyupDelay(options.syntheticKeyupDelayMs);
    }
  }

  createElement<T extends PaintElementTagName>(tagName: T): PaintElementByTagName[T] {
    if (!isPaintElementTagName(tagName)) {
      const supported = PAINT_ELEMENT_TAG_NAMES.map(tag => `<${tag}>`).join(", ");
      throw new Error(`paintcannon only supports ${supported} right now, got <${tagName}>`);
    }

    return this.elementFactories[tagName]();
  }

  private createDivElement(): DivElement {
    const element = new DivElement(
      this,
      this.createNativeDiv(),
      (parent, child) => this.appendNativeChild(parent, child),
      (parent, child, before) => this.insertNativeChildBefore(parent, child, before),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    return element;
  }

  private createSpanElement(): SpanElement {
    const element = new SpanElement(
      this,
      this.createNativeSpan(),
      (parent, child) => this.appendNativeChild(parent, child),
      (parent, child, before) => this.insertNativeChildBefore(parent, child, before),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    return element;
  }

  private createFormElement(): FormElement {
    const element = new FormElement(
      this,
      this.createNativeDiv(),
      (parent, child) => this.appendNativeChild(parent, child),
      (parent, child, before) => this.insertNativeChildBefore(parent, child, before),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    return element;
  }

  private createButtonElement(): ButtonElement {
    const element = new ButtonElement(
      this,
      this.createNativeDiv(),
      (parent, child) => this.appendNativeChild(parent, child),
      (parent, child, before) => this.insertNativeChildBefore(parent, child, before),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    return element;
  }

  private createImageElement(): ImageElement {
    const element = new ImageElement(
      this,
      this.createNativeImage(),
      (id, src) => this.setNativeImageSource(id, src),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    return element;
  }

  private createInputElement(): InputElement {
    const element = new InputElement(
      this,
      this.createNativeInput(),
      (id, value, cursor) => this.setNativeInputValue(id, value, cursor),
      (id, focused) => this.setNativeInputFocused(id, focused),
      (id, placeholder) => this.setNativeInputPlaceholder(id, placeholder),
      (id, x, y) => this.binding.setTextControlCursorAtPoint(id, x, y),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    this.textControls.add(element);
    return element;
  }

  private createTextAreaElement(): TextAreaElement {
    const element = new TextAreaElement(
      this,
      this.createNativeTextArea(),
      (id, value, cursor) => this.setNativeTextAreaValue(id, value, cursor),
      (id, focused) => this.setNativeTextAreaFocused(id, focused),
      (id, placeholder) => this.setNativeTextAreaPlaceholder(id, placeholder),
      (id, x, y) => this.binding.setTextControlCursorAtPoint(id, x, y),
      (id, direction) => this.binding.moveTextAreaCursorVertically(id, direction),
      id => this.binding.getTextAreaCursorVisualPosition(id),
      (id, row) => this.binding.getTextAreaVisualLineRange(id, row),
      (id, property, value) => this.setNativeStyleProperty(id, property, value),
    );
    this.registerElement(element);
    this.textControls.add(element);
    return element;
  }

  createTextNode(data: string): TextNode {
    const text = String(data);
    const node = new TextNode(
      this,
      this.createNativeTextNode(text),
      (id, value) => this.setNativeTextNodeValue(id, value),
      text,
    );
    this.registerBatchNode(node);
    return node;
  }

  setRoot(element: PaintElement): void {
    assertElement(element);
    const viewport = this.viewportElement;
    if (viewport !== undefined) {
      const previousRoot = this.rootElement;
      if (
        previousRoot !== undefined &&
        previousRoot !== element &&
        this.parents.get(previousRoot.id) === viewport
      ) {
        viewport.detachChild(previousRoot);
      }
      if (this.parents.get(element.id) !== viewport) {
        viewport.appendChild(element);
      }
    } else {
      this.setNativeRoot(element.id);
    }
    this.rootElement = element;
  }

  get terminalSize(): TerminalSize {
    return this.binding.terminalSize();
  }

  get kittyKeyboardEnabled(): boolean {
    return this.binding.kittyKeyboardEnabled;
  }

  get hasFocus(): boolean {
    return this.binding.hasFocus;
  }

  setSyntheticKeyupDelay(delayMs: number): void {
    if (!Number.isFinite(delayMs) || delayMs < 0) {
      throw new Error(`synthetic keyup delay must be a non-negative number, got ${delayMs}`);
    }

    this.binding.setSyntheticKeyupDelay(Math.floor(delayMs));
  }

  setFrameRate(fps: number): void {
    this.animationFrameIntervalMs = fpsToInterval(fps);
    this.binding.setFrameRate(fps);

    if (this.animationFrameTimer !== undefined) {
      clearTimeout(this.animationFrameTimer);
      this.animationFrameTimer = undefined;
      this.scheduleAnimationFrameTick();
    }
  }

  requestAnimationFrame(callback: AnimationFrameCallback): number {
    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }

    const id = this.nextAnimationFrameId++;
    this.animationFrameCallbacks.set(id, callback);
    this.scheduleAnimationFrameTick();
    return id;
  }

  cancelAnimationFrame(id: number): void {
    this.animationFrameCallbacks.delete(id);
  }

  [registerIntersectionObserver](observer: IntersectionObserver): number {
    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }
    const id = this.nextIntersectionObserverId++;
    this.intersectionObservers.set(id, observer);
    return id;
  }

  [observeNativeIntersection](
    observer: IntersectionObserver,
    observerId: number,
    target: PaintElement,
    root: PaintElement | null,
    rootMargin: string[],
    thresholds: readonly number[],
  ): void {
    if (target.id < 0 || (root !== null && root.id < 0)) {
      let targets = this.pendingIntersectionTargets.get(observer);
      if (targets === undefined) {
        targets = new Set();
        this.pendingIntersectionTargets.set(observer, targets);
      }
      targets.add(target);
      return;
    }
    this.binding.observeIntersection(
      observerId,
      target.id,
      root?.id,
      rootMargin,
      Array.from(thresholds),
    );
  }

  [unobserveNativeIntersection](observerId: number, target: PaintElement): void {
    for (const targets of this.pendingIntersectionTargets.values()) {
      targets.delete(target);
    }
    if (target.id >= 0) {
      this.binding.unobserveIntersection(observerId, target.id);
    }
  }

  [disconnectNativeIntersectionObserver](observerId: number): void {
    const observer = this.intersectionObservers.get(observerId);
    if (observer !== undefined) {
      this.pendingIntersectionTargets.delete(observer);
    }
    this.binding.disconnectIntersectionObserver(observerId);
  }

  transaction<T>(callback: () => T): T {
    const isOuterTransaction = this.transactionDepth === 0;
    this.beginTransaction();

    let result: T;
    try {
      result = callback();
    } catch (error) {
      this.transactionDepth -= 1;
      if (isOuterTransaction) {
        this.rollbackTransaction();
      }
      throw error;
    }

    this.transactionDepth -= 1;
    if (isOuterTransaction) {
      this.flushTransaction();
    }

    return result;
  }

  beginTransaction(): void {
    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }

    this.transactionDepth += 1;
  }

  commitTransaction(): void {
    if (this.transactionDepth <= 0) {
      throw new Error("no active paintcannon transaction to commit");
    }

    this.transactionDepth -= 1;
    if (this.transactionDepth === 0) {
      this.flushTransaction();
    }
  }

  addEventListener(type: KeyboardEventType, listener: KeyboardEventListener): void;
  addEventListener(type: ClipboardEventType, listener: ClipboardEventListener): void;
  addEventListener(type: ClipboardWriteEventType, listener: ClipboardWriteEventListener): void;
  addEventListener(type: PaintCannonFocusEventType, listener: PaintCannonFocusEventListener): void;
  addEventListener(type: "resize", listener: ResizeEventListener): void;
  addEventListener(
    type: PaintCannonEventType,
    listener:
      | KeyboardEventListener
      | ClipboardEventListener
      | ClipboardWriteEventListener
      | PaintCannonFocusEventListener
      | ResizeEventListener,
  ): void {
    if (
      type !== "keydown" &&
      type !== "keyup" &&
      type !== "paste" &&
      type !== "clipboardWrite" &&
      type !== "focus" &&
      type !== "blur" &&
      type !== "resize"
    ) {
      throw new Error(`unsupported event type: ${type}`);
    }

    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }

    if (type === "resize") {
      this.resizeEventListeners.add(listener as ResizeEventListener);
    } else if (type === "paste") {
      this.clipboardEventListeners.add(listener as ClipboardEventListener);
    } else if (type === "clipboardWrite") {
      this.clipboardWriteEventListeners.add(listener as ClipboardWriteEventListener);
    } else if (type === "focus" || type === "blur") {
      this.focusEventListeners[type].add(listener as PaintCannonFocusEventListener);
    } else {
      this.keyboardEventListeners[type].add(listener as KeyboardEventListener);
    }
  }

  removeEventListener(type: KeyboardEventType, listener: KeyboardEventListener): void;
  removeEventListener(type: ClipboardEventType, listener: ClipboardEventListener): void;
  removeEventListener(type: ClipboardWriteEventType, listener: ClipboardWriteEventListener): void;
  removeEventListener(
    type: PaintCannonFocusEventType,
    listener: PaintCannonFocusEventListener,
  ): void;
  removeEventListener(type: "resize", listener: ResizeEventListener): void;
  removeEventListener(
    type: PaintCannonEventType,
    listener:
      | KeyboardEventListener
      | ClipboardEventListener
      | ClipboardWriteEventListener
      | PaintCannonFocusEventListener
      | ResizeEventListener,
  ): void {
    if (
      type !== "keydown" &&
      type !== "keyup" &&
      type !== "paste" &&
      type !== "clipboardWrite" &&
      type !== "focus" &&
      type !== "blur" &&
      type !== "resize"
    ) {
      return;
    }

    if (type === "resize") {
      this.resizeEventListeners.delete(listener as ResizeEventListener);
    } else if (type === "paste") {
      this.clipboardEventListeners.delete(listener as ClipboardEventListener);
    } else if (type === "clipboardWrite") {
      this.clipboardWriteEventListeners.delete(listener as ClipboardWriteEventListener);
    } else if (type === "focus" || type === "blur") {
      this.focusEventListeners[type].delete(listener as PaintCannonFocusEventListener);
    } else {
      this.keyboardEventListeners[type].delete(listener as KeyboardEventListener);
    }
  }

  addElementEventListener(
    element: ElementEventTarget,
    type: ElementEventType,
    listener: ElementEventListener,
  ): void {
    if (!isElementEventType(type)) {
      throw new Error(`unsupported event type: ${type}`);
    }

    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }

    let eventListeners = this.elementEventListeners.get(element.id);
    if (eventListeners === undefined) {
      eventListeners = {};
      this.elementEventListeners.set(element.id, eventListeners);
    }

    let listeners = eventListeners[type];
    if (listeners === undefined) {
      listeners = new Set();
      eventListeners[type] = listeners;
    }
    listeners.add(listener);
  }

  removeElementEventListener(
    element: ElementEventTarget,
    type: ElementEventType,
    listener: ElementEventListener,
  ): void {
    if (!isElementEventType(type)) {
      return;
    }

    const eventListeners = this.elementEventListeners.get(element.id);
    const listeners = eventListeners?.[type];
    listeners?.delete(listener);
    if (listeners?.size === 0) {
      delete eventListeners?.[type];
    }
    if (eventListeners !== undefined && Object.keys(eventListeners).length === 0) {
      this.elementEventListeners.delete(element.id);
    }
  }

  renderSync(): void {
    if (this.stopped) {
      throw new Error("paintcannon renderer has been stopped");
    }
    if (this.isTransactionActive()) {
      throw new Error("cannot render synchronously during a transaction");
    }
    this.binding.renderSync();
  }

  stop(): void {
    if (this.stopped) {
      return;
    }

    this.stopped = true;
    if (this.animationFrameTimer !== undefined) {
      clearTimeout(this.animationFrameTimer);
      this.animationFrameTimer = undefined;
    }
    this.animationFrameCallbacks.clear();
    this.keyboardEventListeners.keydown.clear();
    this.keyboardEventListeners.keyup.clear();
    this.clipboardEventListeners.clear();
    this.clipboardWriteEventListeners.clear();
    this.focusEventListeners.focus.clear();
    this.focusEventListeners.blur.clear();
    this.resizeEventListeners.clear();
    this.elementEventListeners.clear();
    for (const observer of this.intersectionObservers.values()) {
      observer[stopIntersectionObserver]();
    }
    this.intersectionObservers.clear();
    this.pendingIntersectionTargets.clear();
    this.elements.clear();
    this.parents.clear();
    this.children.clear();
    this.textControls.clear();
    this.scrollMetrics.clear();
    this.hoveredElement = undefined;
    this.rootElement = undefined;
    livePaintCannons.delete(this);
    process.off("SIGCONT", this.handleSigcont);
    this.binding.stop();
  }

  releaseForProcessExit(): void {
    if (this.stopped) {
      return;
    }

    this.stopped = true;
    if (this.animationFrameTimer !== undefined) {
      clearTimeout(this.animationFrameTimer);
      this.animationFrameTimer = undefined;
    }
    livePaintCannons.delete(this);
    process.off("SIGCONT", this.handleSigcont);
    this.binding.stop();
  }

  private setParent(child: PaintNodeBase, parent: PaintElement): void {
    const previousParent = this.parents.get(child.id);
    if (previousParent !== undefined && previousParent !== parent) {
      this.children.get(previousParent.id)?.delete(child.id);
    }
    this.parents.set(child.id, parent);
    let children = this.children.get(parent.id);
    if (children === undefined) {
      children = new Set();
      this.children.set(parent.id, children);
    }
    children.add(child.id);
  }

  getScrollLeft(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.scrollLeft ?? 0;
  }

  setScrollLeft(element: PaintElementBase, value: number): void {
    this.setScrollOffset(element, value, this.getScrollTop(element));
  }

  getScrollTop(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.scrollTop ?? 0;
  }

  setScrollTop(element: PaintElementBase, value: number): void {
    this.setScrollOffset(element, this.getScrollLeft(element), value);
  }

  getScrollWidth(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.scrollWidth ?? 0;
  }

  getScrollHeight(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.scrollHeight ?? 0;
  }

  getClientWidth(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.clientWidth ?? 0;
  }

  getClientHeight(element: PaintElementBase): number {
    return this.getScrollMetrics(element)?.clientHeight ?? 0;
  }

  private createNativeDiv(): number {
    if (!this.isTransactionActive()) {
      return this.binding.createDiv();
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createDiv", id });
    return id;
  }

  private createNativeSpan(): number {
    if (!this.isTransactionActive()) {
      return this.binding.createSpan();
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createSpan", id });
    return id;
  }

  private createNativeImage(): number {
    if (!this.isTransactionActive()) {
      return this.binding.createImage();
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createImage", id });
    return id;
  }

  private createNativeInput(): number {
    if (!this.isTransactionActive()) {
      return this.binding.createInput();
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createInput", id });
    return id;
  }

  private createNativeTextArea(): number {
    if (!this.isTransactionActive()) {
      return this.binding.createTextArea();
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createTextArea", id });
    return id;
  }

  private createNativeTextNode(text: string): number {
    if (!this.isTransactionActive()) {
      return this.binding.createTextNode(text);
    }

    const id = this.allocateTemporaryId();
    this.batchCommands.push({ type: "createText", id, text });
    return id;
  }

  private setNativeTextNodeValue(id: number, text: string): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setText", id, text });
      return;
    }

    this.binding.setTextNodeValue(id, text);
  }

  private setNativeImageSource(id: number, src: string): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setImageSource", id, src });
      return;
    }

    this.binding.setImageSource(id, src);
  }

  private setNativeInputValue(id: number, value: string, cursor: number): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setInputValue", id, value, cursor });
      return;
    }

    this.binding.setInputValue(id, value, cursor);
  }

  private setNativeInputFocused(id: number, focused: boolean): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setInputFocused", id, focused });
      return;
    }

    this.binding.setInputFocused(id, focused);
  }

  private setNativeInputPlaceholder(id: number, placeholder: string): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setInputPlaceholder", id, placeholder });
      return;
    }

    this.binding.setInputPlaceholder(id, placeholder);
  }

  private setNativeTextAreaValue(id: number, value: string, cursor: number): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setTextAreaValue", id, value, cursor });
      return;
    }

    this.binding.setTextAreaValue(id, value, cursor);
  }

  private setNativeTextAreaFocused(id: number, focused: boolean): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setTextAreaFocused", id, focused });
      return;
    }

    this.binding.setTextAreaFocused(id, focused);
  }

  private setNativeTextAreaPlaceholder(id: number, placeholder: string): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setTextAreaPlaceholder", id, placeholder });
      return;
    }

    this.binding.setTextAreaPlaceholder(id, placeholder);
  }

  private setNativeRoot(id: number): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setRoot", id });
      return;
    }

    this.binding.setRoot(id);
  }

  private appendNativeChild(parent: PaintElement, child: PaintNodeBase): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "appendChild", parent: parent.id, child: child.id });
    } else {
      this.binding.appendChild(parent.id, child.id);
    }

    this.setParent(child, parent);
  }

  private insertNativeChildBefore(
    parent: PaintElement,
    child: PaintNodeBase,
    before: PaintNodeBase,
  ): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({
        type: "insertChildBefore",
        parent: parent.id,
        child: child.id,
        before: before.id,
      });
    } else {
      this.binding.insertChildBefore(parent.id, child.id, before.id);
    }

    this.setParent(child, parent);
  }

  detachChild<T extends PaintNodeBase>(parent: PaintElement, child: T): T {
    if (this.parents.get(child.id) !== parent) {
      throw new Error("node is not a child of this parent");
    }

    this.detachNativeNode(child.id);
    this.parents.delete(child.id);
    this.children.get(parent.id)?.delete(child.id);
    this.clearDetachedState(child);
    return child;
  }

  detachNode(node: PaintNodeBase): void {
    if (!this.parents.has(node.id) && !isPaintElement(node)) {
      return;
    }

    this.detachNativeNode(node.id);
    const parent = this.parents.get(node.id);
    this.parents.delete(node.id);
    if (parent !== undefined) {
      this.children.get(parent.id)?.delete(node.id);
    }
    this.clearDetachedState(node);
  }

  destroyNode(node: PaintNodeBase): void {
    this.cleanupDestroyedNode(node);
    this.destroyNativeNode(node.id);
  }

  private detachNativeNode(id: number): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "detachNode", id });
      return;
    }

    this.binding.detachNode(id);
  }

  private destroyNativeNode(id: number): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "destroyNode", id });
      return;
    }

    this.binding.destroyNode(id);
  }

  private clearDetachedState(node: PaintNodeBase): void {
    const ids = this.collectSubtreeIds(node.id);

    if (this.focusedElement !== undefined && ids.has(this.focusedElement.id)) {
      this.blurElement(this.focusedElement);
    }
    if (this.hoveredElement !== undefined && ids.has(this.hoveredElement.id)) {
      this.hoveredElement = undefined;
    }
    if (this.rootElement !== undefined && ids.has(this.rootElement.id)) {
      this.rootElement = undefined;
    }
  }

  private cleanupDestroyedNode(node: PaintNodeBase): void {
    const ids = this.collectSubtreeIds(node.id);

    const parent = this.parents.get(node.id);
    if (parent !== undefined && !ids.has(parent.id)) {
      this.children.get(parent.id)?.delete(node.id);
    }

    for (const id of ids) {
      this.parents.delete(id);
      this.children.delete(id);
      const element = this.elements.get(id);
      if (isTextControl(element)) {
        this.textControls.delete(element);
      }
      this.elements.delete(id);
      this.elementEventListeners.delete(id);
      this.scrollMetrics.delete(id);
      this.batchNodes.delete(id);
      if (element !== undefined) {
        for (const [observerId, observer] of this.intersectionObservers) {
          if (observer[removeIntersectionTarget](element)) {
            this.binding.unobserveIntersection(observerId, id);
          }
        }
      }
    }

    if (this.focusedElement !== undefined && ids.has(this.focusedElement.id)) {
      this.blurElement(this.focusedElement);
    }
    if (this.hoveredElement !== undefined && ids.has(this.hoveredElement.id)) {
      this.hoveredElement = undefined;
    }
    if (this.rootElement !== undefined && ids.has(this.rootElement.id)) {
      this.rootElement = undefined;
    }
  }

  private collectSubtreeIds(id: number): Set<number> {
    const ids = new Set<number>();
    const pending = [id];
    while (pending.length > 0) {
      const currentId = pending.pop();
      if (currentId === undefined || ids.has(currentId)) {
        continue;
      }
      ids.add(currentId);
      const children = this.children.get(currentId);
      if (children !== undefined) {
        pending.push(...children);
      }
    }
    return ids;
  }

  private setNativeStyleProperty(id: number, property: string, value: string): void {
    if (this.isTransactionActive()) {
      this.batchCommands.push({ type: "setStyleProperty", id, property, value });
      return;
    }

    this.binding.setStyleProperty(id, property, value);
  }

  private isTransactionActive(): boolean {
    return this.transactionDepth > 0;
  }

  private allocateTemporaryId(): number {
    const id = this.nextTemporaryId;
    this.nextTemporaryId -= 1;
    return id;
  }

  private registerBatchNode(node: PaintNodeBase): void {
    if (node.id < 0) {
      this.batchNodes.set(node.id, node);
    }
  }

  private flushTransaction(): void {
    const commands = this.batchCommands;
    this.batchCommands = [];

    try {
      if (commands.length > 0) {
        const mappings = this.binding.applyBatch(commands);
        this.applyBatchIdMappings(mappings);
      }
      this.flushPendingIntersectionTargets();
    } finally {
      this.batchNodes.clear();
      this.pendingIntersectionTargets.clear();
    }
  }

  private rollbackTransaction(): void {
    this.batchCommands = [];
    this.batchNodes.clear();
    this.pendingIntersectionTargets.clear();
  }

  private flushPendingIntersectionTargets(): void {
    const pending = Array.from(this.pendingIntersectionTargets);
    this.pendingIntersectionTargets.clear();
    for (const [observer, targets] of pending) {
      for (const target of targets) {
        observer[syncIntersectionTarget](target);
      }
    }
  }

  private applyBatchIdMappings(mappings: NativeBatchIdMapping[]): void {
    if (mappings.length === 0) {
      return;
    }

    const ids = new Map(mappings.map(mapping => [mapping.temporaryId, mapping.id]));
    for (const [temporaryId, node] of this.batchNodes) {
      const id = ids.get(temporaryId);
      if (id !== undefined) {
        node.id = id;
      }
    }

    this.rekeyElementMap(ids);
    this.rekeyParentMap(ids);
    this.rekeyChildrenMap(ids);
    this.rekeyElementEventListeners(ids);
    this.rekeyScrollMetrics(ids);
  }

  private rekeyElementMap(ids: Map<number, number>): void {
    for (const temporaryId of ids.keys()) {
      this.elements.delete(temporaryId);
    }
    for (const element of Array.from(this.elements.values())) {
      this.elements.set(element.id, element);
    }
    for (const node of this.batchNodes.values()) {
      if (isPaintElement(node)) {
        this.elements.set(node.id, node);
      }
    }
  }

  private rekeyParentMap(ids: Map<number, number>): void {
    if (this.parents.size === 0) {
      return;
    }

    const entries = Array.from(this.parents.entries());
    this.parents.clear();
    for (const [childId, parent] of entries) {
      this.parents.set(ids.get(childId) ?? childId, parent);
    }
  }

  private rekeyChildrenMap(ids: Map<number, number>): void {
    if (this.children.size === 0) {
      return;
    }

    const entries = Array.from(this.children.entries());
    this.children.clear();
    for (const [parentId, children] of entries) {
      this.children.set(
        ids.get(parentId) ?? parentId,
        new Set(Array.from(children, childId => ids.get(childId) ?? childId)),
      );
    }
  }

  private rekeyElementEventListeners(ids: Map<number, number>): void {
    if (this.elementEventListeners.size === 0) {
      return;
    }

    const entries = Array.from(this.elementEventListeners.entries());
    this.elementEventListeners.clear();
    for (const [id, listeners] of entries) {
      this.elementEventListeners.set(ids.get(id) ?? id, listeners);
    }
  }

  private rekeyScrollMetrics(ids: Map<number, number>): void {
    if (this.scrollMetrics.size === 0) {
      return;
    }

    const entries = Array.from(this.scrollMetrics.entries());
    this.scrollMetrics.clear();
    for (const [id, metrics] of entries) {
      this.scrollMetrics.set(ids.get(id) ?? id, metrics);
    }
  }

  private scheduleAnimationFrameTick(): void {
    if (this.animationFrameTimer !== undefined || this.animationFrameCallbacks.size === 0) {
      return;
    }

    this.animationFrameTimer = setTimeout(() => {
      this.animationFrameTimer = undefined;
      this.runAnimationFrameTick();
    }, this.animationFrameIntervalMs);
  }

  private runAnimationFrameTick(): void {
    if (this.binding.interruptedByCtrlC) {
      this.stop();
      return;
    }

    const callbacks = Array.from(this.animationFrameCallbacks.values());
    this.animationFrameCallbacks.clear();

    const timestamp = performance.now();
    for (const callback of callbacks) {
      try {
        callback(timestamp);
      } catch (error) {
        if (this.stopAfterCtrlCRendererError(error)) {
          return;
        }
        throw error;
      }
    }

    if (!this.stopped) {
      this.scheduleAnimationFrameTick();
    }
  }

  private stopAfterCtrlCRendererError(error: unknown): boolean {
    if (
      !(error instanceof Error) ||
      error.message !== "renderer thread stopped" ||
      !this.binding.interruptedByCtrlC
    ) {
      return false;
    }

    this.stop();
    return true;
  }

  private handleNativeEventNotification(): void {
    if (this.stopped) {
      return;
    }

    const events: NativeQueuedEvent[] = this.binding.drainEvents();
    for (const nativeEvent of events) {
      switch (nativeEvent.kind) {
        case "keyboard": {
          const input = nativeEvent.keyboard;
          if (input === undefined) throw new Error("native keyboard event is missing its payload");
          const event = new PaintKeyboardEvent(input, this.keyboardEventTarget());
          if (!this.handleDefaultControlEvent(event)) {
            this.dispatchKeyboardInputEvent(event);
          }
          break;
        }
        case "paste": {
          if (nativeEvent.paste === undefined) {
            throw new Error("native paste event is missing its payload");
          }
          this.dispatchPasteInputEvent(
            new PaintClipboardEvent(nativeEvent.paste, this.keyboardEventTarget()),
          );
          break;
        }
        case "clipboardWrite": {
          if (nativeEvent.clipboardWrite === undefined) {
            throw new Error("native clipboardWrite event is missing its payload");
          }
          const event = new PaintClipboardWriteEvent(
            nativeEvent.clipboardWrite.text,
            nativeEvent.clipboardWrite.success,
          );
          for (const listener of Array.from(this.clipboardWriteEventListeners)) {
            listener(event);
          }
          break;
        }
        case "resize": {
          if (nativeEvent.resize === undefined) {
            throw new Error("native resize event is missing its payload");
          }
          this.dispatchResizeEvent(nativeEvent.resize);
          break;
        }
        case "focus": {
          if (nativeEvent.focus === undefined) {
            throw new Error("native focus event is missing its payload");
          }
          const event = new PaintCannonFocusEvent(nativeEvent.focus, this);
          const listeners = Array.from(this.focusEventListeners[event.type]);
          for (const listener of listeners) {
            listener(event);
          }
          break;
        }
        case "transition": {
          if (nativeEvent.transition === undefined) {
            throw new Error("native transition event is missing its payload");
          }
          this.dispatchTransitionEvent(nativeEvent.transition);
          break;
        }
        case "intersection": {
          if (nativeEvent.intersection === undefined) {
            throw new Error("native intersection event is missing its payload");
          }
          this.dispatchIntersectionEvent(nativeEvent.intersection);
          break;
        }
        case "scroll": {
          const input = nativeEvent.scroll;
          if (input === undefined) throw new Error("native scroll event is missing its payload");
          const target = this.elements.get(input.targetId);
          if (target === undefined) break;
          // Layout or explicit scrolling may have superseded the queued adjustment.
          const metrics = this.binding.scrollMetrics(target.id);
          if (metrics === null) break;
          this.scrollMetrics.set(target.id, metrics);
          this.dispatchScrollEvent(target, metrics, 0, 0);
          break;
        }
        case "mouse": {
          if (nativeEvent.mouse === undefined) {
            throw new Error("native mouse event is missing its payload");
          }
          if (this.captureMouse) {
            this.handleTerminalMouseEvent(nativeEvent.mouse);
          }
          break;
        }
        default:
          throw new Error(`unknown native event kind: ${nativeEvent.kind}`);
      }
    }
  }

  private dispatchIntersectionEvent(event: NativeIntersectionEvent): void {
    const observer = this.intersectionObservers.get(event.observerId);
    if (observer === undefined) {
      return;
    }
    const entries = event.entries.flatMap(entry => {
      const target = this.elements.get(entry.targetId);
      return target === undefined
        ? []
        : [
            new IntersectionObserverEntry({
              time: entry.time,
              rootBounds: domRect(entry.rootBounds),
              boundingClientRect: domRect(entry.boundingClientRect),
              intersectionRect: domRect(entry.intersectionRect),
              isIntersecting: entry.isIntersecting,
              intersectionRatio: entry.intersectionRatio,
              target,
            }),
          ];
    });
    if (entries.length > 0) {
      observer[enqueueIntersectionEntries](entries);
    }
  }

  private dispatchKeyboardInputEvent(event: PaintKeyboardEvent): void {
    if (event.target !== undefined) {
      this.dispatchKeyboardEvent(event.target, event);
    }

    if (!event.propagationStopped) {
      for (const listener of Array.from(this.keyboardEventListeners[event.type])) {
        listener(event);
      }
    }

    if (event.defaultPrevented) {
      return;
    }

    const changedInput = this.focusedTextControl;
    const beforeValue = changedInput?.value;
    if (
      this.handleDefaultInputEvent(event) &&
      changedInput !== undefined &&
      beforeValue !== undefined &&
      changedInput.value !== beforeValue
    ) {
      this.dispatchChangeEvent(changedInput);
    }
  }

  private dispatchPasteInputEvent(event: PaintClipboardEvent): void {
    if (event.target !== undefined) {
      this.dispatchClipboardEvent(event.target, event);
    }

    if (!event.propagationStopped) {
      for (const listener of Array.from(this.clipboardEventListeners)) {
        listener(event);
      }
    }

    if (event.defaultPrevented || this.focusedTextControl === undefined) {
      return;
    }

    const input = this.focusedTextControl;
    const beforeValue = input.value;
    input.insertText(event.clipboardData.getData("text/plain"));
    if (input.value !== beforeValue) {
      this.dispatchChangeEvent(input);
    }
  }

  private handleDefaultControlEvent(event: KeyboardEvent): boolean {
    if (event.type !== "keydown" || !event.ctrlKey) {
      return false;
    }

    if (!this.captureCtrlZ && event.code === "KeyZ") {
      this.binding.releaseTerminal();
      this.suspendedByPaintCannon = true;
      try {
        this.binding.suspendProcessGroup();
      } catch (error) {
        this.suspendedByPaintCannon = false;
        this.binding.captureTerminal();
        throw error;
      }
      return true;
    }

    return false;
  }

  private handleDefaultInputEvent(event: KeyboardEvent): boolean {
    if (event.type !== "keydown") {
      return false;
    }

    if (!event.ctrlKey && !event.altKey && !event.metaKey && event.key === "Tab") {
      return this.focusNextInput(event.shiftKey ? -1 : 1);
    }

    const input = this.focusedTextControl;
    if (input === undefined || event.altKey || event.metaKey) {
      return false;
    }

    if (event.ctrlKey) {
      return this.handleInputControlKey(input, event);
    }

    switch (event.key) {
      case "Backspace":
        return input.deleteBackward();
      case "Delete":
        return input.deleteForward();
      case "ArrowLeft":
        input.moveCursorHorizontally(-1);
        return true;
      case "ArrowRight":
        input.moveCursorHorizontally(1);
        return true;
      case "ArrowUp":
        if (input instanceof TextAreaElement) {
          return input.moveCursorVertically(-1);
        }
        return false;
      case "ArrowDown":
        if (input instanceof TextAreaElement) {
          return input.moveCursorVertically(1);
        }
        return false;
      case "Home":
        if (input instanceof TextAreaElement) {
          moveTextAreaCursorToLineStart(input);
        } else {
          input.cursorToStart();
        }
        return true;
      case "End":
        if (input instanceof TextAreaElement) {
          moveTextAreaCursorToLineEnd(input);
        } else {
          input.cursorToEnd();
        }
        return true;
      case "Enter":
        if (input instanceof TextAreaElement) {
          input.insertText("\n");
          return true;
        }
        return this.submitInputForm(input);
      default:
        if (Array.from(event.key).length === 1) {
          input.insertText(event.key);
          return true;
        }
        return false;
    }
  }

  private handleInputControlKey(input: TextControlElement, event: KeyboardEvent): boolean {
    switch (event.code) {
      case "KeyA":
        if (input instanceof TextAreaElement) {
          moveTextAreaCursorToLineStart(input);
        } else {
          input.cursorToStart();
        }
        return true;
      case "KeyE":
        if (input instanceof TextAreaElement) {
          moveTextAreaCursorToLineEnd(input);
        } else {
          input.cursorToEnd();
        }
        return true;
      case "KeyB":
        input.moveCursorHorizontally(-1);
        return true;
      case "KeyF":
        input.moveCursorHorizontally(1);
        return true;
      case "KeyD":
        return input.deleteForward();
      case "KeyH":
        return input.deleteBackward();
      case "KeyK":
        return input.deleteToEnd();
      case "KeyU":
        return input.deleteToStart();
      case "KeyW":
        return input.deletePreviousWord();
      default:
        return false;
    }
  }

  focusInput(element: TextControlElement): void {
    this.focusElement(element);
  }

  blurInput(element: TextControlElement): void {
    this.blurElement(element);
  }

  private isHiddenFromFocus(element: PaintElement): boolean {
    let current: PaintElement | undefined = element;
    while (current !== undefined) {
      if (current.style.display === "none" || current.style.visibility === "hidden") return true;
      current = this.parents.get(current.id);
    }
    return false;
  }

  private isConnectedToRoot(element: PaintElement): boolean {
    let current: PaintElement | undefined = element;
    while (current !== undefined) {
      if (current === this.rootElement) return true;
      current = this.parents.get(current.id);
    }
    return false;
  }

  focusElement(element: PaintElement): void {
    if (
      this.elements.get(element.id) !== element ||
      !this.isConnectedToRoot(element) ||
      this.focusedElement === element ||
      this.isHiddenFromFocus(element)
    ) {
      return;
    }

    if (this.focusedElement !== undefined) {
      this.blurElement(this.focusedElement);
      // A blur handler may explicitly move focus elsewhere.
      if (this.focusedElement !== undefined) {
        return;
      }
    }
    if (
      this.elements.get(element.id) !== element ||
      !this.isConnectedToRoot(element) ||
      this.isHiddenFromFocus(element)
    ) {
      return;
    }
    this.focusedElement = element;
    if (isTextControl(element)) {
      element.setFocused(true);
    }
    this.dispatchFocusEvent("focus", element);
  }

  blurElement(element: PaintElement): void {
    if (this.focusedElement !== element) {
      return;
    }

    this.focusedElement = undefined;
    if (isTextControl(element)) {
      element.setFocused(false);
    }
    this.dispatchFocusEvent("blur", element);
  }

  private focusNextInput(direction: 1 | -1): boolean {
    if (this.textControls.size === 0) {
      return false;
    }

    const textControls = Array.from(this.textControls);
    const currentIndex =
      this.focusedTextControl === undefined ? -1 : textControls.indexOf(this.focusedTextControl);
    const start = currentIndex < 0 ? (direction === 1 ? -1 : 0) : currentIndex;
    const nextIndex = (start + direction + textControls.length) % textControls.length;
    this.focusInput(textControls[nextIndex]);
    return true;
  }

  private registerElement(element: PaintElement): void {
    this.elements.set(element.id, element);
    this.registerBatchNode(element);
  }

  private getScrollMetrics(element: PaintElementBase): NativeScrollMetrics | undefined {
    const metrics = this.binding.scrollMetrics(element.id);
    if (metrics !== null) {
      this.scrollMetrics.set(element.id, metrics);
      return metrics;
    }
    return this.scrollMetrics.get(element.id);
  }

  private setScrollOffset(
    element: PaintElementBase,
    left: number,
    top: number,
  ): NativeScrollMetrics | null {
    const nextLeft = normalizeScrollOffset(left);
    const nextTop = normalizeScrollOffset(top);
    const before = this.getScrollMetrics(element);
    const metrics = this.binding.setScrollOffset(element.id, nextLeft, nextTop);
    if (metrics === null) {
      return null;
    }

    this.scrollMetrics.set(element.id, metrics);
    if (
      before !== undefined &&
      before.scrollLeft === metrics.scrollLeft &&
      before.scrollTop === metrics.scrollTop &&
      before.scrollWidth === metrics.scrollWidth &&
      before.scrollHeight === metrics.scrollHeight &&
      before.clientWidth === metrics.clientWidth &&
      before.clientHeight === metrics.clientHeight
    ) {
      return null;
    }

    return metrics;
  }

  private handleTerminalMouseEvent(input: TerminalMouseEvent): void {
    if (input.type === "wheel") {
      this.handleWheelEvent(input);
      return;
    }

    if (input.type === "mousedown" && this.handleScrollbarMouseDown(input)) {
      return;
    }

    if (input.type === "mousedrag" && this.handleScrollbarMouseDrag(input)) {
      return;
    }

    if (input.type === "mouseup" && this.handleScrollbarMouseUp()) {
      return;
    }

    if (input.type === "click" && this.suppressNextScrollbarClick) {
      this.suppressNextScrollbarClick = false;
      return;
    }

    const hasMouseEnter = this.hasElementEventListeners("mouseenter");
    const hasMouseLeave = this.hasElementEventListeners("mouseleave");
    const hasMouseMove = this.hasElementEventListeners("mousemove");
    const hasClick = this.hasElementEventListeners("click");

    if (input.type === "mousemove" && !hasMouseEnter && !hasMouseLeave && !hasMouseMove) {
      return;
    }

    const targetId = this.binding.targetIdForPoint(input.x, input.y);
    const target = targetId === null ? undefined : this.elements.get(targetId);

    if (
      input.type === "click" &&
      !hasClick &&
      !isTextControl(target) &&
      !(target instanceof ButtonElement)
    ) {
      return;
    }

    if (input.type === "mousemove") {
      if (hasMouseEnter || hasMouseLeave) {
        this.dispatchHoverBoundaryEvents(target, input);
      }
      if (target !== undefined && hasMouseMove) {
        this.dispatchMouseEvent("mousemove", target, input, true);
      }
      return;
    }

    if (input.type === "click" && target !== undefined) {
      if (isTextControl(target)) {
        this.focusInput(target);
        target.setCursorPositionFromNativePoint(input.x, input.y);
      }
      if (hasClick) {
        const event = this.dispatchMouseEvent("click", target, input, true);
        if (target instanceof ButtonElement && !event.defaultPrevented) {
          this.submitButtonForm(target);
        }
      } else if (target instanceof ButtonElement) {
        this.submitButtonForm(target);
      }
    }
  }

  private handleScrollbarMouseDown(input: TerminalMouseEvent): boolean {
    if (input.button !== 0) {
      return false;
    }

    const hit: NativeScrollbarHit | null = this.binding.scrollbarHitForPoint(input.x, input.y);
    if (hit === null) {
      return false;
    }

    const axis = parseScrollbarAxis(hit.axis);
    this.suppressNextScrollbarClick = true;
    const target = this.elements.get(hit.targetId);
    if (target === undefined) {
      return true;
    }

    const coordinate = scrollbarCoordinate(input, axis);
    const thumbEnd = hit.thumbStart + hit.thumbLength;
    if (coordinate >= hit.thumbStart && coordinate < thumbEnd) {
      this.scrollbarDrag = {
        target,
        axis,
        dragOffset: coordinate - hit.thumbStart,
        railStart: hit.railStart,
        railLength: hit.railLength,
        thumbLength: hit.thumbLength,
        maxScroll: hit.maxScroll,
      };
      return true;
    }

    const direction = coordinate < hit.thumbStart ? -1 : 1;
    const nextOffset = hit.scrollOffset + direction * hit.clientLength;
    const metrics = this.setScrollbarAxisOffset(target, axis, nextOffset);
    if (metrics !== null) {
      this.dispatchScrollEvent(target, metrics, input.deltaX, input.deltaY);
    }
    return true;
  }

  private handleScrollbarMouseDrag(input: TerminalMouseEvent): boolean {
    const drag = this.scrollbarDrag;
    if (drag === undefined) {
      return false;
    }

    const movableLength = Math.max(0, drag.railLength - drag.thumbLength);
    const nextThumbStart =
      movableLength === 0
        ? drag.railStart
        : clamp(
            scrollbarCoordinate(input, drag.axis) - drag.dragOffset,
            drag.railStart,
            drag.railStart + movableLength,
          );
    const nextOffset =
      movableLength === 0
        ? 0
        : Math.round(((nextThumbStart - drag.railStart) / movableLength) * drag.maxScroll);
    const metrics = this.setScrollbarAxisOffset(drag.target, drag.axis, nextOffset);
    if (metrics !== null) {
      this.dispatchScrollEvent(drag.target, metrics, input.deltaX, input.deltaY);
    }
    return true;
  }

  private handleScrollbarMouseUp(): boolean {
    if (this.scrollbarDrag === undefined) {
      return false;
    }

    this.scrollbarDrag = undefined;
    this.suppressNextScrollbarClick = true;
    return true;
  }

  private setScrollbarAxisOffset(
    target: PaintElement,
    axis: ScrollbarAxis,
    offset: number,
  ): NativeScrollMetrics | null {
    if (axis === "x") {
      return this.setScrollOffset(target, offset, this.getScrollTop(target));
    }

    return this.setScrollOffset(target, this.getScrollLeft(target), offset);
  }

  private handleWheelEvent(input: TerminalMouseEvent): boolean {
    const targetId = this.binding.targetIdForPoint(input.x, input.y);
    const target = targetId === null ? undefined : this.elements.get(targetId);
    if (target === undefined) {
      return false;
    }

    const scrollTarget = this.findScrollableAncestor(target, input.deltaX, input.deltaY);
    if (scrollTarget === undefined) {
      return false;
    }

    const current = this.getScrollMetrics(scrollTarget) ?? emptyScrollMetrics();
    const canScrollX = this.isElementAxisScrollable(scrollTarget, "x");
    const canScrollY = this.isElementAxisScrollable(scrollTarget, "y");
    const nextLeft = canScrollX ? current.scrollLeft + input.deltaX * 4 : current.scrollLeft;
    const nextTop = canScrollY ? current.scrollTop + input.deltaY * 3 : current.scrollTop;

    const metrics = this.setScrollOffset(scrollTarget, nextLeft, nextTop);
    if (metrics === null) {
      return false;
    }

    this.dispatchScrollEvent(scrollTarget, metrics, input.deltaX, input.deltaY);
    return true;
  }

  private findScrollableAncestor(
    target: PaintElement,
    deltaX: number,
    deltaY: number,
  ): PaintElement | undefined {
    for (const element of this.elementPath(target)) {
      const scrollsX = deltaX !== 0 && this.isElementAxisScrollable(element, "x");
      const scrollsY = deltaY !== 0 && this.isElementAxisScrollable(element, "y");
      if (!scrollsX && !scrollsY) {
        continue;
      }
      const metrics = this.getScrollMetrics(element);
      if (metrics === undefined) {
        continue;
      }
      const canScrollX = scrollsX && canScrollAxisInDirection("x", deltaX, metrics);
      const canScrollY = scrollsY && canScrollAxisInDirection("y", deltaY, metrics);
      if (canScrollX || canScrollY) {
        return element;
      }
    }
    return undefined;
  }

  private isElementAxisScrollable(element: PaintElement, axis: ScrollbarAxis): boolean {
    return element === this.viewportElement || isElementAxisScrollable(element, axis);
  }

  private dispatchHoverBoundaryEvents(
    nextHoveredElement: PaintElement | undefined,
    input: TerminalMouseEvent,
  ): void {
    if (nextHoveredElement === this.hoveredElement) {
      return;
    }

    const previousPath = this.elementPath(this.hoveredElement);
    const nextPath = this.elementPath(nextHoveredElement);
    const nextIds = new Set(nextPath.map(element => element.id));
    const previousIds = new Set(previousPath.map(element => element.id));

    for (const element of previousPath) {
      if (!nextIds.has(element.id)) {
        this.dispatchMouseEvent("mouseleave", element, input, false);
      }
    }

    for (const element of nextPath.slice().reverse()) {
      if (!previousIds.has(element.id)) {
        this.dispatchMouseEvent("mouseenter", element, input, false);
      }
    }

    this.hoveredElement = nextHoveredElement;
  }

  private hasElementEventListeners(type: ElementEventType): boolean {
    for (const eventListeners of this.elementEventListeners.values()) {
      if ((eventListeners[type]?.size ?? 0) > 0) {
        return true;
      }
    }
    return false;
  }

  private keyboardEventTarget(): PaintElement | undefined {
    if (this.focusedElement !== undefined) {
      return this.focusedElement;
    }
    if (this.rootElement === undefined) {
      return undefined;
    }
    return this.firstElementChild(this.rootElement) ?? this.rootElement;
  }

  private firstElementChild(parent: PaintElement): PaintElement | undefined {
    for (const [childId, childParent] of this.parents) {
      if (childParent.id === parent.id) {
        return this.elements.get(childId);
      }
    }
    return undefined;
  }

  private dispatchKeyboardEvent(target: PaintElement, event: PaintKeyboardEvent): void {
    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(
        this.elementEventListeners.get(currentTarget.id)?.[event.type] ?? [],
      );
      for (const listener of listeners) {
        (listener as KeyboardEventListener)(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }

  private dispatchClipboardEvent(target: PaintElement, event: PaintClipboardEvent): void {
    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(
        this.elementEventListeners.get(currentTarget.id)?.[event.type] ?? [],
      );
      for (const listener of listeners) {
        (listener as ClipboardEventListener)(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }

  private submitInputForm(input: InputElement): boolean {
    const form = this.findFormAncestor(input);
    if (form === undefined) {
      return false;
    }

    this.dispatchSubmitEvent(form, input);
    return true;
  }

  private submitButtonForm(button: ButtonElement): boolean {
    if (button.type !== "submit") {
      return false;
    }

    const form = this.findFormAncestor(button);
    if (form === undefined) {
      return false;
    }

    this.dispatchSubmitEvent(form, button);
    return true;
  }

  private findFormAncestor(element: PaintElement): FormElement | undefined {
    let current: PaintElement | undefined = element;
    while (current !== undefined) {
      if (current instanceof FormElement) {
        return current;
      }
      current = this.parents.get(current.id);
    }
    return undefined;
  }

  private dispatchMouseEvent(
    type: ElementEventType,
    target: PaintElement,
    input: TerminalMouseEvent,
    bubbles: boolean,
  ): PaintMouseEvent {
    const event = new PaintMouseEvent({
      type,
      target,
      clientX: input.x,
      clientY: input.y,
      button: input.button,
      ctrlKey: input.ctrlKey,
      altKey: input.altKey,
      metaKey: input.metaKey,
      shiftKey: input.shiftKey,
    });

    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(this.elementEventListeners.get(currentTarget.id)?.[type] ?? []);
      for (const listener of listeners) {
        (listener as MouseEventListener)(event);
        if (event.propagationStopped) {
          return event;
        }
      }
      if (!bubbles) {
        return event;
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
    return event;
  }

  private dispatchFocusEvent(type: FocusElementEventType, target: PaintElement): void {
    const event = new PaintFocusEvent({ type, target });
    event.setCurrentTarget(target);
    const listeners = Array.from(this.elementEventListeners.get(target.id)?.[type] ?? []);
    for (const listener of listeners) {
      (listener as FocusEventListener)(event);
      if (event.propagationStopped) {
        return;
      }
    }
  }

  private dispatchSubmitEvent(target: FormElement, submitter: InputElement | ButtonElement): void {
    const event = new PaintSubmitEvent({ target, submitter });
    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(this.elementEventListeners.get(currentTarget.id)?.submit ?? []);
      for (const listener of listeners) {
        (listener as SubmitEventListener)(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }

  private dispatchChangeEvent(target: TextControlElement): void {
    const event = new PaintChangeEvent({ target });
    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(this.elementEventListeners.get(currentTarget.id)?.change ?? []);
      for (const listener of listeners) {
        (listener as ChangeEventListener)(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }

  private dispatchScrollEvent(
    target: PaintElement,
    metrics: NativeScrollMetrics,
    deltaX: number,
    deltaY: number,
  ): void {
    const event = new PaintScrollEvent({
      target,
      scrollLeft: metrics.scrollLeft,
      scrollTop: metrics.scrollTop,
      scrollWidth: metrics.scrollWidth,
      scrollHeight: metrics.scrollHeight,
      deltaX,
      deltaY,
    });
    event.setCurrentTarget(target);
    const listeners = Array.from(this.elementEventListeners.get(target.id)?.scroll ?? []);
    for (const listener of listeners) {
      (listener as ScrollEventListener)(event);
      if (event.propagationStopped) {
        return;
      }
    }
  }

  private dispatchResizeEvent(input: TerminalResizeEvent): void {
    const listeners = Array.from(this.resizeEventListeners);
    if (listeners.length === 0) {
      return;
    }

    const event = new PaintResizeEvent(input.cols, input.rows);
    for (const listener of listeners) {
      listener(event);
    }
  }

  private dispatchTransitionEvent(nativeEvent: NativeTransitionEvent): void {
    if (!isTransitionElementEventType(nativeEvent.type)) {
      return;
    }

    const target = this.elements.get(nativeEvent.targetId);
    if (target === undefined) {
      return;
    }

    const type = nativeEvent.type;
    const event = new PaintTransitionEvent({
      type,
      target,
      propertyName: nativeEvent.propertyName,
    });

    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(this.elementEventListeners.get(currentTarget.id)?.[type] ?? []);
      for (const listener of listeners) {
        (listener as TransitionEventListener)(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }

  private elementPath(element: PaintElement | undefined): PaintElement[] {
    const path: PaintElement[] = [];
    let current = element;
    while (current !== undefined) {
      path.push(current);
      current = this.parents.get(current.id);
    }
    return path;
  }
}

interface PaintMouseEventInit {
  type: ElementEventType;
  target: PaintElement;
  clientX: number;
  clientY: number;
  button: number;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

interface PaintFocusEventInit {
  type: FocusElementEventType;
  target: PaintElement;
}

interface PaintSubmitEventInit {
  target: FormElement;
  submitter: InputElement | ButtonElement;
}

interface PaintChangeEventInit {
  target: TextControlElement;
}

interface PaintScrollEventInit {
  target: PaintElement;
  scrollLeft: number;
  scrollTop: number;
  scrollWidth: number;
  scrollHeight: number;
  deltaX: number;
  deltaY: number;
}

interface PaintTransitionEventInit {
  type: TransitionElementEventType;
  target: PaintElement;
  propertyName: string;
}

export class PaintMouseEvent {
  readonly type: ElementEventType;
  readonly target: PaintElement;
  currentTarget: PaintElement;
  readonly clientX: number;
  readonly clientY: number;
  readonly button: number;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintMouseEventInit) {
    this.type = event.type;
    this.target = event.target;
    this.currentTarget = event.target;
    this.clientX = event.clientX;
    this.clientY = event.clientY;
    this.button = event.button;
    this.ctrlKey = event.ctrlKey;
    this.altKey = event.altKey;
    this.metaKey = event.metaKey;
    this.shiftKey = event.shiftKey;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export class PaintFocusEvent {
  readonly type: FocusElementEventType;
  readonly target: PaintElement;
  currentTarget: PaintElement;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintFocusEventInit) {
    this.type = event.type;
    this.target = event.target;
    this.currentTarget = event.target;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export class PaintSubmitEvent {
  readonly type: FormElementEventType = "submit";
  readonly target: FormElement;
  currentTarget: PaintElement;
  readonly submitter: InputElement | ButtonElement;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintSubmitEventInit) {
    this.target = event.target;
    this.currentTarget = event.target;
    this.submitter = event.submitter;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export class PaintChangeEvent {
  readonly type: ChangeElementEventType = "change";
  readonly target: TextControlElement;
  currentTarget: PaintElement;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintChangeEventInit) {
    this.target = event.target;
    this.currentTarget = event.target;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export class PaintScrollEvent {
  readonly type: "scroll" = "scroll";
  readonly target: PaintElement;
  currentTarget: PaintElement;
  readonly scrollLeft: number;
  readonly scrollTop: number;
  readonly scrollWidth: number;
  readonly scrollHeight: number;
  readonly deltaX: number;
  readonly deltaY: number;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintScrollEventInit) {
    this.target = event.target;
    this.currentTarget = event.target;
    this.scrollLeft = event.scrollLeft;
    this.scrollTop = event.scrollTop;
    this.scrollWidth = event.scrollWidth;
    this.scrollHeight = event.scrollHeight;
    this.deltaX = event.deltaX;
    this.deltaY = event.deltaY;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

export class PaintResizeEvent {
  readonly type: "resize" = "resize";
  readonly cols: number;
  readonly rows: number;

  constructor(cols: number, rows: number) {
    this.cols = cols;
    this.rows = rows;
  }
}

export class PaintCannonFocusEvent {
  readonly type: PaintCannonFocusEventType;
  readonly target: PaintCannon;
  readonly currentTarget: PaintCannon;
  readonly hasFocus: boolean;

  constructor(event: TerminalFocusEvent, target: PaintCannon) {
    this.type = event.type as PaintCannonFocusEventType;
    this.target = target;
    this.currentTarget = target;
    this.hasFocus = event.type === "focus";
  }
}

export class PaintTransitionEvent {
  readonly type: TransitionElementEventType;
  readonly target: PaintElement;
  currentTarget: PaintElement;
  readonly propertyName: string;
  defaultPrevented = false;
  propagationStopped = false;

  constructor(event: PaintTransitionEventInit) {
    this.type = event.type;
    this.target = event.target;
    this.currentTarget = event.target;
    this.propertyName = event.propertyName;
  }

  preventDefault(): void {
    this.defaultPrevented = true;
  }

  stopPropagation(): void {
    this.propagationStopped = true;
  }

  setCurrentTarget(element: PaintElement): void {
    this.currentTarget = element;
  }
}

type ElementEventTarget = {
  readonly id: number;
};

type SetNativeStyleProperty = (id: number, property: string, value: string) => void;

abstract class PaintNodeBase implements ElementEventTarget {
  readonly ownerDocument: PaintCannon;
  id: number;

  protected constructor(owner: PaintCannon, id: number) {
    this.ownerDocument = owner;
    this.id = id;
  }

  detach(): void {
    this.ownerDocument.detachNode(this);
  }

  destroy(): void {
    this.ownerDocument.destroyNode(this);
  }
}

abstract class PaintElementBase extends PaintNodeBase {
  readonly style: CSSStyleDeclaration;

  protected constructor(
    owner: PaintCannon,
    id: number,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(owner, id);
    this.style = new CSSStyleDeclaration(() => this.id, setNativeStyleProperty);
  }

  get scrollLeft(): number {
    return this.ownerDocument.getScrollLeft(this);
  }

  set scrollLeft(value: number) {
    this.ownerDocument.setScrollLeft(this, value);
  }

  get scrollTop(): number {
    return this.ownerDocument.getScrollTop(this);
  }

  set scrollTop(value: number) {
    this.ownerDocument.setScrollTop(this, value);
  }

  get scrollWidth(): number {
    return this.ownerDocument.getScrollWidth(this);
  }

  get scrollHeight(): number {
    return this.ownerDocument.getScrollHeight(this);
  }

  get clientWidth(): number {
    return this.ownerDocument.getClientWidth(this);
  }

  get clientHeight(): number {
    return this.ownerDocument.getClientHeight(this);
  }
}

abstract class PaintElementEventTarget<
  TEvents extends ElementEventListenerTuple,
> extends PaintElementBase {
  constructor(owner: PaintCannon, id: number, setNativeStyleProperty: SetNativeStyleProperty) {
    super(owner, id, setNativeStyleProperty);
  }

  focus(): void {
    if (isPaintElement(this)) {
      this.ownerDocument.focusElement(this);
    }
  }

  blur(): void {
    if (isPaintElement(this)) {
      this.ownerDocument.blurElement(this);
    }
  }

  addEventListener<TType extends ElementEventType>(
    type: TType,
    listener: EventListenerForTuple<TEvents, TType>,
  ): void {
    this.ownerDocument.addElementEventListener(this, type, listener);
  }

  removeEventListener<TType extends ElementEventType>(
    type: TType,
    listener: EventListenerForTuple<TEvents, TType>,
  ): void {
    this.ownerDocument.removeElementEventListener(this, type, listener);
  }
}

export class DivElement extends PaintElementEventTarget<ContainerElementEventListenerTuple> {
  constructor(
    owner: PaintCannon,
    id: number,
    private readonly appendNativeChild: (parent: PaintElement, child: PaintNode) => void,
    private readonly insertNativeChildBefore: (
      parent: PaintElement,
      child: PaintNode,
      before: PaintNode,
    ) => void,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(owner, id, setNativeStyleProperty);
  }

  appendChild(child: PaintNode): PaintNode {
    this.appendNativeChild(this, child);
    return child;
  }

  insertBefore(child: PaintNode, before: PaintNode): PaintNode {
    this.insertNativeChildBefore(this, child, before);
    return child;
  }

  detachChild(child: PaintNode): PaintNode {
    return this.ownerDocument.detachChild(this, child);
  }
}

export class FormElement extends DivElement {}

export class ButtonElement extends DivElement {
  private buttonType: "submit" | "button" = "submit";

  get type(): "submit" | "button" {
    return this.buttonType;
  }

  set type(value: string) {
    const next = String(value);
    if (next !== "submit" && next !== "button") {
      throw new Error(
        `paintcannon only supports <button type="submit"> and <button type="button"> right now, got "${next}"`,
      );
    }
    this.buttonType = next;
  }
}

export class SpanElement extends PaintElementEventTarget<ContainerElementEventListenerTuple> {
  constructor(
    owner: PaintCannon,
    id: number,
    private readonly appendNativeChild: (parent: PaintElement, child: PaintNode) => void,
    private readonly insertNativeChildBefore: (
      parent: PaintElement,
      child: PaintNode,
      before: PaintNode,
    ) => void,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(owner, id, setNativeStyleProperty);
  }

  appendChild(child: PaintNode): PaintNode {
    this.appendNativeChild(this, child);
    return child;
  }

  insertBefore(child: PaintNode, before: PaintNode): PaintNode {
    this.insertNativeChildBefore(this, child, before);
    return child;
  }

  detachChild(child: PaintNode): PaintNode {
    return this.ownerDocument.detachChild(this, child);
  }
}

export class ImageElement extends PaintElementEventTarget<BasicElementEventListenerTuple> {
  private source = "";

  constructor(
    owner: PaintCannon,
    id: number,
    private readonly setNativeImageSource: (id: number, src: string) => void,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(owner, id, setNativeStyleProperty);
  }

  get src(): string {
    return this.source;
  }

  set src(value: string) {
    this.source = String(value);
    this.setNativeImageSource(this.id, this.source);
  }
}

abstract class TextControlElementBase<
  TEvents extends ElementEventListenerTuple,
> extends PaintElementEventTarget<TEvents> {
  private readonly graphemeSegmenter = new Intl.Segmenter("und", { granularity: "grapheme" });
  private inputType = "text";
  private inputValue = "";
  private placeholderValue = "";
  private cursor = 0;
  private focused = false;

  constructor(
    owner: PaintCannon,
    id: number,
    private readonly setNativeInputValue: (id: number, value: string, cursor: number) => void,
    private readonly setNativeInputFocused: (id: number, focused: boolean) => void,
    private readonly setNativeInputPlaceholder: (id: number, placeholder: string) => void,
    private readonly setNativeCursorAtPoint: (id: number, x: number, y: number) => number | null,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(owner, id, setNativeStyleProperty);
  }

  protected abstract textControlNode(): TextControlElement;

  get type(): string {
    return this.inputType;
  }

  set type(value: string) {
    const next = String(value);
    if (next !== "text") {
      throw new Error(`paintcannon only supports <input type="text"> right now, got "${next}"`);
    }
    this.inputType = next;
  }

  get value(): string {
    return this.inputValue;
  }

  set value(value: string) {
    const next = String(value);
    if (next === this.inputValue) {
      return;
    }

    const oldLength = Array.from(this.inputValue).length;
    const cursorWasAtEnd = this.cursor === oldLength;
    this.inputValue = next;
    const nextLength = Array.from(this.inputValue).length;
    this.cursor = cursorWasAtEnd ? nextLength : Math.min(this.cursor, nextLength);
    this.syncValue();
  }

  get placeholder(): string {
    return this.placeholderValue;
  }

  set placeholder(value: string) {
    this.placeholderValue = String(value);
    this.setNativeInputPlaceholder(this.id, this.placeholderValue);
  }

  get cursorPosition(): number {
    return this.cursor;
  }

  set cursorPosition(value: number) {
    this.setCursorPosition(value);
  }

  setCursorPosition(position: number): void {
    if (!Number.isFinite(position)) {
      throw new Error(`cursor position must be a finite number, got ${position}`);
    }

    const length = Array.from(this.inputValue).length;
    this.cursor = Math.max(0, Math.min(length, Math.floor(position)));
    this.syncValue();
  }

  protected setCursorPositionFromNative(position: number): void {
    if (!Number.isFinite(position)) {
      throw new Error(`cursor position must be a finite number, got ${position}`);
    }

    const length = Array.from(this.inputValue).length;
    this.cursor = this.graphemeRangeAt(Math.max(0, Math.min(length, Math.floor(position)))).start;
  }

  setCursorPositionFromNativePoint(x: number, y: number): boolean {
    const cursor = this.setNativeCursorAtPoint(this.id, x, y);
    if (cursor === null) {
      return false;
    }

    this.setCursorPositionFromNative(cursor);
    return true;
  }

  insertText(text: string): void {
    const chars = Array.from(this.inputValue);
    const insert = Array.from(text);
    chars.splice(this.cursor, 0, ...insert);
    this.inputValue = chars.join("");
    this.cursor += insert.length;
    this.syncValue();
  }

  deleteBackward(): boolean {
    if (this.cursor === 0) {
      return false;
    }

    const chars = Array.from(this.inputValue);
    const start = this.graphemeRangeAt(this.cursor - 1).start;
    chars.splice(start, this.cursor - start);
    this.cursor = start;
    this.inputValue = chars.join("");
    this.syncValue();
    return true;
  }

  deleteForward(): boolean {
    const chars = Array.from(this.inputValue);
    if (this.cursor >= chars.length) {
      return false;
    }

    const end = this.graphemeRangeAt(this.cursor).end;
    chars.splice(this.cursor, end - this.cursor);
    this.inputValue = chars.join("");
    this.syncValue();
    return true;
  }

  deleteToStart(): boolean {
    if (this.cursor === 0) {
      return false;
    }

    const chars = Array.from(this.inputValue);
    chars.splice(0, this.cursor);
    this.inputValue = chars.join("");
    this.cursor = 0;
    this.syncValue();
    return true;
  }

  deleteToEnd(): boolean {
    const chars = Array.from(this.inputValue);
    if (this.cursor >= chars.length) {
      return false;
    }

    chars.splice(this.cursor);
    this.inputValue = chars.join("");
    this.syncValue();
    return true;
  }

  deletePreviousWord(): boolean {
    if (this.cursor === 0) {
      return false;
    }

    const chars = Array.from(this.inputValue);
    let start = this.cursor;
    while (start > 0 && chars[start - 1] === " ") {
      start -= 1;
    }
    while (start > 0 && chars[start - 1] !== " ") {
      start -= 1;
    }

    chars.splice(start, this.cursor - start);
    this.inputValue = chars.join("");
    this.cursor = start;
    this.syncValue();
    return true;
  }

  moveCursorHorizontally(direction: -1 | 1): void {
    this.cursor =
      direction === -1
        ? this.graphemeRangeAt(this.cursor - 1).start
        : this.graphemeRangeAt(this.cursor).end;
    this.syncValue();
  }

  cursorToStart(): void {
    this.cursor = 0;
    this.syncValue();
  }

  cursorToEnd(): void {
    this.cursor = Array.from(this.inputValue).length;
    this.syncValue();
  }

  setFocused(focused: boolean): void {
    if (this.focused === focused) {
      return;
    }

    this.focused = focused;
    this.setNativeInputFocused(this.id, focused);
  }

  private graphemeRangeAt(position: number): { start: number; end: number } {
    let start = 0;
    for (const { segment } of this.graphemeSegmenter.segment(this.inputValue)) {
      const end = start + Array.from(segment).length;
      if (position < end) {
        return { start, end };
      }
      start = end;
    }
    return { start, end: start };
  }

  private syncValue(): void {
    this.cursor = this.graphemeRangeAt(this.cursor).start;
    this.setNativeInputValue(this.id, this.inputValue, this.cursor);
  }
}

export class InputElement extends TextControlElementBase<BasicElementEventListenerTuple> {
  protected textControlNode(): InputElement {
    return this;
  }
}

export class TextAreaElement extends TextControlElementBase<TextAreaElementEventListenerTuple> {
  constructor(
    owner: PaintCannon,
    id: number,
    setNativeInputValue: (id: number, value: string, cursor: number) => void,
    setNativeInputFocused: (id: number, focused: boolean) => void,
    setNativeInputPlaceholder: (id: number, placeholder: string) => void,
    setNativeCursorAtPoint: (id: number, x: number, y: number) => number | null,
    private readonly moveNativeTextAreaCursorVertically: (
      id: number,
      direction: number,
    ) => number | null,
    private readonly getNativeTextAreaCursorVisualPosition: (
      id: number,
    ) => CursorVisualPosition | null,
    private readonly getNativeTextAreaVisualLineRange: (
      id: number,
      row: number,
    ) => VisualLineRange | null,
    setNativeStyleProperty: SetNativeStyleProperty,
  ) {
    super(
      owner,
      id,
      setNativeInputValue,
      setNativeInputFocused,
      setNativeInputPlaceholder,
      setNativeCursorAtPoint,
      setNativeStyleProperty,
    );
  }

  protected textControlNode(): TextAreaElement {
    return this;
  }

  override get type(): string {
    return "textarea";
  }

  override set type(value: string) {
    throw new Error(`textarea does not support input type assignment, got "${String(value)}"`);
  }

  moveCursorVertically(direction: -1 | 1): boolean {
    const cursor = this.moveNativeTextAreaCursorVertically(this.id, direction);
    if (cursor === null) {
      return false;
    }

    this.setCursorPositionFromNative(cursor);
    return true;
  }

  getCursorVisualPosition(): CursorVisualPosition | null {
    return this.getNativeTextAreaCursorVisualPosition(this.id);
  }

  getVisualLineRange(row: number): VisualLineRange | null {
    if (!Number.isSafeInteger(row) || row < 0 || row > 0xffffffff) {
      return null;
    }
    return this.getNativeTextAreaVisualLineRange(this.id, row);
  }
}

export class TextNode extends PaintNodeBase {
  constructor(
    owner: PaintCannon,
    id: number,
    private readonly setNativeTextNodeValue: (id: number, value: string) => void,
    private data: string = "",
  ) {
    super(owner, id);
  }

  get nodeValue(): string {
    return this.data;
  }

  set nodeValue(value: string) {
    this.data = String(value);
    this.setNativeTextNodeValue(this.id, this.data);
  }

  get textContent(): string {
    return this.nodeValue;
  }

  set textContent(value: string) {
    this.nodeValue = value;
  }
}

export class CSSStyleDeclaration {
  private readonly values: Record<string, string> = Object.create(null);

  constructor(
    private readonly getElementId: () => number,
    private readonly setNativeStyleProperty: (id: number, property: string, value: string) => void,
  ) {}

  setProperty(property: CSSStylePropertyName, value: CSSStyleValue): void {
    const name = normalizeStyleName(property);
    if (!SUPPORTED_STYLE_PROPERTIES.has(name)) {
      throw new Error(`unsupported style property: ${property}`);
    }

    const stringValue = String(value);
    this.values[name] = stringValue;
    this.setNativeStyleProperty(this.getElementId(), name, stringValue);
  }

  removeProperty(property: CSSStylePropertyName): string {
    const name = normalizeStyleName(property);
    if (!SUPPORTED_STYLE_PROPERTIES.has(name)) {
      throw new Error(`unsupported style property: ${property}`);
    }

    const previous = this.getPropertyValue(name);
    delete this.values[name];
    this.setNativeStyleProperty(this.getElementId(), name, "");
    return previous;
  }

  getPropertyValue(property: CSSStylePropertyName): string {
    return this.values[normalizeStyleName(property)] || "";
  }

  get display(): "inline" | "block" | "flex" | "flexbox" | "grid" | string {
    return this.getPropertyValue("display");
  }

  set display(value: "inline" | "block" | "flex" | "flexbox" | "grid" | string) {
    this.setProperty("display", value);
  }

  get position(): CSSPosition | string {
    return this.getPropertyValue("position");
  }

  set position(value: CSSPosition | string) {
    this.setProperty("position", value);
  }

  get top(): string {
    return this.getPropertyValue("top");
  }

  set top(value: string | number) {
    this.setProperty("top", value);
  }

  get right(): string {
    return this.getPropertyValue("right");
  }

  set right(value: string | number) {
    this.setProperty("right", value);
  }

  get bottom(): string {
    return this.getPropertyValue("bottom");
  }

  set bottom(value: string | number) {
    this.setProperty("bottom", value);
  }

  get left(): string {
    return this.getPropertyValue("left");
  }

  set left(value: string | number) {
    this.setProperty("left", value);
  }

  get zIndex(): string {
    return this.getPropertyValue("z-index");
  }

  set zIndex(value: string | number) {
    this.setProperty("z-index", value);
  }

  get visibility(): CSSVisibility | string {
    return this.getPropertyValue("visibility");
  }

  set visibility(value: CSSVisibility | string) {
    this.setProperty("visibility", value);
  }

  get opacity(): string {
    return this.getPropertyValue("opacity");
  }

  set opacity(value: string | number) {
    this.setProperty("opacity", value);
  }

  get overflow(): "visible" | "hidden" | "scroll" | string {
    return this.getPropertyValue("overflow");
  }

  set overflow(value: "visible" | "hidden" | "scroll" | string) {
    this.setProperty("overflow", value);
  }

  get overflowX(): "visible" | "hidden" | "scroll" | string {
    return this.getPropertyValue("overflow-x");
  }

  set overflowX(value: "visible" | "hidden" | "scroll" | string) {
    this.setProperty("overflow-x", value);
  }

  get overflowY(): "visible" | "hidden" | "scroll" | string {
    return this.getPropertyValue("overflow-y");
  }

  set overflowY(value: "visible" | "hidden" | "scroll" | string) {
    this.setProperty("overflow-y", value);
  }

  get overflowAnchor(): CSSOverflowAnchor | string {
    return this.getPropertyValue("overflow-anchor");
  }

  set overflowAnchor(value: CSSOverflowAnchor | string) {
    this.setProperty("overflow-anchor", value);
  }

  get scrollbarColor(): string {
    return this.getPropertyValue("scrollbar-color");
  }

  set scrollbarColor(value: string) {
    this.setProperty("scrollbar-color", value);
  }

  get scrollbarGutter(): "auto" | "stable" | string {
    return this.getPropertyValue("scrollbar-gutter");
  }

  set scrollbarGutter(value: "auto" | "stable" | string) {
    this.setProperty("scrollbar-gutter", value);
  }

  get flexDirection(): "row" | "column" | string {
    return this.getPropertyValue("flex-direction");
  }

  set flexDirection(value: "row" | "column" | string) {
    this.setProperty("flex-direction", value);
  }

  get flexWrap(): string {
    return this.getPropertyValue("flex-wrap");
  }

  set flexWrap(value: string) {
    this.setProperty("flex-wrap", value);
  }

  get flexFlow(): string {
    return this.getPropertyValue("flex-flow");
  }

  set flexFlow(value: string) {
    this.setProperty("flex-flow", value);
  }

  get flexBasis(): string {
    return this.getPropertyValue("flex-basis");
  }

  set flexBasis(value: string | number) {
    this.setProperty("flex-basis", value);
  }

  get flexGrow(): string {
    return this.getPropertyValue("flex-grow");
  }

  set flexGrow(value: string | number) {
    this.setProperty("flex-grow", value);
  }

  get flexShrink(): string {
    return this.getPropertyValue("flex-shrink");
  }

  set flexShrink(value: string | number) {
    this.setProperty("flex-shrink", value);
  }

  get flex(): string {
    return this.getPropertyValue("flex");
  }

  set flex(value: string | number) {
    this.setProperty("flex", value);
  }

  get justifyContent(): string {
    return this.getPropertyValue("justify-content");
  }

  set justifyContent(value: string) {
    this.setProperty("justify-content", value);
  }

  get alignItems(): string {
    return this.getPropertyValue("align-items");
  }

  set alignItems(value: string) {
    this.setProperty("align-items", value);
  }

  get alignSelf(): string {
    return this.getPropertyValue("align-self");
  }

  set alignSelf(value: string) {
    this.setProperty("align-self", value);
  }

  get alignContent(): string {
    return this.getPropertyValue("align-content");
  }

  set alignContent(value: string) {
    this.setProperty("align-content", value);
  }

  get justifyItems(): string {
    return this.getPropertyValue("justify-items");
  }

  set justifyItems(value: string) {
    this.setProperty("justify-items", value);
  }

  get justifySelf(): string {
    return this.getPropertyValue("justify-self");
  }

  set justifySelf(value: string) {
    this.setProperty("justify-self", value);
  }

  get gap(): string {
    return this.getPropertyValue("gap");
  }

  set gap(value: string | number) {
    this.setProperty("gap", value);
  }

  get rowGap(): string {
    return this.getPropertyValue("row-gap");
  }

  set rowGap(value: string | number) {
    this.setProperty("row-gap", value);
  }

  get columnGap(): string {
    return this.getPropertyValue("column-gap");
  }

  set columnGap(value: string | number) {
    this.setProperty("column-gap", value);
  }

  get padding(): string {
    return this.getPropertyValue("padding");
  }

  set padding(value: string | number) {
    this.setProperty("padding", value);
  }

  get paddingTop(): string {
    return this.getPropertyValue("padding-top");
  }

  set paddingTop(value: string | number) {
    this.setProperty("padding-top", value);
  }

  get paddingRight(): string {
    return this.getPropertyValue("padding-right");
  }

  set paddingRight(value: string | number) {
    this.setProperty("padding-right", value);
  }

  get paddingBottom(): string {
    return this.getPropertyValue("padding-bottom");
  }

  set paddingBottom(value: string | number) {
    this.setProperty("padding-bottom", value);
  }

  get paddingLeft(): string {
    return this.getPropertyValue("padding-left");
  }

  set paddingLeft(value: string | number) {
    this.setProperty("padding-left", value);
  }

  get margin(): string {
    return this.getPropertyValue("margin");
  }

  set margin(value: string | number) {
    this.setProperty("margin", value);
  }

  get marginTop(): string {
    return this.getPropertyValue("margin-top");
  }

  set marginTop(value: string | number) {
    this.setProperty("margin-top", value);
  }

  get marginRight(): string {
    return this.getPropertyValue("margin-right");
  }

  set marginRight(value: string | number) {
    this.setProperty("margin-right", value);
  }

  get marginBottom(): string {
    return this.getPropertyValue("margin-bottom");
  }

  set marginBottom(value: string | number) {
    this.setProperty("margin-bottom", value);
  }

  get marginLeft(): string {
    return this.getPropertyValue("margin-left");
  }

  set marginLeft(value: string | number) {
    this.setProperty("margin-left", value);
  }

  get width(): string {
    return this.getPropertyValue("width");
  }

  set width(value: string | number) {
    this.setProperty("width", value);
  }

  get height(): string {
    return this.getPropertyValue("height");
  }

  set height(value: string | number) {
    this.setProperty("height", value);
  }

  get minWidth(): string {
    return this.getPropertyValue("min-width");
  }

  set minWidth(value: string | number) {
    this.setProperty("min-width", value);
  }

  get maxWidth(): string {
    return this.getPropertyValue("max-width");
  }

  set maxWidth(value: string | number) {
    this.setProperty("max-width", value);
  }

  get minHeight(): string {
    return this.getPropertyValue("min-height");
  }

  set minHeight(value: string | number) {
    this.setProperty("min-height", value);
  }

  get maxHeight(): string {
    return this.getPropertyValue("max-height");
  }

  set maxHeight(value: string | number) {
    this.setProperty("max-height", value);
  }

  get whiteSpace(): CSSWhiteSpace | string {
    return this.getPropertyValue("white-space");
  }

  set whiteSpace(value: CSSWhiteSpace | string) {
    this.setProperty("white-space", value);
  }

  get overflowWrap(): CSSOverflowWrap | string {
    return this.getPropertyValue("overflow-wrap");
  }

  set overflowWrap(value: CSSOverflowWrap | string) {
    this.setProperty("overflow-wrap", value);
  }

  get wordBreak(): CSSWordBreak | string {
    return this.getPropertyValue("word-break");
  }

  set wordBreak(value: CSSWordBreak | string) {
    this.setProperty("word-break", value);
  }

  get imageRendering(): ImageRendering | string {
    return this.getPropertyValue("image-rendering");
  }

  set imageRendering(value: ImageRendering | string) {
    this.setProperty("image-rendering", value);
  }

  get border(): string {
    return this.getPropertyValue("border");
  }

  set border(value: string) {
    this.setProperty("border", value);
  }

  get borderTop(): string {
    return this.getPropertyValue("border-top");
  }

  set borderTop(value: string) {
    this.setProperty("border-top", value);
  }

  get borderRight(): string {
    return this.getPropertyValue("border-right");
  }

  set borderRight(value: string) {
    this.setProperty("border-right", value);
  }

  get borderBottom(): string {
    return this.getPropertyValue("border-bottom");
  }

  set borderBottom(value: string) {
    this.setProperty("border-bottom", value);
  }

  get borderLeft(): string {
    return this.getPropertyValue("border-left");
  }

  set borderLeft(value: string) {
    this.setProperty("border-left", value);
  }

  get borderColor(): string {
    return this.getPropertyValue("border-color");
  }

  set borderColor(value: string) {
    this.setProperty("border-color", value);
  }

  get color(): string {
    return this.getPropertyValue("color");
  }

  set color(value: string) {
    this.setProperty("color", value);
  }

  get placeholderColor(): string {
    return this.getPropertyValue("placeholder-color");
  }

  set placeholderColor(value: string) {
    this.setProperty("placeholder-color", value);
  }

  get backgroundColor(): string {
    return this.getPropertyValue("background-color");
  }

  set backgroundColor(value: string) {
    this.setProperty("background-color", value);
  }

  get selectionBackgroundColor(): string {
    return this.getPropertyValue("selection-background-color");
  }

  set selectionBackgroundColor(value: string) {
    this.setProperty("selection-background-color", value);
  }

  get fontWeight(): CSSFontWeight | string {
    return this.getPropertyValue("font-weight");
  }

  set fontWeight(value: CSSFontWeight) {
    this.setProperty("font-weight", value);
  }

  get fontStyle(): CSSFontStyle | string {
    return this.getPropertyValue("font-style");
  }

  set fontStyle(value: CSSFontStyle) {
    this.setProperty("font-style", value);
  }

  get textDecoration(): CSSTextDecoration | string {
    return this.getPropertyValue("text-decoration");
  }

  set textDecoration(value: CSSTextDecoration) {
    this.setProperty("text-decoration", value);
  }

  get textDecorationLine(): CSSTextDecoration | string {
    return this.getPropertyValue("text-decoration-line");
  }

  set textDecorationLine(value: CSSTextDecoration) {
    this.setProperty("text-decoration-line", value);
  }

  get transition(): string {
    return this.getPropertyValue("transition");
  }

  set transition(value: string) {
    this.setProperty("transition", value);
  }

  get cursor(): CSSCursor | string {
    return this.getPropertyValue("cursor");
  }

  set cursor(value: CSSCursor | string) {
    this.setProperty("cursor", value);
  }

  get gridTemplateColumns(): string {
    return this.getPropertyValue("grid-template-columns");
  }

  set gridTemplateColumns(value: string) {
    this.setProperty("grid-template-columns", value);
  }

  get gridTemplateRows(): string {
    return this.getPropertyValue("grid-template-rows");
  }

  set gridTemplateRows(value: string) {
    this.setProperty("grid-template-rows", value);
  }

  get gridAutoColumns(): string {
    return this.getPropertyValue("grid-auto-columns");
  }

  set gridAutoColumns(value: string) {
    this.setProperty("grid-auto-columns", value);
  }

  get gridAutoRows(): string {
    return this.getPropertyValue("grid-auto-rows");
  }

  set gridAutoRows(value: string) {
    this.setProperty("grid-auto-rows", value);
  }

  get gridAutoFlow(): string {
    return this.getPropertyValue("grid-auto-flow");
  }

  set gridAutoFlow(value: string) {
    this.setProperty("grid-auto-flow", value);
  }

  get gridColumn(): string {
    return this.getPropertyValue("grid-column");
  }

  set gridColumn(value: string) {
    this.setProperty("grid-column", value);
  }

  get gridRow(): string {
    return this.getPropertyValue("grid-row");
  }

  set gridRow(value: string) {
    this.setProperty("grid-row", value);
  }

  get gridColumnStart(): string {
    return this.getPropertyValue("grid-column-start");
  }

  set gridColumnStart(value: string | number) {
    this.setProperty("grid-column-start", value);
  }

  get gridColumnEnd(): string {
    return this.getPropertyValue("grid-column-end");
  }

  set gridColumnEnd(value: string | number) {
    this.setProperty("grid-column-end", value);
  }

  get gridRowStart(): string {
    return this.getPropertyValue("grid-row-start");
  }

  set gridRowStart(value: string | number) {
    this.setProperty("grid-row-start", value);
  }

  get gridRowEnd(): string {
    return this.getPropertyValue("grid-row-end");
  }

  set gridRowEnd(value: string | number) {
    this.setProperty("grid-row-end", value);
  }
}

function assertElement(value: unknown): asserts value is PaintElement {
  if (!isPaintElement(value)) {
    throw new TypeError("expected a paintcannon element");
  }
}

function isPaintElementTagName(tagName: string): tagName is PaintElementTagName {
  return PAINT_ELEMENT_TAG_NAME_SET.has(tagName);
}

function isPaintElement(value: unknown): value is PaintElement {
  return (
    value instanceof DivElement ||
    value instanceof SpanElement ||
    value instanceof FormElement ||
    value instanceof ButtonElement ||
    value instanceof ImageElement ||
    value instanceof InputElement ||
    value instanceof TextAreaElement
  );
}

function isTextControl(value: unknown): value is TextControlElement {
  return value instanceof InputElement || value instanceof TextAreaElement;
}

function moveTextAreaCursorToLineStart(input: TextAreaElement): void {
  const chars = Array.from(input.value);
  input.cursorPosition = lineStart(chars, input.cursorPosition);
}

function moveTextAreaCursorToLineEnd(input: TextAreaElement): void {
  const chars = Array.from(input.value);
  input.cursorPosition = lineEnd(chars, input.cursorPosition);
}

function lineStart(chars: string[], cursor: number): number {
  let index = Math.max(0, Math.min(chars.length, Math.floor(cursor)));
  while (index > 0 && chars[index - 1] !== "\n") {
    index -= 1;
  }
  return index;
}

function lineEnd(chars: string[], cursor: number): number {
  let index = Math.max(0, Math.min(chars.length, Math.floor(cursor)));
  while (index < chars.length && chars[index] !== "\n") {
    index += 1;
  }
  return index;
}

function isElementEventType(type: string): type is ElementEventType {
  return (ELEMENT_EVENT_TYPES as readonly string[]).includes(type);
}

function isAxisScrollable(value: string): boolean {
  return value === "scroll";
}

function isElementAxisScrollable(element: PaintElement, axis: ScrollbarAxis): boolean {
  const overflow =
    axis === "x"
      ? element.style.overflowX || element.style.overflow
      : element.style.overflowY || element.style.overflow;
  if (isAxisScrollable(overflow)) {
    return true;
  }

  return element instanceof TextAreaElement && axis === "y" && overflow !== "hidden";
}

function canScrollAxisInDirection(
  axis: ScrollbarAxis,
  delta: number,
  metrics: NativeScrollMetrics,
): boolean {
  const offset = axis === "x" ? metrics.scrollLeft : metrics.scrollTop;
  const scrollLength = axis === "x" ? metrics.scrollWidth : metrics.scrollHeight;
  const clientLength = axis === "x" ? metrics.clientWidth : metrics.clientHeight;
  const maxOffset = Math.max(0, scrollLength - clientLength);
  return delta < 0 ? offset > 0 : offset < maxOffset;
}

function scrollbarCoordinate(input: TerminalMouseEvent, axis: ScrollbarAxis): number {
  return axis === "x" ? input.x : input.y;
}

function parseScrollbarAxis(axis: string): ScrollbarAxis {
  if (axis === "x" || axis === "y") {
    return axis;
  }
  throw new Error(`unsupported native scrollbar axis: ${axis}`);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function normalizeScrollOffset(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.floor(value));
}

function emptyScrollMetrics(): NativeScrollMetrics {
  return {
    scrollLeft: 0,
    scrollTop: 0,
    scrollWidth: 0,
    scrollHeight: 0,
    clientWidth: 0,
    clientHeight: 0,
  };
}

function normalizeStyleName(property: string): CSSStyleProperty {
  return property.replace(/[A-Z]/g, char => `-${char.toLowerCase()}`) as CSSStyleProperty;
}

export const SUPPORTED_STYLE_PROPERTY_NAMES = [
  "display",
  "position",
  "top",
  "right",
  "bottom",
  "left",
  "z-index",
  "visibility",
  "opacity",
  "overflow",
  "overflow-x",
  "overflow-y",
  "overflow-anchor",
  "scrollbar-color",
  "scrollbar-gutter",
  "image-rendering",
  "flex-direction",
  "flex-wrap",
  "flex-flow",
  "flex-basis",
  "flex-grow",
  "flex-shrink",
  "flex",
  "justify-content",
  "align-items",
  "align-self",
  "align-content",
  "justify-items",
  "justify-self",
  "gap",
  "row-gap",
  "column-gap",
  "padding",
  "padding-top",
  "padding-right",
  "padding-bottom",
  "padding-left",
  "margin",
  "margin-top",
  "margin-right",
  "margin-bottom",
  "margin-left",
  "width",
  "height",
  "min-width",
  "max-width",
  "min-height",
  "max-height",
  "white-space",
  "overflow-wrap",
  "word-break",
  "border",
  "border-top",
  "border-right",
  "border-bottom",
  "border-left",
  "border-color",
  "color",
  "placeholder-color",
  "transition",
  "font-weight",
  "font-style",
  "text-decoration",
  "text-decoration-line",
  "background",
  "background-color",
  "selection-background-color",
  "cursor",
  "grid-template-columns",
  "grid-template-rows",
  "grid-auto-columns",
  "grid-auto-rows",
  "grid-auto-flow",
  "grid-column",
  "grid-row",
  "grid-column-start",
  "grid-column-end",
  "grid-row-start",
  "grid-row-end",
] as const;
export type CSSStyleProperty = (typeof SUPPORTED_STYLE_PROPERTY_NAMES)[number];
type CamelCase<S extends string> = S extends `${infer Head}-${infer Tail}`
  ? `${Head}${Capitalize<CamelCase<Tail>>}`
  : S;
export type CSSStylePropertyName = CSSStyleProperty | CamelCase<CSSStyleProperty>;
export type CSSStyleProperties = Partial<
  Record<CSSStylePropertyName, CSSStyleValue | null | undefined>
>;
const SUPPORTED_STYLE_PROPERTIES = new Set<string>(SUPPORTED_STYLE_PROPERTY_NAMES);

function domRect(rect: NativeIntersectionRect): DOMRectReadOnly {
  return new DOMRectReadOnly(rect.x, rect.y, rect.width, rect.height);
}

function parseRootMargin(value: string): string[] {
  const parts = value.trim().split(/\s+/).filter(Boolean);
  if (parts.length < 1 || parts.length > 4) {
    throw new SyntaxError(`invalid IntersectionObserver rootMargin: ${value}`);
  }
  const normalized = parts.map(part => {
    const normalizedPart = part.toLowerCase();
    if (normalizedPart === "0" || normalizedPart === "+0" || normalizedPart === "-0") {
      return "0px";
    }
    if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:px|%)$/.test(normalizedPart)) {
      throw new SyntaxError(`invalid IntersectionObserver rootMargin: ${value}`);
    }
    return normalizedPart;
  });
  const [top, second = top, third = top, fourth = second] = normalized;
  return [top, second, third, fourth];
}

function normalizeThresholds(value: number | number[] | undefined): readonly number[] {
  const thresholds = value === undefined ? [0] : Array.isArray(value) ? value : [value];
  if (thresholds.some(threshold => !Number.isFinite(threshold) || threshold < 0 || threshold > 1)) {
    throw new RangeError("IntersectionObserver thresholds must be finite numbers between 0 and 1");
  }
  return Array.from(new Set(thresholds.length === 0 ? [0] : thresholds)).sort(
    (left, right) => left - right,
  );
}

function fpsToInterval(fps: number): number {
  if (!Number.isFinite(fps) || fps <= 0) {
    throw new RangeError(`fps must be a positive finite number, got ${fps}`);
  }

  return 1000 / fps;
}

function loadNativeBinding(): NativeBinding {
  return nativeBinding;
}
