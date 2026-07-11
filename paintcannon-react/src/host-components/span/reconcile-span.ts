import type { PaintCannon } from "paintcannon";
import * as span from "./span.ts";
import type { MountedComponent } from "../mounted.ts";
import { type ApplyCommonProps, applyScrollableElementProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: span.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof span> {
  const node = paintCannon.createElement("span");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: span.type, props, children: new Set(), node };
}

export function applyProps(
  node: span.Element,
  oldProps: Partial<span.Props>,
  newProps: span.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyScrollableElementProps(node, oldProps, newProps, applyCommonProps);
}
