import type {
  NativeBatchCommand,
  NativeBatchIdMapping,
  NativeBinding,
  NativeKeyboardEvent,
  NativePaintCannon,
  NativeScrollbarHit,
  NativeScrollMetrics,
  NativeTransitionEvent,
  TerminalMouseEvent,
  TerminalResizeEvent,
  TerminalSize,
} from "../main.ts";

export interface NativeTextControlState {
  value: string;
  cursor: number;
  focused: boolean;
  placeholder: string;
}

export interface NativeStyleMutation {
  id: number;
  property: string;
  value: string;
}

export function createMockNativeBinding(instances: MockNativePaintCannon[] = []): NativeBinding {
  return {
    PaintCannon: class extends MockNativePaintCannon {
      constructor(
        forceCompatMode?: boolean,
        alternateScreen?: boolean,
        captureMouse?: boolean,
        captureCtrlC?: boolean,
      ) {
        super(forceCompatMode, alternateScreen, captureMouse, captureCtrlC);
        instances.push(this);
      }
    },
  };
}

export class MockNativePaintCannon implements NativePaintCannon {
  readonly kittyKeyboardEnabled = false;
  renderCalls = 0;
  renderSyncCalls = 0;
  stopCalls = 0;
  releaseTerminalCalls = 0;
  captureTerminalCalls = 0;
  suspendProcessGroupCalls = 0;
  interruptProcessGroupCalls = 0;
  targetIdAtPoint: number | null = null;
  scrollbarHitAtPoint: NativeScrollbarHit | null = null;
  cursorAtPoint: number | null = null;
  keyboardEvents: NativeKeyboardEvent[] = [];
  mouseEvents: TerminalMouseEvent[] = [];
  resizeEvents: TerminalResizeEvent[] = [];
  transitionEvents: NativeTransitionEvent[] = [];
  textControls = new Map<number, NativeTextControlState>();
  styleMutations: NativeStyleMutation[] = [];
  scrollMetricsById = new Map<number, NativeScrollMetrics>();
  private nextId = 1;

  constructor(
    readonly forceCompatMode = false,
    readonly alternateScreen = false,
    readonly captureMouse = false,
    readonly captureCtrlC = false,
  ) {}

  createDiv(): number {
    return this.allocateId();
  }

  createSpan(): number {
    return this.allocateId();
  }

  createImage(): number {
    return this.allocateId();
  }

  createInput(): number {
    const id = this.allocateId();
    this.textControls.set(id, emptyTextControlState());
    return id;
  }

  createTextArea(): number {
    const id = this.allocateId();
    this.textControls.set(id, emptyTextControlState());
    return id;
  }

  createTextNode(_text: string): number {
    return this.allocateId();
  }

  setTextNodeValue(_id: number, _text: string): void {}
  setImageSource(_id: number, _src: string): void {}

  setInputValue(id: number, value: string, cursor: number): void {
    this.setTextControlValue(id, value, cursor);
  }

  setInputFocused(id: number, focused: boolean): void {
    this.setTextControlFocused(id, focused);
  }

  setInputPlaceholder(id: number, placeholder: string): void {
    this.setTextControlPlaceholder(id, placeholder);
  }

  setTextAreaValue(id: number, value: string, cursor: number): void {
    this.setTextControlValue(id, value, cursor);
  }

  setTextAreaFocused(id: number, focused: boolean): void {
    this.setTextControlFocused(id, focused);
  }

  setTextAreaPlaceholder(id: number, placeholder: string): void {
    this.setTextControlPlaceholder(id, placeholder);
  }

  moveTextAreaCursorVertically(_id: number, _direction: number): number | null {
    return null;
  }

  setTextControlCursorAtPoint(_id: number, _x: number, _y: number): number | null {
    return this.cursorAtPoint;
  }

  setRoot(_id: number): void {}
  appendChild(_parent: number, _child: number): void {}
  insertChildBefore(_parent: number, _child: number, _before: number): void {}
  detachNode(_id: number): void {}
  destroyNode(_id: number): void {}
  setStyleProperty(id: number, property: string, value: string): void {
    this.styleMutations.push({ id, property, value });
  }

  applyBatch(commands: NativeBatchCommand[]): NativeBatchIdMapping[] {
    const mappings: NativeBatchIdMapping[] = [];
    for (const command of commands) {
      if (command.id !== undefined && command.id < 0) {
        mappings.push({ temporaryId: command.id, id: this.allocateId() });
      }
    }
    for (const command of commands) {
      if (
        command.type === "setStyleProperty" &&
        command.id !== undefined &&
        command.property !== undefined &&
        command.value !== undefined
      ) {
        const id = resolveBatchId(command.id, mappings);
        this.setStyleProperty(id, command.property, command.value);
      }
    }
    return mappings;
  }

  terminalSize(): TerminalSize {
    return {
      cols: 80,
      rows: 24,
      pixelWidth: 800,
      pixelHeight: 480,
    };
  }

  render(): void {
    this.renderCalls += 1;
  }

  renderSync(): void {
    this.renderSyncCalls += 1;
  }

  invalidateFrame(): void {}

  drainKeyboardEvents(): NativeKeyboardEvent[] {
    const events = this.keyboardEvents;
    this.keyboardEvents = [];
    return events;
  }

  drainMouseEvents(): TerminalMouseEvent[] {
    const events = this.mouseEvents;
    this.mouseEvents = [];
    return events;
  }

  drainResizeEvents(): TerminalResizeEvent[] {
    const events = this.resizeEvents;
    this.resizeEvents = [];
    return events;
  }

  drainTransitionEvents(): NativeTransitionEvent[] {
    const events = this.transitionEvents;
    this.transitionEvents = [];
    return events;
  }

  clickEventForMouseClick(): null {
    return null;
  }

  targetIdForPoint(): number | null {
    return this.targetIdAtPoint;
  }

  scrollbarHitForPoint(): NativeScrollbarHit | null {
    return this.scrollbarHitAtPoint;
  }

  setScrollOffset(id: number, scrollLeft: number, scrollTop: number): NativeScrollMetrics {
    const current = this.scrollMetrics(id);
    const maxLeft = Math.max(0, current.scrollWidth - current.clientWidth);
    const maxTop = Math.max(0, current.scrollHeight - current.clientHeight);
    const metrics = {
      ...current,
      scrollLeft: Math.min(maxLeft, Math.max(0, Math.floor(scrollLeft))),
      scrollTop: Math.min(maxTop, Math.max(0, Math.floor(scrollTop))),
    };
    this.scrollMetricsById.set(id, metrics);
    return metrics;
  }

  scrollMetrics(id: number): NativeScrollMetrics {
    return (
      this.scrollMetricsById.get(id) ?? {
        scrollLeft: 0,
        scrollTop: 0,
        scrollWidth: 0,
        scrollHeight: 0,
        clientWidth: 0,
        clientHeight: 0,
      }
    );
  }

  setSyntheticKeyupDelay(_delayMs: number): void {}

  releaseTerminal(): void {
    this.releaseTerminalCalls += 1;
  }

  captureTerminal(): void {
    this.captureTerminalCalls += 1;
  }

  interruptProcessGroup(): void {
    this.interruptProcessGroupCalls += 1;
  }

  suspendProcessGroup(): void {
    this.suspendProcessGroupCalls += 1;
  }

  stop(): void {
    this.stopCalls += 1;
  }

  private allocateId(): number {
    const id = this.nextId;
    this.nextId += 1;
    return id;
  }

  private setTextControlValue(id: number, value: string, cursor: number): void {
    this.ensureTextControl(id).value = value;
    this.ensureTextControl(id).cursor = cursor;
  }

  private setTextControlFocused(id: number, focused: boolean): void {
    this.ensureTextControl(id).focused = focused;
  }

  private setTextControlPlaceholder(id: number, placeholder: string): void {
    this.ensureTextControl(id).placeholder = placeholder;
  }

  private ensureTextControl(id: number): NativeTextControlState {
    let state = this.textControls.get(id);
    if (state === undefined) {
      state = emptyTextControlState();
      this.textControls.set(id, state);
    }
    return state;
  }
}

function resolveBatchId(id: number, mappings: NativeBatchIdMapping[]): number {
  if (id >= 0) {
    return id;
  }

  return mappings.find(mapping => mapping.temporaryId === id)?.id ?? id;
}

export function keyDown(
  key: string,
  options: Partial<NativeKeyboardEvent> = {},
): NativeKeyboardEvent {
  return {
    type: "keydown",
    key,
    code: options.code ?? (key.length === 1 ? `Key${key.toUpperCase()}` : key),
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    repeat: false,
    ...options,
  };
}

export function mouseEvent(
  type: TerminalMouseEvent["type"],
  options: Partial<TerminalMouseEvent> = {},
): TerminalMouseEvent {
  return {
    type,
    x: 1,
    y: 1,
    button: 0,
    deltaX: 0,
    deltaY: 0,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    ...options,
  };
}

function emptyTextControlState(): NativeTextControlState {
  return {
    value: "",
    cursor: 0,
    focused: false,
    placeholder: "",
  };
}
