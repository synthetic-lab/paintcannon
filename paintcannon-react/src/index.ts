import fs from 'node:fs';
import path from 'node:path';
import React from 'react';
import createReconciler, {type ReactContext} from 'react-reconciler';
import {
  DefaultEventPriority,
  NoEventPriority,
} from 'react-reconciler/constants';
import * as Scheduler from 'scheduler';
import type {
  CSSStyleDeclaration,
  DivElement,
  ElementEventType,
  FocusElementEventType,
  FocusEventListener,
  InputElement,
  KeyboardEventListener,
  KeyboardEventType,
  MouseElementEventType,
  MouseEventListener,
  PaintCannonOptions,
  PaintElement,
  PaintFocusEvent,
  PaintKeyboardEvent,
  PaintMouseEvent,
  PaintNode,
  PaintScrollEvent,
  ScrollEventListener,
  SpanElement,
  TextAreaElement,
  TextNode,
  TransitionElementEventType,
  TransitionEventListener,
} from 'paintcannon';
import { ELEMENT_EVENT_TYPES, KEYBOARD_EVENT_TYPES, PaintCannon } from 'paintcannon';

type HostType = 'paintcannon.div' | 'paintcannon.span' | 'paintcannon.input' | 'paintcannon.textarea';
type HostNode = HostElement | HostText;
type HostParent = HostElement | RootContainer;
type StyleValue = string | number | boolean | null | undefined;
type StyleProps = Partial<Record<keyof CSSStyleDeclaration, StyleValue>> & Record<string, StyleValue>;
type EventPropName<T extends ElementEventType = ElementEventType> =
  T extends `mouse${infer Rest}` ? `onMouse${Capitalize<Rest>}` :
  T extends `transition${infer Rest}` ? `onTransition${Capitalize<Rest>}` :
  `on${Capitalize<T>}`;
type ElementEventListenerFor<T extends ElementEventType> =
  T extends MouseElementEventType ? MouseEventListener :
  T extends FocusElementEventType ? FocusEventListener :
  T extends TransitionElementEventType ? TransitionEventListener :
  T extends 'scroll' ? ScrollEventListener :
  never;
type ElementEventProps = {
  [T in ElementEventType as EventPropName<T>]?: ElementEventListenerFor<T>;
};
type KeyboardEventPropName<T extends KeyboardEventType = KeyboardEventType> =
  T extends 'keydown' ? 'onKeyDown' :
  T extends 'keyup' ? 'onKeyUp' :
  never;
type KeyboardEventProps = {
  [T in KeyboardEventType as KeyboardEventPropName<T>]?: KeyboardEventListener;
};

export interface CommonProps extends ElementEventProps, KeyboardEventProps {
  children?: React.ReactNode;
  style?: StyleProps;
}

export interface DivProps extends CommonProps {
  scrollLeft?: number;
  scrollTop?: number;
}

export interface SpanProps extends CommonProps {
  scrollLeft?: number;
  scrollTop?: number;
}

export interface InputProps extends CommonProps {
  type?: 'text';
  value?: string;
  placeholder?: string;
  cursorPosition?: number;
  autoFocus?: boolean;
}

export interface TextareaProps extends InputProps {
  type?: never;
  scrollLeft?: number;
  scrollTop?: number;
}

export const Div = 'paintcannon.div' as unknown as React.ComponentType<DivProps>;
export const Span = 'paintcannon.span' as unknown as React.ComponentType<SpanProps>;
export const Input = 'paintcannon.input' as unknown as React.ComponentType<InputProps>;
export const Textarea = 'paintcannon.textarea' as unknown as React.ComponentType<TextareaProps>;

export interface CreateRootOptions extends PaintCannonOptions {
  paintCannon?: PaintCannon;
  container?: DivElement | SpanElement;
}

export interface PaintCannonReactRoot {
  readonly paintCannon: PaintCannon;
  readonly container: DivElement | SpanElement;
  render(element: React.ReactNode): void;
  unmount(): void;
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
}

export interface PaintCannonReactApp {
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
}

interface HostElement {
  kind: 'element';
  type: HostType;
  props: Props;
  children: HostNode[];
  node: PaintElement;
}

interface HostText {
  kind: 'text';
  text: string;
  node: TextNode;
}

interface RootContainer {
  paintCannon: PaintCannon;
  root: DivElement | SpanElement;
  children: HostNode[];
}

type Props = Record<string, unknown>;
type PackageInfo = {
  name: string;
  version: string;
};
type AnimationCallback = (currentTime: number) => void;
type AnimationSubscription = {
  startTime: number;
  unsubscribe(): void;
};
type AnimationContextValue = {
  subscribe(callback: AnimationCallback, interval: number | undefined): AnimationSubscription;
};

export interface AnimationOptions {
  interval?: number;
  isActive?: boolean;
}

export interface AnimationResult {
  readonly frame: number;
  readonly time: number;
  readonly delta: number;
  readonly reset: () => void;
}

let currentUpdatePriority = NoEventPriority;
const packageInfo = loadPackageInfo();
const AppContext = React.createContext<PaintCannonReactApp | undefined>(undefined);
const AnimationContext = React.createContext<AnimationContextValue | undefined>(undefined);
const maximumTimerInterval = 2_147_483_647;
const zeroAnimationState: Omit<AnimationResult, 'reset'> = { frame: 0, time: 0, delta: 0 };

const reconciler = createReconciler({
  getRootHostContext: () => ({}),
  getChildHostContext: () => ({}),
  prepareForCommit(container: RootContainer) {
    container.paintCannon.beginTransaction();
    return null;
  },
  resetAfterCommit(container: RootContainer) {
    container.paintCannon.commitTransaction();
    container.paintCannon.render();
  },
  preparePortalMount: () => null,
  clearContainer(container: RootContainer) {
    for (const child of container.children) {
      destroyHostNode(child);
    }
    container.children = [];
    return false;
  },
  shouldSetTextContent: () => false,
  createInstance(type: HostType, props: Props, container: RootContainer) {
    const node = createPaintElement(container.paintCannon, type);
    applyProps(node, type, {}, props);
    return {
      kind: 'element',
      type,
      props,
      children: [],
      node,
    } satisfies HostElement;
  },
  createTextInstance(text: string, container: RootContainer) {
    return {
      kind: 'text',
      text,
      node: container.paintCannon.createTextNode(text),
    } satisfies HostText;
  },
  appendInitialChild(parent: HostElement, child: HostNode) {
    appendVirtualChild(parent, child);
    appendPaintChild(parent.node, child.node);
  },
  appendChild(parent: HostElement, child: HostNode) {
    appendVirtualChild(parent, child);
    appendPaintChild(parent.node, child.node);
  },
  appendChildToContainer(container: RootContainer, child: HostNode) {
    appendVirtualChild(container, child);
    container.root.appendChild(child.node);
  },
  insertBefore(parent: HostElement, child: HostNode, before: HostNode) {
    insertVirtualChild(parent, child, before);
    insertPaintChild(parent.node, child.node, before.node);
  },
  insertInContainerBefore(container: RootContainer, child: HostNode, before: HostNode) {
    insertVirtualChild(container, child, before);
    container.root.insertBefore(child.node, before.node);
  },
  removeChild(parent: HostElement, child: HostNode) {
    removeVirtualChild(parent, child);
    destroyHostNode(child);
  },
  removeChildFromContainer(container: RootContainer, child: HostNode) {
    removeVirtualChild(container, child);
    destroyHostNode(child);
  },
  finalizeInitialChildren: () => false,
  resetTextContent: () => {},
  getPublicInstance(instance: HostNode) {
    return instance.node;
  },
  commitUpdate(instance: HostElement, _type: HostType, oldProps: Props, newProps: Props) {
    instance.props = newProps;
    applyProps(instance.node, instance.type, oldProps, newProps);
  },
  commitTextUpdate(instance: HostText, _oldText: string, newText: string) {
    instance.text = newText;
    instance.node.nodeValue = newText;
  },
  hideInstance(instance: HostElement) {
    instance.node.style.display = 'none';
  },
  unhideInstance(instance: HostElement) {
    instance.node.style.display = defaultDisplay(instance.type);
  },
  hideTextInstance(instance: HostText) {
    instance.node.nodeValue = '';
  },
  unhideTextInstance(instance: HostText, text: string) {
    instance.node.nodeValue = text;
  },
  detachDeletedInstance: () => {},
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  getInstanceFromNode: () => null,
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,
  isPrimaryRenderer: false,
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  supportsMicrotasks: true,
  scheduleMicrotask: queueMicrotask,
  // @ts-expect-error @types/react-reconciler omits scheduler integration hooks.
  scheduleCallback: Scheduler.unstable_scheduleCallback,
  cancelCallback: Scheduler.unstable_cancelCallback,
  shouldYield: Scheduler.unstable_shouldYield,
  now: Scheduler.unstable_now,
  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,
  setCurrentUpdatePriority(priority: number) {
    currentUpdatePriority = priority;
  },
  getCurrentUpdatePriority: () => currentUpdatePriority,
  resolveUpdatePriority() {
    return currentUpdatePriority !== NoEventPriority ? currentUpdatePriority : DefaultEventPriority;
  },
  maySuspendCommit: () => false,
  NotPendingTransition: undefined,
  HostTransitionContext: React.createContext(null) as unknown as ReactContext<undefined>,
  resetFormInstance: () => {},
  requestPostPaintCallback: () => {},
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => -1.1,
  preloadInstance: () => true,
  startSuspendingCommit: () => {},
  suspendInstance: () => {},
  waitForCommitToBeReady: () => null,
  rendererPackageName: packageInfo.name,
  rendererVersion: packageInfo.version,
});

export function createRoot(options: CreateRootOptions = {}): PaintCannonReactRoot {
  const paintCannon = options.paintCannon ?? new PaintCannon(options);
  const container = options.container ?? paintCannon.createElement('div');
  if (options.container === undefined) {
    container.style.width = '100%';
    container.style.height = '100%';
  }
  paintCannon.setRoot(container);
  const rootContainer: RootContainer = {
    paintCannon,
    root: container,
    children: [],
  };
  const animationScheduler = new AnimationScheduler(paintCannon);
  const reactRoot = reconciler.createContainer(
    rootContainer,
    0,
    null,
    false,
    null,
    '',
    reportReactError,
    reportReactError,
    reportReactError,
    () => {},
  );
  let exited = false;
  let resolveExit: (value: unknown) => void;
  let rejectExit: (error: Error) => void;
  const exitPromise = new Promise<unknown>((resolve, reject) => {
    resolveExit = resolve;
    rejectExit = reject;
  });
  const settleExit = (errorOrResult?: unknown): void => {
    if (errorOrResult instanceof Error) {
      rejectExit(errorOrResult);
      return;
    }
    resolveExit(errorOrResult);
  };
  const app: PaintCannonReactApp = {
    exit(errorOrResult?: unknown): void {
      if (exited) {
        return;
      }

      exited = true;
      reconciler.updateContainer(null, reactRoot, null, () => {
        animationScheduler.stop();
        paintCannon.stop();
        settleExit(errorOrResult);
      });
    },
    waitUntilExit(): Promise<unknown> {
      return exitPromise;
    },
  };

  return {
    paintCannon,
    container,
    render(element: React.ReactNode): void {
      if (exited) {
        throw new Error('paintcannon-react root has exited');
      }

      reconciler.updateContainer(
        React.createElement(
          AppContext.Provider,
          { value: app },
          React.createElement(
            AnimationContext.Provider,
            { value: animationScheduler },
            element,
          ),
        ),
        reactRoot,
        null,
        null,
      );
    },
    unmount(): void {
      app.exit();
    },
    exit(errorOrResult?: unknown): void {
      app.exit(errorOrResult);
    },
    waitUntilExit(): Promise<unknown> {
      return app.waitUntilExit();
    },
  };
}

export function render(element: React.ReactNode, options: CreateRootOptions = {}): PaintCannonReactRoot {
  const root = createRoot(options);
  root.render(element);
  return root;
}

export function useApp(): PaintCannonReactApp {
  const app = React.useContext(AppContext);
  if (app === undefined) {
    throw new Error('useApp() must be used inside a paintcannon-react render tree');
  }
  return app;
}

export function useAnimation(options: AnimationOptions = {}): AnimationResult {
  const { interval, isActive = true } = options;
  const safeInterval = interval === undefined ? undefined : normalizeAnimationInterval(interval);
  const animation = React.useContext(AnimationContext);
  if (animation === undefined) {
    throw new Error('useAnimation() must be used inside a paintcannon-react render tree');
  }

  const [resetKey, setResetKey] = React.useState(0);
  const [state, setState] = React.useState(zeroAnimationState);
  const lastRenderTimeRef = React.useRef(0);
  const previousOptionsRef = React.useRef({ isActive, safeInterval, resetKey });
  const previousOptions = previousOptionsRef.current;
  const shouldReset = isActive && (
    safeInterval !== previousOptions.safeInterval ||
    !previousOptions.isActive ||
    resetKey !== previousOptions.resetKey
  );
  const reset = React.useCallback(() => {
    setResetKey(value => value + 1);
  }, []);

  React.useLayoutEffect(() => {
    if (!isActive) {
      return undefined;
    }

    setState(zeroAnimationState);
    let startTime = 0;
    const subscription = animation.subscribe((currentTime) => {
      const elapsed = currentTime - startTime;
      const delta = currentTime - lastRenderTimeRef.current;
      lastRenderTimeRef.current = currentTime;
      setState(previous => ({
        frame: safeInterval === undefined ? previous.frame + 1 : Math.floor(elapsed / safeInterval),
        time: elapsed,
        delta,
      }));
    }, safeInterval);

    startTime = subscription.startTime;
    lastRenderTimeRef.current = subscription.startTime;
    return subscription.unsubscribe;
  }, [animation, isActive, safeInterval, resetKey]);

  React.useLayoutEffect(() => {
    previousOptionsRef.current = { isActive, safeInterval, resetKey };
  }, [isActive, safeInterval, resetKey]);

  if (shouldReset) {
    return { ...zeroAnimationState, reset };
  }

  return { ...state, reset };
}

class AnimationScheduler implements AnimationContextValue {
  private nextId = 1;
  private animationFrameId: number | undefined;
  private readonly subscribers = new Map<number, {
    callback: AnimationCallback;
    interval: number | undefined;
    nextTime: number;
  }>();

  subscribe(callback: AnimationCallback, interval: number | undefined): AnimationSubscription {
    const id = this.nextId;
    this.nextId += 1;
    const startTime = performance.now();
    this.subscribers.set(id, {
      callback,
      interval,
      nextTime: interval === undefined ? startTime : startTime + interval,
    });
    this.scheduleAnimationFrame();

    return {
      startTime,
      unsubscribe: () => {
        this.subscribers.delete(id);
        if (this.subscribers.size === 0) {
          this.cancelAnimationFrame();
        }
      },
    };
  }

  constructor(private readonly paintCannon: PaintCannon) {}

  stop(): void {
    this.cancelAnimationFrame();
    this.subscribers.clear();
  }

  private scheduleAnimationFrame(): void {
    if (this.animationFrameId !== undefined || this.subscribers.size === 0) {
      return;
    }

    this.animationFrameId = this.paintCannon.requestAnimationFrame((timestamp) => {
      this.animationFrameId = undefined;
      this.tick(timestamp);
    });
  }

  private cancelAnimationFrame(): void {
    if (this.animationFrameId === undefined) {
      return;
    }

    this.paintCannon.cancelAnimationFrame(this.animationFrameId);
    this.animationFrameId = undefined;
  }

  private tick(currentTime: number): void {
    for (const subscriber of this.subscribers.values()) {
      if (subscriber.interval === undefined) {
        subscriber.callback(currentTime);
        continue;
      }

      if (subscriber.nextTime <= currentTime) {
        const intervalsElapsed = Math.max(1, Math.floor((currentTime - subscriber.nextTime) / subscriber.interval) + 1);
        subscriber.nextTime += subscriber.interval * intervalsElapsed;
        subscriber.callback(currentTime);
      }
    }
    this.scheduleAnimationFrame();
  }
}

function normalizeAnimationInterval(interval: number): number {
  if (!Number.isFinite(interval)) {
    return maximumTimerInterval;
  }

  return Math.min(maximumTimerInterval, Math.max(1, interval));
}

function appendVirtualChild(parent: HostParent, child: HostNode): void {
  removeVirtualChild(parent, child);
  parent.children.push(child);
}

function insertVirtualChild(parent: HostParent, child: HostNode, before: HostNode): void {
  removeVirtualChild(parent, child);
  const index = parent.children.indexOf(before);
  if (index === -1) {
    parent.children.push(child);
  } else {
    parent.children.splice(index, 0, child);
  }
}

function removeVirtualChild(parent: HostParent, child: HostNode): void {
  const index = parent.children.indexOf(child);
  if (index !== -1) {
    parent.children.splice(index, 1);
  }
}

function createPaintElement(paintCannon: PaintCannon, type: HostType): PaintElement {
  switch (type) {
    case 'paintcannon.div':
      return paintCannon.createElement('div');
    case 'paintcannon.span':
      return paintCannon.createElement('span');
    case 'paintcannon.input':
      return paintCannon.createElement('input');
    case 'paintcannon.textarea':
      return paintCannon.createElement('textarea');
  }
}

function appendPaintChild(parent: PaintElement, child: PaintNode): void {
  if (!('appendChild' in parent)) {
    throw new Error('Input and Textarea cannot have children');
  }
  parent.appendChild(child);
}

function insertPaintChild(parent: PaintElement, child: PaintNode, before: PaintNode): void {
  if (!('insertBefore' in parent)) {
    throw new Error('Input and Textarea cannot have children');
  }
  parent.insertBefore(child, before);
}

function destroyHostNode(host: HostNode): void {
  if (host.kind === 'element') {
    removeKeyboardEvents(host.node.ownerDocument, host.props);
  }
  host.node.destroy();
}

function applyProps(node: PaintElement, type: HostType, oldProps: Props, newProps: Props): void {
  applyStyle(node, oldProps.style as StyleProps | undefined, newProps.style as StyleProps | undefined);
  applyEvents(node, oldProps, newProps);

  if (type === 'paintcannon.input' || type === 'paintcannon.textarea') {
    const input = node as InputElement | TextAreaElement;
    if (typeof newProps.type === 'string' && type === 'paintcannon.input') {
      input.type = newProps.type;
    }
    if (newProps.value !== undefined) {
      input.value = String(newProps.value);
    }
    if (newProps.placeholder !== undefined) {
      input.placeholder = String(newProps.placeholder);
    }
    if (typeof newProps.cursorPosition === 'number') {
      input.cursorPosition = newProps.cursorPosition;
    }
    if (newProps.autoFocus === true) {
      input.focus();
    }
  }

  if ('scrollLeft' in newProps && typeof newProps.scrollLeft === 'number') {
    (node as DivElement | SpanElement | TextAreaElement).scrollLeft = newProps.scrollLeft;
  }
  if ('scrollTop' in newProps && typeof newProps.scrollTop === 'number') {
    (node as DivElement | SpanElement | TextAreaElement).scrollTop = newProps.scrollTop;
  }
}

function applyStyle(node: PaintElement, oldStyle: StyleProps | undefined, newStyle: StyleProps | undefined): void {
  if (oldStyle === newStyle || newStyle === undefined) {
    return;
  }

  for (const [key, value] of Object.entries(newStyle)) {
    if (value !== undefined && value !== null && value !== oldStyle?.[key]) {
      node.style.setProperty(key, String(value));
    }
  }
}

function applyEvents(node: PaintElement, oldProps: Props, newProps: Props): void {
  applyKeyboardEvents(node.ownerDocument, oldProps, newProps);

  for (const [prop, eventType] of eventProps) {
    const previous = oldProps[prop] as ((event: unknown) => void) | undefined;
    const next = newProps[prop] as ((event: unknown) => void) | undefined;
    if (previous === next) {
      continue;
    }
    if (previous !== undefined) {
      removeElementListener(node, eventType, previous);
    }
    if (next !== undefined) {
      addElementListener(node, eventType, next);
    }
  }
}

function applyKeyboardEvents(paintCannon: PaintCannon, oldProps: Props, newProps: Props): void {
  for (const [prop, eventType] of keyboardEventProps) {
    const previous = oldProps[prop] as KeyboardEventListener | undefined;
    const next = newProps[prop] as KeyboardEventListener | undefined;
    if (previous === next) {
      continue;
    }
    if (previous !== undefined) {
      paintCannon.removeEventListener(eventType, previous);
    }
    if (next !== undefined) {
      paintCannon.addEventListener(eventType, next);
    }
  }
}

function removeKeyboardEvents(paintCannon: PaintCannon, props: Props): void {
  for (const [prop, eventType] of keyboardEventProps) {
    const listener = props[prop] as KeyboardEventListener | undefined;
    if (listener !== undefined) {
      paintCannon.removeEventListener(eventType, listener);
    }
  }
}

function addElementListener(node: PaintElement, eventType: ElementEventType, listener: (event: unknown) => void): void {
  (node as {addEventListener(type: ElementEventType, listener: (event: unknown) => void): void})
    .addEventListener(eventType, listener);
}

function removeElementListener(node: PaintElement, eventType: ElementEventType, listener: (event: unknown) => void): void {
  (node as {removeEventListener(type: ElementEventType, listener: (event: unknown) => void): void})
    .removeEventListener(eventType, listener);
}

function defaultDisplay(type: HostType): string {
  return type === 'paintcannon.span' ? 'inline' : 'block';
}

function reportReactError(error: unknown): void {
  if (error !== null && error !== undefined) {
    console.error(error);
  }
}

function loadPackageInfo(): PackageInfo {
  const packageJsonPath = path.join(__dirname, '..', '..', 'package.json');
  const packageJson = fs.readFileSync(packageJsonPath, 'utf8');
  const parsed = JSON.parse(packageJson) as Partial<PackageInfo> | undefined;

  if (typeof parsed?.name !== 'string' || typeof parsed.version !== 'string') {
    throw new Error(`Invalid package metadata in ${packageJsonPath}`);
  }

  return {
    name: parsed.name,
    version: parsed.version,
  };
}

const eventProps = [
  ...ELEMENT_EVENT_TYPES.map((eventType) => [eventPropName(eventType), eventType] as const),
] satisfies ReadonlyArray<readonly [EventPropName, ElementEventType]>;

const keyboardEventProps = [
  ...KEYBOARD_EVENT_TYPES.map((eventType) => [keyboardEventPropName(eventType), eventType] as const),
] satisfies ReadonlyArray<readonly [KeyboardEventPropName, KeyboardEventType]>;

function eventPropName<T extends ElementEventType>(eventType: T): EventPropName<T> {
  if (eventType.startsWith('mouse')) {
    return `onMouse${capitalize(eventType.slice('mouse'.length))}` as EventPropName<T>;
  }
  if (eventType.startsWith('transition')) {
    return `onTransition${capitalize(eventType.slice('transition'.length))}` as EventPropName<T>;
  }
  return `on${capitalize(eventType)}` as EventPropName<T>;
}

function capitalize(value: string): string {
  return `${value[0]?.toUpperCase() ?? ''}${value.slice(1)}`;
}

function keyboardEventPropName<T extends KeyboardEventType>(eventType: T): KeyboardEventPropName<T> {
  if (eventType === 'keydown') {
    return 'onKeyDown' as KeyboardEventPropName<T>;
  }
  return 'onKeyUp' as KeyboardEventPropName<T>;
}

export type {
  PaintCannon,
  PaintCannonOptions,
  PaintElement,
  PaintFocusEvent,
  PaintKeyboardEvent,
  PaintMouseEvent,
  PaintScrollEvent,
};
