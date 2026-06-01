import type {
  NativeBatchCommand,
  NativeBatchIdMapping,
  NativeBinding,
  NativeKeyboardEvent,
  NativePaintCannon,
  NativeScrollMetrics,
  NativeTransitionEvent,
  TerminalMouseEvent,
  TerminalResizeEvent,
  TerminalSize,
} from 'paintcannon';

export function createFakeNativeBinding(instances: FakeNativePaintCannon[] = []): NativeBinding {
  return {
    PaintCannon: class extends FakeNativePaintCannon {
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

export class FakeNativePaintCannon implements NativePaintCannon {
  readonly kittyKeyboardEnabled = false;
  renderCalls = 0;
  renderSyncCalls = 0;
  resizeEvents: TerminalResizeEvent[] = [];
  private nextId = 1;

  constructor(
    _forceCompatMode?: boolean,
    _alternateScreen?: boolean,
    _captureMouse?: boolean,
    _captureCtrlC?: boolean,
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
    return this.allocateId();
  }

  createTextArea(): number {
    return this.allocateId();
  }

  createTextNode(_text: string): number {
    return this.allocateId();
  }

  setTextNodeValue(_id: number, _text: string): void {}
  setImageSource(_id: number, _src: string): void {}
  setInputValue(_id: number, _value: string, _cursor: number): void {}
  setInputFocused(_id: number, _focused: boolean): void {}
  setInputPlaceholder(_id: number, _placeholder: string): void {}
  setTextAreaValue(_id: number, _value: string, _cursor: number): void {}
  setTextAreaFocused(_id: number, _focused: boolean): void {}
  setTextAreaPlaceholder(_id: number, _placeholder: string): void {}

  moveTextAreaCursorVertically(_id: number, _direction: number): number | null {
    return null;
  }

  setTextControlCursorAtPoint(_id: number, _x: number, _y: number): number | null {
    return null;
  }

  setRoot(_id: number): void {}
  appendChild(_parent: number, _child: number): void {}
  insertChildBefore(_parent: number, _child: number, _before: number): void {}
  detachNode(_id: number): void {}
  destroyNode(_id: number): void {}
  setStyleProperty(_id: number, _property: string, _value: string): void {}

  applyBatch(commands: NativeBatchCommand[]): NativeBatchIdMapping[] {
    const mappings: NativeBatchIdMapping[] = [];
    for (const command of commands) {
      if (command.id !== undefined && command.id < 0) {
        mappings.push({ temporaryId: command.id, id: this.allocateId() });
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
    return [];
  }

  drainMouseEvents(): TerminalMouseEvent[] {
    return [];
  }

  drainResizeEvents(): TerminalResizeEvent[] {
    const events = this.resizeEvents;
    this.resizeEvents = [];
    return events;
  }

  drainTransitionEvents(): NativeTransitionEvent[] {
    return [];
  }

  clickEventForMouseClick(): null {
    return null;
  }

  targetIdForPoint(): null {
    return null;
  }

  setScrollOffset(_id: number, scrollLeft: number, scrollTop: number): NativeScrollMetrics {
    return {
      scrollLeft,
      scrollTop,
      scrollWidth: 0,
      scrollHeight: 0,
      clientWidth: 0,
      clientHeight: 0,
    };
  }

  scrollMetrics(_id: number): NativeScrollMetrics {
    return {
      scrollLeft: 0,
      scrollTop: 0,
      scrollWidth: 0,
      scrollHeight: 0,
      clientWidth: 0,
      clientHeight: 0,
    };
  }

  setSyntheticKeyupDelay(_delayMs: number): void {}
  releaseTerminal(): void {}
  captureTerminal(): void {}
  interruptProcessGroup(): void {}
  suspendProcessGroup(): void {}
  stop(): void {}

  private allocateId(): number {
    const id = this.nextId;
    this.nextId += 1;
    return id;
  }
}
