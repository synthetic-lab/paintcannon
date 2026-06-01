import type {InputElement, PaintElement, TextAreaElement} from 'paintcannon';
import type * as input from './input/input.ts';
import type * as textarea from './textarea/textarea.ts';
import type {CommonProps, Scrollable} from './shared.ts';

export type ApplyCommonProps = (
  node: PaintElement,
  oldProps: Partial<CommonProps>,
  newProps: CommonProps,
) => void;

export function applyScrollableElementProps<Props extends CommonProps & Partial<Scrollable>>(
  node: PaintElement & Scrollable,
  oldProps: Partial<Props>,
  newProps: Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyCommonProps(node, oldProps, newProps);
  applyScrollableProps(node, newProps);
}

export function applyScrollableProps(node: Scrollable, props: Partial<Scrollable>): void {
  if (props.scrollLeft !== undefined) {
    node.scrollLeft = props.scrollLeft;
  }
  if (props.scrollTop !== undefined) {
    node.scrollTop = props.scrollTop;
  }
}

export function applyTextControlProps<T extends InputElement | TextAreaElement>(
  node: T,
  oldProps: Partial<input.Props | textarea.Props>,
  newProps: input.Props | textarea.Props,
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
