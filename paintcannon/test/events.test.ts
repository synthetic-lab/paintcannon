import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mock } from 'antipattern';
import {
  PaintCannon,
  paintCannonDeps,
  type PaintElement,
} from '../index.ts';
import {
  createMockNativeBinding,
  keyDown,
  mouseEvent,
  type MockNativePaintCannon,
} from './mock-native.ts';

let restores: Array<() => void> = [];
let mockNativeInstances: MockNativePaintCannon[] = [];

beforeEach(() => {
  mockNativeInstances = [];
  restores = [
    mock(paintCannonDeps, 'loadNativeBinding', () => createMockNativeBinding(mockNativeInstances)),
  ];
});

afterEach(() => {
  for (const restore of restores.reverse()) {
    restore();
  }
  restores = [];
});

describe('core keyboard events', () => {
  it('targets the first root child and bubbles through ancestors before document listeners', () => {
    const { paintCannon, mockNative, root, child } = createPaintTree();
    const events: string[] = [];

    child.addEventListener('keydown', (event) => {
      events.push(`child:${event.key}:${event.target === child}:${event.currentTarget === child}`);
    });
    root.addEventListener('keydown', (event) => {
      events.push(`root:${event.key}:${event.target === child}:${event.currentTarget === root}`);
    });
    paintCannon.addEventListener('keydown', (event) => {
      events.push(`document:${event.key}:${event.target === child}`);
    });

    mockNative.keyboardEvents.push(keyDown('a'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      'child:a:true:true',
      'root:a:true:true',
      'document:a:true',
    ]);
  });

  it('respects stopPropagation for element and document keyboard listeners', () => {
    const { paintCannon, mockNative, root, child } = createPaintTree();
    const events: string[] = [];

    child.addEventListener('keydown', (event) => {
      events.push('child');
      event.stopPropagation();
    });
    root.addEventListener('keydown', () => {
      events.push('root');
    });
    paintCannon.addEventListener('keydown', () => {
      events.push('document');
    });

    mockNative.keyboardEvents.push(keyDown('a'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(['child']);
  });

  it('routes key events to the focused input and lets preventDefault block text insertion', () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement('input');
    const events: string[] = [];
    root.appendChild(input);

    input.addEventListener('keydown', (event) => {
      events.push(`input:${event.key}`);
      event.preventDefault();
    });
    root.addEventListener('keydown', () => {
      events.push('root');
    });
    input.focus();
    mockNative.keyboardEvents.push(keyDown('x'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(['input:x', 'root']);
    expect(input.value).toBe('');
    expect(mockNative.textControls.get(input.id)?.value).toBe('');
  });

  it('applies default text input when keydown is not prevented', () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const input = paintCannon.createElement('input');
    root.appendChild(input);

    input.focus();
    mockNative.keyboardEvents.push(keyDown('x'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(input.value).toBe('x');
    expect(input.cursorPosition).toBe(1);
    expect(mockNative.textControls.get(input.id)).toMatchObject({
      value: 'x',
      cursor: 1,
    });
  });

  it('cycles text control focus with tab and shift-tab', () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const first = paintCannon.createElement('input');
    const second = paintCannon.createElement('textarea');
    const events: string[] = [];
    root.appendChild(first);
    root.appendChild(second);

    first.addEventListener('focus', () => events.push('first:focus'));
    first.addEventListener('blur', () => events.push('first:blur'));
    second.addEventListener('focus', () => events.push('second:focus'));
    second.addEventListener('blur', () => events.push('second:blur'));

    mockNative.keyboardEvents.push(keyDown('Tab', { code: 'Tab' }));
    runKeyboardEventPump(paintCannon);
    mockNative.keyboardEvents.push(keyDown('Tab', { code: 'Tab' }));
    runKeyboardEventPump(paintCannon);
    mockNative.keyboardEvents.push(keyDown('Tab', { code: 'Tab', shiftKey: true }));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      'first:focus',
      'first:blur',
      'second:focus',
      'second:blur',
      'first:focus',
    ]);
    expect(mockNative.textControls.get(first.id)?.focused).toBe(true);
    expect(mockNative.textControls.get(second.id)?.focused).toBe(false);
  });
});

describe('core submit events', () => {
  it('submits the nearest form on input enter and bubbles the submit event', () => {
    const { paintCannon, mockNative, root } = createPaintTree();
    const form = paintCannon.createElement('form');
    const input = paintCannon.createElement('input');
    const events: string[] = [];
    root.appendChild(form);
    form.appendChild(input);

    form.addEventListener('submit', (event) => {
      events.push(`form:${event.target === form}:${event.currentTarget === form}:${event.submitter === input}`);
    });
    root.addEventListener('submit', (event) => {
      events.push(`root:${event.target === form}:${event.currentTarget === root}:${event.submitter === input}`);
    });

    input.focus();
    mockNative.keyboardEvents.push(keyDown('Enter', { code: 'Enter' }));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual([
      'form:true:true:true',
      'root:true:true:true',
    ]);
  });

  it('submits a form from a submit button click unless the click default is prevented', () => {
    const { paintCannon, mockNative, root } = createPaintTree({ captureMouse: true });
    const form = paintCannon.createElement('form');
    const button = paintCannon.createElement('button');
    const events: string[] = [];
    root.appendChild(form);
    form.appendChild(button);
    mockNative.targetIdAtPoint = button.id;

    form.addEventListener('submit', (event) => {
      events.push(`submit:${event.submitter === button}`);
    });
    mockNative.mouseEvents.push(mouseEvent('click'));
    runKeyboardEventPump(paintCannon);

    button.addEventListener('click', (event) => {
      event.preventDefault();
      events.push('click:prevented');
    });
    mockNative.mouseEvents.push(mouseEvent('click'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(['submit:true', 'click:prevented']);
  });
});

describe('core mouse events', () => {
  it('bubbles click events and respects stopPropagation', () => {
    const { paintCannon, mockNative, root, child } = createPaintTree({ captureMouse: true });
    const events: string[] = [];
    mockNative.targetIdAtPoint = child.id;

    child.addEventListener('click', (event) => {
      events.push(`child:${event.target === child}:${event.currentTarget === child}`);
    });
    root.addEventListener('click', (event) => {
      events.push(`root:${event.target === child}:${event.currentTarget === root}`);
      event.stopPropagation();
    });

    mockNative.mouseEvents.push(mouseEvent('click'));
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(events).toEqual(['child:true:true', 'root:true:true']);
  });
});

describe('core resize events', () => {
  it('dispatches the latest resize and uses the normal render path', () => {
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = currentMockNative();
    const sizes: Array<[number, number]> = [];

    paintCannon.addEventListener('resize', (event) => {
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

function createPaintTree(options: { captureMouse?: boolean } = {}): {
  paintCannon: PaintCannon;
  mockNative: MockNativePaintCannon;
  root: PaintElement;
  child: PaintElement;
} {
  const paintCannon = new PaintCannon({ fps: 120, captureMouse: options.captureMouse });
  const root = paintCannon.createElement('div');
  const child = paintCannon.createElement('div');
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
    throw new Error('expected mock native instance');
  }
  return mockNative;
}

function runKeyboardEventPump(paintCannon: PaintCannon): void {
  (paintCannon as unknown as { runKeyboardEventPump(): void }).runKeyboardEventPump();
}
