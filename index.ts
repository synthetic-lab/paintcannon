import fs = require('node:fs');
import path = require('node:path');
import { performance } from 'node:perf_hooks';

export interface PaintCannonOptions {
  fps?: number;
  syntheticKeyupDelayMs?: number;
  forceCompatMode?: boolean;
  captureCtrlC?: boolean;
  captureCtrlZ?: boolean;
  alternateScreen?: boolean;
  captureMouse?: boolean;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

export type AnimationFrameCallback = (timestamp: number) => void;
export type KeyboardEventType = 'keydown' | 'keyup';
export type KeyboardEventListener = (event: KeyboardEvent) => void;
export type ElementEventType = 'click';
export type ClickEventListener = (event: PaintMouseEvent) => void;

export interface KeyboardEvent {
  type: KeyboardEventType;
  key: string;
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
}

export interface TerminalMouseClick {
  x: number;
  y: number;
  button: number;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

export interface NativeClickEvent {
  type: 'click';
  targetId: number;
  clientX: number;
  clientY: number;
  button: number;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

export interface NativePaintCannon {
  createDiv(): number;
  createSpan(): number;
  createTextNode(text: string): number;
  setTextNodeValue(id: number, text: string): void;
  setRoot(id: number): void;
  appendChild(parent: number, child: number): void;
  setStyleProperty(id: number, property: string, value: string): void;
  terminalSize(): TerminalSize;
  readonly kittyKeyboardEnabled: boolean;
  render(): void;
  drainKeyboardEvents(): KeyboardEvent[];
  drainMouseClicks(): TerminalMouseClick[];
  clickEventForMouseClick(
    x: number,
    y: number,
    button: number,
    ctrlKey: boolean,
    altKey: boolean,
    metaKey: boolean,
    shiftKey: boolean,
  ): NativeClickEvent | null;
  setSyntheticKeyupDelay(delayMs: number): void;
  releaseTerminal(): void;
  captureTerminal(): void;
  interruptProcessGroup(): void;
  suspendProcessGroup(): void;
  stop(): void;
}

export interface NativeBinding {
  PaintCannon: new (
    forceCompatMode?: boolean,
    alternateScreen?: boolean,
    captureMouse?: boolean,
  ) => NativePaintCannon;
}

export type PaintElement = DivElement | SpanElement;
export type PaintNode = PaintElement | TextNode;

export const native: NativeBinding = loadNativeBinding();

export class PaintCannon {
  private readonly binding: NativePaintCannon;
  private frameIntervalMs: number;
  private stopped = false;
  private nextAnimationFrameId = 1;
  private animationFrameTimer: NodeJS.Timeout | undefined;
  private keyboardEventTimer: NodeJS.Timeout | undefined;
  private suspendedByPaintCannon = false;
  private readonly captureCtrlC: boolean;
  private readonly captureCtrlZ: boolean;
  private readonly captureMouse: boolean;
  private readonly animationFrameCallbacks = new Map<number, AnimationFrameCallback>();
  private readonly keyboardEventListeners: Record<KeyboardEventType, Set<KeyboardEventListener>> = {
    keydown: new Set(),
    keyup: new Set(),
  };
  private readonly elements = new Map<number, PaintElement>();
  private readonly parents = new Map<number, PaintElement>();
  private readonly clickEventListeners = new Map<number, Set<ClickEventListener>>();
  private readonly handleSigcont = () => {
    if (!this.suspendedByPaintCannon || this.stopped) {
      return;
    }

    this.suspendedByPaintCannon = false;
    this.binding.captureTerminal();
    this.binding.render();
    this.scheduleKeyboardEventPump();
  };

  constructor(options: PaintCannonOptions = {}) {
    this.binding = new native.PaintCannon(
      options.forceCompatMode ?? false,
      options.alternateScreen ?? false,
      options.captureMouse ?? false,
    );
    this.frameIntervalMs = fpsToInterval(options.fps ?? 60);
    this.captureCtrlC = options.captureCtrlC ?? false;
    this.captureCtrlZ = options.captureCtrlZ ?? false;
    this.captureMouse = options.captureMouse ?? false;
    process.on('SIGCONT', this.handleSigcont);
    if (options.syntheticKeyupDelayMs !== undefined) {
      this.setSyntheticKeyupDelay(options.syntheticKeyupDelayMs);
    }
    this.scheduleKeyboardEventPump();
  }

  createElement(tagName: 'div'): DivElement;
  createElement(tagName: 'span'): SpanElement;
  createElement(tagName: string): PaintElement {
    if (tagName === 'div') {
      const element = new DivElement(this, this.binding, this.binding.createDiv());
      this.registerElement(element);
      return element;
    }
    if (tagName === 'span') {
      const element = new SpanElement(this, this.binding, this.binding.createSpan());
      this.registerElement(element);
      return element;
    }

    throw new Error(`paintcannon only supports <div> and <span> right now, got <${tagName}>`);
  }

  createTextNode(data: string): TextNode {
    const text = String(data);
    return new TextNode(this, this.binding, this.binding.createTextNode(text), text);
  }

  setRoot(element: PaintElement): void {
    assertElement(element);
    this.binding.setRoot(element.id);
  }

  get terminalSize(): TerminalSize {
    return this.binding.terminalSize();
  }

  get kittyKeyboardEnabled(): boolean {
    return this.binding.kittyKeyboardEnabled;
  }

  setSyntheticKeyupDelay(delayMs: number): void {
    if (!Number.isFinite(delayMs) || delayMs < 0) {
      throw new Error(`synthetic keyup delay must be a non-negative number, got ${delayMs}`);
    }

    this.binding.setSyntheticKeyupDelay(Math.floor(delayMs));
  }

  setFrameRate(fps: number): void {
    this.frameIntervalMs = fpsToInterval(fps);

    if (this.animationFrameTimer !== undefined) {
      clearTimeout(this.animationFrameTimer);
      this.animationFrameTimer = undefined;
      this.scheduleAnimationFrameTick();
    }
  }

  requestAnimationFrame(callback: AnimationFrameCallback): number {
    if (this.stopped) {
      throw new Error('paintcannon renderer has been stopped');
    }

    const id = this.nextAnimationFrameId++;
    this.animationFrameCallbacks.set(id, callback);
    this.scheduleAnimationFrameTick();
    return id;
  }

  cancelAnimationFrame(id: number): void {
    this.animationFrameCallbacks.delete(id);
  }

  addEventListener(type: KeyboardEventType, listener: KeyboardEventListener): void {
    if (type !== 'keydown' && type !== 'keyup') {
      throw new Error(`unsupported event type: ${type}`);
    }

    if (this.stopped) {
      throw new Error('paintcannon renderer has been stopped');
    }

    this.keyboardEventListeners[type].add(listener);
    this.scheduleKeyboardEventPump();
  }

  removeEventListener(type: KeyboardEventType, listener: KeyboardEventListener): void {
    if (type !== 'keydown' && type !== 'keyup') {
      return;
    }

    this.keyboardEventListeners[type].delete(listener);
    if (!this.shouldPumpInputEvents() && this.keyboardEventTimer !== undefined) {
      clearTimeout(this.keyboardEventTimer);
      this.keyboardEventTimer = undefined;
    }
  }

  addElementEventListener(
    element: PaintElement,
    type: ElementEventType,
    listener: ClickEventListener,
  ): void {
    if (type !== 'click') {
      throw new Error(`unsupported event type: ${type}`);
    }

    if (this.stopped) {
      throw new Error('paintcannon renderer has been stopped');
    }

    let listeners = this.clickEventListeners.get(element.id);
    if (listeners === undefined) {
      listeners = new Set();
      this.clickEventListeners.set(element.id, listeners);
    }
    listeners.add(listener);
    this.scheduleKeyboardEventPump();
  }

  removeElementEventListener(
    element: PaintElement,
    type: ElementEventType,
    listener: ClickEventListener,
  ): void {
    if (type !== 'click') {
      return;
    }

    const listeners = this.clickEventListeners.get(element.id);
    listeners?.delete(listener);
    if (listeners?.size === 0) {
      this.clickEventListeners.delete(element.id);
    }
    if (!this.shouldPumpInputEvents() && this.keyboardEventTimer !== undefined) {
      clearTimeout(this.keyboardEventTimer);
      this.keyboardEventTimer = undefined;
    }
  }

  render(): void {
    if (this.stopped) {
      return;
    }

    this.binding.render();
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
    if (this.keyboardEventTimer !== undefined) {
      clearTimeout(this.keyboardEventTimer);
      this.keyboardEventTimer = undefined;
    }
    this.animationFrameCallbacks.clear();
    this.keyboardEventListeners.keydown.clear();
    this.keyboardEventListeners.keyup.clear();
    this.clickEventListeners.clear();
    this.elements.clear();
    this.parents.clear();
    process.off('SIGCONT', this.handleSigcont);
    this.binding.stop();
  }

  setParent(child: PaintNode, parent: PaintElement): void {
    if (child instanceof DivElement || child instanceof SpanElement) {
      this.parents.set(child.id, parent);
    }
  }

  private scheduleAnimationFrameTick(): void {
    if (this.animationFrameTimer !== undefined || this.animationFrameCallbacks.size === 0) {
      return;
    }

    this.animationFrameTimer = setTimeout(() => {
      this.animationFrameTimer = undefined;
      this.runAnimationFrameTick();
    }, this.frameIntervalMs);
  }

  private runAnimationFrameTick(): void {
    const callbacks = Array.from(this.animationFrameCallbacks.values());
    this.animationFrameCallbacks.clear();

    const timestamp = performance.now();
    for (const callback of callbacks) {
      callback(timestamp);
    }

    if (!this.stopped) {
      this.render();
      this.scheduleAnimationFrameTick();
    }
  }

  private scheduleKeyboardEventPump(): void {
    if (
      this.stopped ||
      this.keyboardEventTimer !== undefined ||
      !this.shouldPumpInputEvents()
    ) {
      return;
    }

    this.keyboardEventTimer = setTimeout(() => {
      this.keyboardEventTimer = undefined;
      this.runKeyboardEventPump();
    }, this.frameIntervalMs);
  }

  private runKeyboardEventPump(): void {
    if (this.stopped) {
      return;
    }

    const events = this.binding.drainKeyboardEvents();
    let handledAnyEvent = false;
    if (events.length > 0) {
      for (const event of events) {
        if (this.handleDefaultControlEvent(event)) {
          return;
        }

        const listeners = Array.from(this.keyboardEventListeners[event.type] ?? []);
        for (const listener of listeners) {
          listener(event);
        }
      }
      handledAnyEvent = true;
    }

    if (this.captureMouse && this.clickListenerCount() > 0) {
      const clicks = this.binding.drainMouseClicks();
      for (const click of clicks) {
        const event = this.binding.clickEventForMouseClick(
          click.x,
          click.y,
          click.button,
          click.ctrlKey,
          click.altKey,
          click.metaKey,
          click.shiftKey,
        );
        if (event !== null) {
          this.dispatchClickEvent(event);
          handledAnyEvent = true;
        }
      }
    }

    if (handledAnyEvent) {
      this.render();
    }

    this.scheduleKeyboardEventPump();
  }

  private keyboardListenerCount(): number {
    return this.keyboardEventListeners.keydown.size + this.keyboardEventListeners.keyup.size;
  }

  private clickListenerCount(): number {
    let count = 0;
    for (const listeners of this.clickEventListeners.values()) {
      count += listeners.size;
    }
    return count;
  }

  private shouldPumpInputEvents(): boolean {
    return (
      this.keyboardListenerCount() > 0 ||
      !this.captureCtrlC ||
      !this.captureCtrlZ ||
      (this.captureMouse && this.clickListenerCount() > 0)
    );
  }

  private handleDefaultControlEvent(event: KeyboardEvent): boolean {
    if (event.type !== 'keydown' || !event.ctrlKey) {
      return false;
    }

    if (!this.captureCtrlC && event.code === 'KeyC') {
      this.stop();
      this.binding.interruptProcessGroup();
      return true;
    }

    if (!this.captureCtrlZ && event.code === 'KeyZ') {
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

  private registerElement(element: PaintElement): void {
    this.elements.set(element.id, element);
  }

  private dispatchClickEvent(nativeEvent: NativeClickEvent): void {
    const target = this.elements.get(nativeEvent.targetId);
    if (target === undefined) {
      return;
    }

    const event = new PaintMouseEvent(nativeEvent, target);
    let currentTarget: PaintElement | undefined = target;
    while (currentTarget !== undefined) {
      event.setCurrentTarget(currentTarget);
      const listeners = Array.from(this.clickEventListeners.get(currentTarget.id) ?? []);
      for (const listener of listeners) {
        listener(event);
        if (event.propagationStopped) {
          return;
        }
      }
      currentTarget = this.parents.get(currentTarget.id);
    }
  }
}

export class PaintMouseEvent {
  readonly type: 'click';
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

  constructor(event: NativeClickEvent, target: PaintElement) {
    this.type = event.type;
    this.target = target;
    this.currentTarget = target;
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

export class DivElement {
  readonly ownerDocument: PaintCannon;
  readonly style: CSSStyleDeclaration;

  constructor(
    owner: PaintCannon,
    private readonly binding: NativePaintCannon,
    readonly id: number,
  ) {
    this.ownerDocument = owner;
    this.style = new CSSStyleDeclaration(binding, id);
  }

  appendChild(child: PaintNode): PaintNode {
    assertPaintNode(child);
    this.binding.appendChild(this.id, child.id);
    this.ownerDocument.setParent(child, this);
    return child;
  }

  addEventListener(type: ElementEventType, listener: ClickEventListener): void {
    this.ownerDocument.addElementEventListener(this, type, listener);
  }

  removeEventListener(type: ElementEventType, listener: ClickEventListener): void {
    this.ownerDocument.removeElementEventListener(this, type, listener);
  }
}

export class SpanElement {
  readonly ownerDocument: PaintCannon;
  readonly style: CSSStyleDeclaration;

  constructor(
    owner: PaintCannon,
    private readonly binding: NativePaintCannon,
    readonly id: number,
  ) {
    this.ownerDocument = owner;
    this.style = new CSSStyleDeclaration(binding, id);
  }

  appendChild(child: PaintNode): PaintNode {
    assertPaintNode(child);
    this.binding.appendChild(this.id, child.id);
    this.ownerDocument.setParent(child, this);
    return child;
  }

  addEventListener(type: ElementEventType, listener: ClickEventListener): void {
    this.ownerDocument.addElementEventListener(this, type, listener);
  }

  removeEventListener(type: ElementEventType, listener: ClickEventListener): void {
    this.ownerDocument.removeElementEventListener(this, type, listener);
  }
}

export class TextNode {
  readonly ownerDocument: PaintCannon;

  constructor(
    owner: PaintCannon,
    private readonly binding: NativePaintCannon,
    readonly id: number,
    private data: string = '',
  ) {
    this.ownerDocument = owner;
  }

  get nodeValue(): string {
    return this.data;
  }

  set nodeValue(value: string) {
    this.data = String(value);
    this.binding.setTextNodeValue(this.id, this.data);
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
    private readonly binding: NativePaintCannon,
    private readonly id: number,
  ) {}

  setProperty(property: string, value: string | number): void {
    const name = normalizeStyleName(property);
    const stringValue = String(value);
    this.values[name] = stringValue;
    this.binding.setStyleProperty(this.id, name, stringValue);
  }

  getPropertyValue(property: string): string {
    return this.values[normalizeStyleName(property)] || '';
  }

  get display(): 'inline' | 'block' | 'flex' | 'flexbox' | 'grid' | string {
    return this.getPropertyValue('display');
  }

  set display(value: 'inline' | 'block' | 'flex' | 'flexbox' | 'grid' | string) {
    this.setProperty('display', value);
  }

  get flexDirection(): 'row' | 'column' | string {
    return this.getPropertyValue('flex-direction');
  }

  set flexDirection(value: 'row' | 'column' | string) {
    this.setProperty('flex-direction', value);
  }

  get flexWrap(): string {
    return this.getPropertyValue('flex-wrap');
  }

  set flexWrap(value: string) {
    this.setProperty('flex-wrap', value);
  }

  get flexFlow(): string {
    return this.getPropertyValue('flex-flow');
  }

  set flexFlow(value: string) {
    this.setProperty('flex-flow', value);
  }

  get flexBasis(): string {
    return this.getPropertyValue('flex-basis');
  }

  set flexBasis(value: string | number) {
    this.setProperty('flex-basis', value);
  }

  get flexGrow(): string {
    return this.getPropertyValue('flex-grow');
  }

  set flexGrow(value: string | number) {
    this.setProperty('flex-grow', value);
  }

  get flexShrink(): string {
    return this.getPropertyValue('flex-shrink');
  }

  set flexShrink(value: string | number) {
    this.setProperty('flex-shrink', value);
  }

  get flex(): string {
    return this.getPropertyValue('flex');
  }

  set flex(value: string | number) {
    this.setProperty('flex', value);
  }

  get justifyContent(): string {
    return this.getPropertyValue('justify-content');
  }

  set justifyContent(value: string) {
    this.setProperty('justify-content', value);
  }

  get alignItems(): string {
    return this.getPropertyValue('align-items');
  }

  set alignItems(value: string) {
    this.setProperty('align-items', value);
  }

  get alignSelf(): string {
    return this.getPropertyValue('align-self');
  }

  set alignSelf(value: string) {
    this.setProperty('align-self', value);
  }

  get alignContent(): string {
    return this.getPropertyValue('align-content');
  }

  set alignContent(value: string) {
    this.setProperty('align-content', value);
  }

  get justifyItems(): string {
    return this.getPropertyValue('justify-items');
  }

  set justifyItems(value: string) {
    this.setProperty('justify-items', value);
  }

  get justifySelf(): string {
    return this.getPropertyValue('justify-self');
  }

  set justifySelf(value: string) {
    this.setProperty('justify-self', value);
  }

  get gap(): string {
    return this.getPropertyValue('gap');
  }

  set gap(value: string | number) {
    this.setProperty('gap', value);
  }

  get rowGap(): string {
    return this.getPropertyValue('row-gap');
  }

  set rowGap(value: string | number) {
    this.setProperty('row-gap', value);
  }

  get columnGap(): string {
    return this.getPropertyValue('column-gap');
  }

  set columnGap(value: string | number) {
    this.setProperty('column-gap', value);
  }

  get width(): string {
    return this.getPropertyValue('width');
  }

  set width(value: string | number) {
    this.setProperty('width', value);
  }

  get height(): string {
    return this.getPropertyValue('height');
  }

  set height(value: string | number) {
    this.setProperty('height', value);
  }

  get backgroundColor(): string {
    return this.getPropertyValue('background-color');
  }

  set backgroundColor(value: string) {
    this.setProperty('background-color', value);
  }

  get gridTemplateColumns(): string {
    return this.getPropertyValue('grid-template-columns');
  }

  set gridTemplateColumns(value: string) {
    this.setProperty('grid-template-columns', value);
  }

  get gridTemplateRows(): string {
    return this.getPropertyValue('grid-template-rows');
  }

  set gridTemplateRows(value: string) {
    this.setProperty('grid-template-rows', value);
  }

  get gridAutoColumns(): string {
    return this.getPropertyValue('grid-auto-columns');
  }

  set gridAutoColumns(value: string) {
    this.setProperty('grid-auto-columns', value);
  }

  get gridAutoRows(): string {
    return this.getPropertyValue('grid-auto-rows');
  }

  set gridAutoRows(value: string) {
    this.setProperty('grid-auto-rows', value);
  }

  get gridAutoFlow(): string {
    return this.getPropertyValue('grid-auto-flow');
  }

  set gridAutoFlow(value: string) {
    this.setProperty('grid-auto-flow', value);
  }

  get gridColumn(): string {
    return this.getPropertyValue('grid-column');
  }

  set gridColumn(value: string) {
    this.setProperty('grid-column', value);
  }

  get gridRow(): string {
    return this.getPropertyValue('grid-row');
  }

  set gridRow(value: string) {
    this.setProperty('grid-row', value);
  }

  get gridColumnStart(): string {
    return this.getPropertyValue('grid-column-start');
  }

  set gridColumnStart(value: string | number) {
    this.setProperty('grid-column-start', value);
  }

  get gridColumnEnd(): string {
    return this.getPropertyValue('grid-column-end');
  }

  set gridColumnEnd(value: string | number) {
    this.setProperty('grid-column-end', value);
  }

  get gridRowStart(): string {
    return this.getPropertyValue('grid-row-start');
  }

  set gridRowStart(value: string | number) {
    this.setProperty('grid-row-start', value);
  }

  get gridRowEnd(): string {
    return this.getPropertyValue('grid-row-end');
  }

  set gridRowEnd(value: string | number) {
    this.setProperty('grid-row-end', value);
  }
}

function assertElement(value: unknown): asserts value is PaintElement {
  if (!(value instanceof DivElement) && !(value instanceof SpanElement)) {
    throw new TypeError('expected a paintcannon element');
  }
}

function assertPaintNode(value: unknown): asserts value is PaintNode {
  if (!(value instanceof DivElement) && !(value instanceof SpanElement) && !(value instanceof TextNode)) {
    throw new TypeError('expected a paintcannon node');
  }
}

function normalizeStyleName(property: string): string {
  return property.replace(/[A-Z]/g, (char) => `-${char.toLowerCase()}`);
}

function fpsToInterval(fps: number): number {
  if (!Number.isFinite(fps) || fps <= 0) {
    throw new RangeError(`fps must be a positive finite number, got ${fps}`);
  }

  return 1000 / fps;
}

function loadNativeBinding(): NativeBinding {
  const candidates = [
    '../paintcannon.node',
    `../paintcannon.${process.platform}-${process.arch}.node`,
    `../paintcannon.${process.platform}-${process.arch}-gnu.node`,
    `../paintcannon.${process.platform}-${process.arch}-musl.node`,
    '../index.node',
  ];

  for (const candidate of candidates) {
    const filename = path.join(__dirname, candidate);
    if (fs.existsSync(filename)) {
      return require(filename) as NativeBinding;
    }
  }

  throw new Error(
    `Could not find paintcannon native binding. Run "npm run build:debug" first. Tried: ${candidates.join(', ')}`
  );
}
