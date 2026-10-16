import type { PaintCannon } from "paintcannon";
import * as a from "./a.ts";
import type { MountedComponent } from "../mounted.ts";
import { type ApplyCommonProps, applyScrollableElementProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: a.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof a> {
  const node = paintCannon.createElement("a");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: a.type, props, children: new Set(), node };
}

export function applyProps(
  node: a.Element,
  oldProps: Partial<a.Props>,
  newProps: a.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyScrollableElementProps(node, oldProps, newProps, applyCommonProps);
  if (oldProps.href !== newProps.href) {
    node.href = newProps.href ?? "";
  }
}
