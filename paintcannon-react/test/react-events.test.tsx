import React from 'react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { mock } from 'antipattern';
import {
  PaintCannon,
  PaintKeyboardEvent,
  paintCannonDeps,
  type InputElement,
  type PaintElement,
  type PaintResizeEvent,
  type TextAreaElement,
} from 'paintcannon';
import { Div, Input, Textarea, render } from '../src/index.ts';
import { createMockNativeBinding, type MockNativePaintCannon } from '../../paintcannon/test/mock-native.ts';

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

describe('keyboard events', () => {
  it('targets root content when no text control is focused', async () => {
    const events: string[] = [];
    const root = render(
      <Div onKeyDown={event => events.push(event.key)}>root</Div>,
      { fps: 120 },
    );

    await commit();
    dispatchKey(root.paintCannon, 'z');
    root.paintCannon.stop();

    expect(events).toEqual(['z']);
  });

  it('targets the focused text control and does not notify sibling controls', async () => {
    const events: string[] = [];
    let input: InputElement | undefined;
    let textarea: TextAreaElement | undefined;

    const root = render(
      <Div>
        <Input
          ref={value => {
            input = value;
          }}
          onKeyDown={event => {
            events.push(`input:${event.key}`);
            event.preventDefault();
          }}
        />
        <Textarea
          ref={value => {
            textarea = value;
          }}
          onKeyDown={event => {
            events.push(`textarea:${event.key}`);
            event.preventDefault();
          }}
        />
      </Div>,
      { fps: 120 },
    );

    await commit();
    expect(input).toBeDefined();
    expect(textarea).toBeDefined();

    input?.focus();
    dispatchKey(root.paintCannon, 'a');
    textarea?.focus();
    dispatchKey(root.paintCannon, 'b');
    root.paintCannon.stop();

    expect(events).toEqual(['input:a', 'textarea:b']);
  });

  it('does not re-apply autoFocus on controlled updates', async () => {
    const events: string[] = [];
    let textarea: TextAreaElement | undefined;
    let update = (): void => {};

    function App(): React.ReactElement {
      const [value, setValue] = React.useState('');
      update = () => {
        setValue(current => `${current}x`);
      };

      return (
        <Div>
          <Input autoFocus value={value} onKeyDown={event => events.push(`input:${event.key}`)} />
          <Textarea
            ref={value => {
              textarea = value;
            }}
            onKeyDown={event => events.push(`textarea:${event.key}`)}
          />
        </Div>
      );
    }

    const root = render(<App />, { fps: 120 });

    await commit();
    textarea?.focus();
    update();
    await commit();
    dispatchKey(root.paintCannon, 'a');
    root.paintCannon.stop();

    expect(events).toEqual(['textarea:a']);
  });
});

describe('resize events', () => {
  it('uses the normal render path instead of sync rendering inside the input pump', () => {
    const sizes: Array<[number, number]> = [];
    const paintCannon = new PaintCannon({ fps: 120 });
    const mockNative = mockNativeInstances[0];
    if (mockNative === undefined) {
      throw new Error('expected mock native instance');
    }

    paintCannon.addEventListener('resize', (event: PaintResizeEvent) => {
      sizes.push([event.cols, event.rows]);
    });
    mockNative.resizeEvents.push({ cols: 100, rows: 40 });
    runKeyboardEventPump(paintCannon);
    paintCannon.stop();

    expect(sizes).toEqual([[100, 40]]);
    expect(mockNative.renderCalls).toBe(1);
    expect(mockNative.renderSyncCalls).toBe(0);
  });
});

function dispatchKey(paintCannon: PaintCannon, key: string): void {
  const anyPaintCannon = paintCannon as unknown as {
    keyboardEventTarget(): PaintElement | undefined;
    dispatchKeyboardEvent(target: PaintElement, event: PaintKeyboardEvent): void;
  };
  const target = anyPaintCannon.keyboardEventTarget();
  if (target === undefined) {
    throw new Error('expected keyboard event target');
  }
  anyPaintCannon.dispatchKeyboardEvent(target, new PaintKeyboardEvent({
    type: 'keydown',
    key,
    code: `Key${key.toUpperCase()}`,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    repeat: false,
  }, target));
}

function runKeyboardEventPump(paintCannon: PaintCannon): void {
  (paintCannon as unknown as { runKeyboardEventPump(): void }).runKeyboardEventPump();
}

async function commit(): Promise<void> {
  await new Promise(resolve => setTimeout(resolve, 20));
}
