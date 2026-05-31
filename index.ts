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

  appendChild(child: DivElement): DivElement {
    assertElement(child);
    this.binding.appendChild(this.id, child.id);
    return child;
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
}

function assertElement(value: unknown): asserts value is DivElement {
  if (!(value instanceof DivElement)) {
    throw new TypeError('expected a paintcannon div element');
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
