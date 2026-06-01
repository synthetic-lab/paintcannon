import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import React from 'react';
import createReconciler, {type ReactContext} from 'react-reconciler';
import {
  DefaultEventPriority,
  NoEventPriority,
} from 'react-reconciler/constants.js';
import * as Scheduler from 'scheduler';
import type {
  CSSStyleProperties,
  CSSStylePropertyName,
  ElementEventType,
  DivElement as PaintDivElement,
  FormElement as PaintFormElement,
  PaintCannonOptions,
  PaintChangeEvent,
  PaintElement,
  PaintFocusEvent,
  PaintKeyboardEvent,
  PaintMouseEvent,
  PaintNode,
  PaintScrollEvent,
  PaintSubmitEvent,
  SpanElement as PaintSpanElement,
  TextNode,
} from 'paintcannon';
import {
  ELEMENT_EVENT_TYPES,
  PaintCannon,
} from 'paintcannon';
import * as hostComponents from './host-components/index.ts';

export type {CSSStyleProperties, CSSStylePropertyName, CSSStyleValue} from 'paintcannon';

type HostType = hostComponents.HostType;
type HostProps =
  | hostComponents.div.Props
  | hostComponents.span.Props
  | hostComponents.form.Props
  | hostComponents.button.Props
  | hostComponents.input.Props
  | hostComponents.textarea.Props;
type HostNode = HostElement | HostText;
type HostParent = HostElement | RootContainer;
type ChildContainerElement = PaintElement & {
  appendChild(child: PaintNode): PaintNode;
  insertBefore(child: PaintNode, before: PaintNode): PaintNode;
};
type MountedHostElement<Type extends HostType, Props, Element extends PaintElement> = {
  kind: 'element';
  type: Type;
  props: Props;
  children: HostNode[];
  node: Element;
};
type MountedComponent<Module> =
  Module extends {
    type: infer Type extends HostType;
    Component: hostComponents.HostComponent<infer Props, infer Element extends PaintElement>;
  }
    ? MountedHostElement<Type, Props, Element>
    : never;

export interface CreateRootOptions extends PaintCannonOptions {
  paintCannon?: PaintCannon;
  container?: PaintDivElement | PaintSpanElement | PaintFormElement;
}

export interface PaintCannonReactRoot {
  readonly paintCannon: PaintCannon;
  readonly container: PaintDivElement | PaintSpanElement | PaintFormElement;
  render(element: React.ReactNode): void;
  unmount(): void;
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
}

export interface PaintCannonReactApp {
  readonly paintCannon: PaintCannon;
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
}

type HostElement =
  | MountedComponent<typeof hostComponents.div>
  | MountedComponent<typeof hostComponents.span>
  | MountedComponent<typeof hostComponents.form>
  | MountedComponent<typeof hostComponents.button>
  | MountedComponent<typeof hostComponents.input>
  | MountedComponent<typeof hostComponents.textarea>;

const hostPropsForType = {
  [hostComponents.div.type]: undefined as unknown as hostComponents.div.Props,
  [hostComponents.span.type]: undefined as unknown as hostComponents.span.Props,
  [hostComponents.form.type]: undefined as unknown as hostComponents.form.Props,
  [hostComponents.button.type]: undefined as unknown as hostComponents.button.Props,
  [hostComponents.input.type]: undefined as unknown as hostComponents.input.Props,
  [hostComponents.textarea.type]: undefined as unknown as hostComponents.textarea.Props,
} satisfies {[K in HostType]: HostProps};
type HostPropsForType = typeof hostPropsForType;

export type DivProps = hostComponents.div.Props;
export type DivElement = hostComponents.div.Element;
export const Div = hostComponents.div.Component;
export type SpanProps = hostComponents.span.Props;
export type SpanElement = hostComponents.span.Element;
export const Span = hostComponents.span.Component;
export type FormProps = hostComponents.form.Props;
export type FormElement = hostComponents.form.Element;
export const Form = hostComponents.form.Component;
export type ButtonProps = hostComponents.button.Props;
export type ButtonElement = hostComponents.button.Element;
export const Button = hostComponents.button.Component;
export type InputProps = hostComponents.input.Props;
export type InputElement = hostComponents.input.Element;
export const Input = hostComponents.input.Component;
export type TextareaProps = hostComponents.textarea.Props;
export type TextareaElement = hostComponents.textarea.Element;
export const Textarea = hostComponents.textarea.Component;

interface HostText {
  kind: 'text';
  text: string;
  node: TextNode;
}

interface RootContainer {
  paintCannon: PaintCannon;
  root: PaintDivElement | PaintSpanElement | PaintFormElement;
  children: HostNode[];
}

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
const sourceDirectory = path.dirname(fileURLToPath(import.meta.url));
const packageInfo = loadPackageInfo();
const AppContext = React.createContext<PaintCannonReactApp | undefined>(undefined);
const AnimationContext = React.createContext<AnimationContextValue | undefined>(undefined);
const maximumTimerInterval = 2_147_483_647;
const zeroAnimationState: Omit<AnimationResult, 'reset'> = { frame: 0, time: 0, delta: 0 };
const eventListenerWrappers = new WeakMap<(event: unknown) => void, (event: unknown) => void>();

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
  createInstance(type: HostType, props: HostProps, container: RootContainer) {
    return createHostElement(container.paintCannon, type, props);
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
  commitUpdate(instance: HostElement, _type: HostType, _oldProps: HostProps, newProps: HostProps) {
    instance.props = applyHostElementProps(instance, newProps);
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
    paintCannon,
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

function createHostElement(paintCannon: PaintCannon, type: HostType, props: HostProps): HostElement {
  if (type === hostComponents.div.type) {
    return createDivElement(paintCannon, castHostProps(type, props));
  }
  if (type === hostComponents.span.type) {
    return createSpanElement(paintCannon, castHostProps(type, props));
  }
  if (type === hostComponents.form.type) {
    return createFormElement(paintCannon, castHostProps(type, props));
  }
  if (type === hostComponents.button.type) {
    return createButtonElement(paintCannon, castHostProps(type, props));
  }
  if (type === hostComponents.input.type) {
    return createInputElement(paintCannon, castHostProps(type, props));
  }
  if (type === hostComponents.textarea.type) {
    return createTextareaElement(paintCannon, castHostProps(type, props));
  }

  const unknownType: never = type;
  throw new Error(`Unknown host type: ${String(unknownType)}`);
}

function createDivElement(
  paintCannon: PaintCannon,
  props: hostComponents.div.Props,
): MountedComponent<typeof hostComponents.div> {
  const type = hostComponents.div.type;
  const node = paintCannon.createElement('div');
  applyDivProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function createSpanElement(
  paintCannon: PaintCannon,
  props: hostComponents.span.Props,
): MountedComponent<typeof hostComponents.span> {
  const type = hostComponents.span.type;
  const node = paintCannon.createElement('span');
  applySpanProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function createFormElement(
  paintCannon: PaintCannon,
  props: hostComponents.form.Props,
): MountedComponent<typeof hostComponents.form> {
  const type = hostComponents.form.type;
  const node = paintCannon.createElement('form');
  applyFormProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function createButtonElement(
  paintCannon: PaintCannon,
  props: hostComponents.button.Props,
): MountedComponent<typeof hostComponents.button> {
  const type = hostComponents.button.type;
  const node = paintCannon.createElement('button');
  applyButtonProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function createInputElement(
  paintCannon: PaintCannon,
  props: hostComponents.input.Props,
): MountedComponent<typeof hostComponents.input> {
  const type = hostComponents.input.type;
  const node = paintCannon.createElement('input');
  applyInputProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function createTextareaElement(
  paintCannon: PaintCannon,
  props: hostComponents.textarea.Props,
): MountedComponent<typeof hostComponents.textarea> {
  const type = hostComponents.textarea.type;
  const node = paintCannon.createElement('textarea');
  applyTextareaProps(node, {}, props);
  return { kind: 'element', type, props, children: [], node };
}

function appendPaintChild(parent: PaintElement, child: PaintNode): void {
  if (!canHaveChildren(parent)) {
    throw new Error('Input and Textarea cannot have children');
  }
  parent.appendChild(child);
}

function insertPaintChild(parent: PaintElement, child: PaintNode, before: PaintNode): void {
  if (!canHaveChildren(parent)) {
    throw new Error('Input and Textarea cannot have children');
  }
  parent.insertBefore(child, before);
}

function destroyHostNode(host: HostNode): void {
  host.node.destroy();
}

function applyHostElementProps(host: HostElement, newProps: HostProps): HostProps {
  if (host.type === hostComponents.div.type) {
    const props = castHostProps(host.type, newProps);
    applyDivProps(host.node, host.props, props);
    return props;
  }
  if (host.type === hostComponents.span.type) {
    const props = castHostProps(host.type, newProps);
    applySpanProps(host.node, host.props, props);
    return props;
  }
  if (host.type === hostComponents.form.type) {
    const props = castHostProps(host.type, newProps);
    applyFormProps(host.node, host.props, props);
    return props;
  }
  if (host.type === hostComponents.button.type) {
    const props = castHostProps(host.type, newProps);
    applyButtonProps(host.node, host.props, props);
    return props;
  }
  if (host.type === hostComponents.input.type) {
    const props = castHostProps(host.type, newProps);
    applyInputProps(host.node, host.props, props);
    return props;
  }
  if (host.type === hostComponents.textarea.type) {
    const props = castHostProps(host.type, newProps);
    applyTextareaProps(host.node, host.props, props);
    return props;
  }

  const unknownHost: never = host;
  throw new Error(`Unknown host type: ${String(unknownHost)}`);
}

function castHostProps<Type extends HostType>(_type: Type, props: HostProps): HostPropsForType[Type] {
  return props as HostPropsForType[Type];
}

function applyCommonProps(
  node: PaintElement,
  oldProps: Partial<hostComponents.CommonProps>,
  newProps: hostComponents.CommonProps,
): void {
  applyStyle(node, oldProps.style, newProps.style);
  applyEvents(node, oldProps, newProps);
}

function applyDivProps(
  node: hostComponents.div.Element,
  oldProps: Partial<hostComponents.div.Props>,
  newProps: hostComponents.div.Props,
): void {
  applyScrollableElementProps(node, oldProps, newProps);
}

function applySpanProps(
  node: hostComponents.span.Element,
  oldProps: Partial<hostComponents.span.Props>,
  newProps: hostComponents.span.Props,
): void {
  applyScrollableElementProps(node, oldProps, newProps);
}

function applyFormProps(
  node: hostComponents.form.Element,
  oldProps: Partial<hostComponents.form.Props>,
  newProps: hostComponents.form.Props,
): void {
  applyScrollableElementProps(node, oldProps, newProps);
}

function applyScrollableElementProps<Props extends hostComponents.CommonProps & Partial<hostComponents.Scrollable>>(
  node: PaintElement & hostComponents.Scrollable,
  oldProps: Partial<Props>,
  newProps: Props,
): void {
  applyCommonProps(node, oldProps, newProps);
  applyScrollableProps(node, newProps);
}

function applyButtonProps(
  node: hostComponents.button.Element,
  oldProps: Partial<hostComponents.button.Props>,
  newProps: hostComponents.button.Props,
): void {
  applyCommonProps(node, oldProps, newProps);
  if (newProps.type !== undefined) {
    node.type = newProps.type;
  }
  applyScrollableProps(node, newProps);
}

function applyInputProps(
  node: hostComponents.input.Element,
  oldProps: Partial<hostComponents.input.Props>,
  newProps: hostComponents.input.Props,
): void {
  applyCommonProps(node, oldProps, newProps);
  if (newProps.type !== undefined) {
    node.type = newProps.type;
  }
  applyTextControlProps(node, oldProps, newProps);
}

function applyTextareaProps(
  node: hostComponents.textarea.Element,
  oldProps: Partial<hostComponents.textarea.Props>,
  newProps: hostComponents.textarea.Props,
): void {
  applyCommonProps(node, oldProps, newProps);
  applyTextControlProps(node, oldProps, newProps);
  applyScrollableProps(node, newProps);
}

function applyTextControlProps<T extends hostComponents.input.Element | hostComponents.textarea.Element>(
  node: T,
  oldProps: Partial<hostComponents.input.Props | hostComponents.textarea.Props>,
  newProps: hostComponents.input.Props | hostComponents.textarea.Props,
): void {
  if (newProps.value !== undefined) {
    node.value = newProps.value;
  }
  if (newProps.placeholder !== undefined) {
    node.placeholder = newProps.placeholder;
  }
  if (newProps.cursorPosition !== undefined) {
    node.cursorPosition = newProps.cursorPosition;
  }
  if (newProps.autoFocus === true && oldProps.autoFocus !== true) {
    node.focus();
  }
}

function applyScrollableProps(
  node: hostComponents.Scrollable,
  props: Partial<hostComponents.Scrollable>,
): void {
  if (props.scrollLeft !== undefined) {
    node.scrollLeft = props.scrollLeft;
  }
  if (props.scrollTop !== undefined) {
    node.scrollTop = props.scrollTop;
  }
}

function applyStyle(node: PaintElement, oldStyle: CSSStyleProperties | undefined, newStyle: CSSStyleProperties | undefined): void {
  if (oldStyle === newStyle || newStyle === undefined) {
    return;
  }

  for (const [key, value] of Object.entries(newStyle)) {
    const styleKey = key as CSSStylePropertyName;
    if (value !== undefined && value !== null && value !== oldStyle?.[styleKey]) {
      node.style.setProperty(styleKey, value);
    }
  }
}

function applyEvents(node: PaintElement, oldProps: Partial<HostProps>, newProps: HostProps): void {
  for (const [prop, eventType] of eventProps) {
    const previous = listenerProp(oldProps, prop);
    const next = listenerProp(newProps, prop);
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

function listenerProp(props: Partial<HostProps>, prop: hostComponents.EventPropName): ((event: unknown) => void) | undefined {
  const value = (props as Partial<Record<hostComponents.EventPropName, unknown>>)[prop];
  return typeof value === 'function' ? value as (event: unknown) => void : undefined;
}

function canHaveChildren(node: PaintElement): node is ChildContainerElement {
  return 'appendChild' in node && 'insertBefore' in node;
}

function addElementListener(node: PaintElement, eventType: ElementEventType, listener: (event: unknown) => void): void {
  (node as {addEventListener(type: ElementEventType, listener: (event: unknown) => void): void})
    .addEventListener(eventType, reactEventListener(listener));
}

function removeElementListener(node: PaintElement, eventType: ElementEventType, listener: (event: unknown) => void): void {
  (node as {removeEventListener(type: ElementEventType, listener: (event: unknown) => void): void})
    .removeEventListener(eventType, reactEventListener(listener));
}

function reactEventListener(listener: (event: unknown) => void): (event: unknown) => void {
  let wrapped = eventListenerWrappers.get(listener);
  if (wrapped === undefined) {
    wrapped = (event: unknown): void => {
      reconciler.flushSyncFromReconciler(() => {
        listener(event);
      });
    };
    eventListenerWrappers.set(listener, wrapped);
  }
  return wrapped;
}

function defaultDisplay(type: HostType): string {
  return type === hostComponents.span.type ? 'inline' : 'block';
}

function reportReactError(error: unknown): void {
  if (error !== null && error !== undefined) {
    console.error(error);
  }
}

function loadPackageInfo(): PackageInfo {
  const packageJsonPath = findPackageJsonPath();
  const packageJson = fs.readFileSync(packageJsonPath, 'utf8');
  const parsed = JSON.parse(packageJson) as Partial<PackageInfo> | undefined;

  if (parsed?.name !== 'paintcannon-react' || typeof parsed.version !== 'string') {
    throw new Error(`Invalid package metadata in ${packageJsonPath}`);
  }

  return {
    name: parsed.name,
    version: parsed.version,
  };
}

function findPackageJsonPath(): string {
  let directory = sourceDirectory;
  while (true) {
    const packageJsonPath = path.join(directory, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      const parsed = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as Partial<PackageInfo> | undefined;
      if (parsed?.name === 'paintcannon-react') {
        return packageJsonPath;
      }
    }

    const parent = path.dirname(directory);
    if (parent === directory) {
      throw new Error('Could not find paintcannon-react package metadata');
    }
    directory = parent;
  }
}

const eventProps = [
  ...ELEMENT_EVENT_TYPES.map((eventType) => [eventPropName(eventType), eventType] as const),
] satisfies ReadonlyArray<readonly [hostComponents.EventPropName, ElementEventType]>;

function eventPropName<T extends ElementEventType>(eventType: T): hostComponents.EventPropName<T> {
  if (eventType.startsWith('key')) {
    return `onKey${capitalize(eventType.slice('key'.length))}` as hostComponents.EventPropName<T>;
  }
  if (eventType.startsWith('mouse')) {
    return `onMouse${capitalize(eventType.slice('mouse'.length))}` as hostComponents.EventPropName<T>;
  }
  if (eventType.startsWith('transition')) {
    return `onTransition${capitalize(eventType.slice('transition'.length))}` as hostComponents.EventPropName<T>;
  }
  return `on${capitalize(eventType)}` as hostComponents.EventPropName<T>;
}

function capitalize(value: string): string {
  return `${value[0]?.toUpperCase() ?? ''}${value.slice(1)}`;
}

export type {
  PaintCannon,
  PaintCannonOptions,
  PaintChangeEvent,
  PaintElement,
  PaintFocusEvent,
  PaintKeyboardEvent,
  PaintMouseEvent,
  PaintScrollEvent,
  PaintSubmitEvent,
};
