import type { PaintCannon } from "paintcannon";
import * as textarea from "./textarea.ts";
import type { MountedComponent } from "../mounted.ts";
import {
  type ApplyCommonProps,
  applyScrollableProps,
  applyTextControlProps,
} from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: textarea.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof textarea> {
  const node = paintCannon.createElement("textarea");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: textarea.type, props, children: new Set(), node };
}

export function applyProps(
  node: textarea.Element,
  oldProps: Partial<textarea.Props>,
  newProps: textarea.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyCommonProps(node, oldProps, newProps);
  applyTextControlProps(node, oldProps, newProps);
  applyScrollableProps(node, newProps);
}
