import type {PaintCannon} from 'paintcannon';
import * as input from './input.ts';
import type {MountedComponent} from '../mounted.ts';
import {type ApplyCommonProps, applyTextControlProps} from '../reconcile-shared.ts';

export function create(
  paintCannon: PaintCannon,
  props: input.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof input> {
  const node = paintCannon.createElement('input');
  applyProps(node, {}, props, applyCommonProps);
  return { kind: 'element', type: input.type, props, children: [], node };
}

export function applyProps(
  node: input.Element,
  oldProps: Partial<input.Props>,
  newProps: input.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyCommonProps(node, oldProps, newProps);
  if (newProps.type !== undefined) {
    node.type = newProps.type;
  }
  applyTextControlProps(node, oldProps, newProps);
}
