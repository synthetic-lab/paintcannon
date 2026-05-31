import fs = require('node:fs');
import path = require('node:path');
import { performance } from 'node:perf_hooks';

export interface PaintCannonOptions {
  fps?: number;
}

export interface TerminalSize {
  cols: number;
  rows: number;
}

export type AnimationFrameCallback = (timestamp: number) => void;

export interface NativePaintCannon {
  createDiv(): number;
  createTextNode(text: string): number;
  setTextNodeValue(id: number, text: string): void;
  setRoot(id: number): void;
  appendChild(parent: number, child: number): void;
  setStyleProperty(id: number, property: string, value: string): void;
  terminalSize(): TerminalSize;
  render(): void;
  stop(): void;
}

export interface NativeBinding {
  PaintCannon: new () => NativePaintCannon;
}

export type PaintNode = DivElement | TextNode;

export const native: NativeBinding = loadNativeBinding();

export class PaintCannon {
  private readonly binding: NativePaintCannon;
  private frameIntervalMs: number;
  private stopped = false;
  private nextAnimationFrameId = 1;
  private animationFrameTimer: NodeJS.Timeout | undefined;
  private readonly animationFrameCallbacks = new Map<number, AnimationFrameCallback>();

  constructor(options: PaintCannonOptions = {}) {
    this.binding = new native.PaintCannon();
    this.frameIntervalMs = fpsToInterval(options.fps ?? 60);
  }

  createElement(tagName: 'div'): DivElement;
  createElement(tagName: string): DivElement {
    if (tagName !== 'div') {
      throw new Error(`paintcannon only supports <div> right now, got <${tagName}>`);
    }

    return new DivElement(this, this.binding, this.binding.createDiv());
  }

  createTextNode(data: string): TextNode {
    const text = String(data);
    return new TextNode(this, this.binding, this.binding.createTextNode(text), text);
  }

  setRoot(element: DivElement): void {
    assertElement(element);
    this.binding.setRoot(element.id);
  }

  get terminalSize(): TerminalSize {
    return this.binding.terminalSize();
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
    this.animationFrameCallbacks.clear();
    this.binding.stop();
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
    return child;
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

  get display(): 'block' | 'flex' | 'grid' | string {
    return this.getPropertyValue('display');
  }

  set display(value: 'block' | 'flex' | 'grid' | string) {
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

function assertElement(value: unknown): asserts value is DivElement {
  if (!(value instanceof DivElement)) {
    throw new TypeError('expected a paintcannon div element');
  }
}

function assertPaintNode(value: unknown): asserts value is PaintNode {
  if (!(value instanceof DivElement) && !(value instanceof TextNode)) {
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
