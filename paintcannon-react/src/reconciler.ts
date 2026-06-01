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
  PaintElement,
  PaintNode,
  SpanElement as PaintSpanElement,
} from 'paintcannon';
import {
  ELEMENT_EVENT_TYPES,
  PaintCannon,
} from 'paintcannon';
import {
  AnimationContext,
  AnimationScheduler,
  AppContext,
} from './hooks/index.ts';
import * as hostComponents from './host-components/index.ts';

type HostType = hostComponents.HostType;
type HostProps = hostComponents.HostProps;
type HostNode = hostComponents.HostNode;
type HostElement = hostComponents.HostElement;
type HostText = hostComponents.HostText;
type HostParent = HostElement | RootContainer;
type ChildContainerElement = PaintElement & {
  appendChild(child: PaintNode): PaintNode;
  insertBefore(child: PaintNode, before: PaintNode): PaintNode;
};

export type CreateRootOptions = PaintCannonOptions & {
  paintCannon?: PaintCannon;
  container?: PaintDivElement | PaintSpanElement | PaintFormElement;
};

export type PaintCannonReactRoot = {
  readonly paintCannon: PaintCannon;
  readonly container: PaintDivElement | PaintSpanElement | PaintFormElement;
  render(element: React.ReactNode): void;
  unmount(): void;
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
};

type RootContainer = {
  paintCannon: PaintCannon;
  root: PaintDivElement | PaintSpanElement | PaintFormElement;
  children: HostNode[];
};

type PackageInfo = {
  name: string;
  version: string;
};

let currentUpdatePriority = NoEventPriority;
const sourceDirectory = path.dirname(fileURLToPath(import.meta.url));
const packageInfo = loadPackageInfo();
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
  const app = {
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
    return hostComponents.reconcileDiv.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }
  if (type === hostComponents.span.type) {
    return hostComponents.reconcileSpan.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }
  if (type === hostComponents.form.type) {
    return hostComponents.reconcileForm.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }
  if (type === hostComponents.button.type) {
    return hostComponents.reconcileButton.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }
  if (type === hostComponents.input.type) {
    return hostComponents.reconcileInput.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }
  if (type === hostComponents.textarea.type) {
    return hostComponents.reconcileTextarea.create(
      paintCannon,
      hostComponents.castHostProps(type, props),
      applyCommonProps,
    );
  }

  const unknownType: never = type;
  throw new Error(`Unknown host type: ${String(unknownType)}`);
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
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileDiv.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }
  if (host.type === hostComponents.span.type) {
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileSpan.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }
  if (host.type === hostComponents.form.type) {
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileForm.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }
  if (host.type === hostComponents.button.type) {
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileButton.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }
  if (host.type === hostComponents.input.type) {
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileInput.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }
  if (host.type === hostComponents.textarea.type) {
    const props = hostComponents.castHostProps(host.type, newProps);
    hostComponents.reconcileTextarea.applyProps(host.node, host.props, props, applyCommonProps);
    return props;
  }

  const unknownHost: never = host;
  throw new Error(`Unknown host type: ${String(unknownHost)}`);
}

function applyCommonProps(
  node: PaintElement,
  oldProps: Partial<hostComponents.CommonProps>,
  newProps: hostComponents.CommonProps,
): void {
  applyStyle(node, oldProps.style, newProps.style);
  applyEvents(node, oldProps, newProps);
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
  ...ELEMENT_EVENT_TYPES.map((eventType) => [hostComponents.EVENT_PROP_NAMES[eventType], eventType] as const),
] satisfies ReadonlyArray<readonly [hostComponents.EventPropName, ElementEventType]>;
