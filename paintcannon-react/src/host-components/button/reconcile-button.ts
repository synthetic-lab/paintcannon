import type { PaintCannon } from "paintcannon";
import * as button from "./button.ts";
import type { MountedComponent } from "../mounted.ts";
import { type ApplyCommonProps, applyScrollableProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: button.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof button> {
  const node = paintCannon.createElement("button");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: button.type, props, children: [], node };
}

export function applyProps(
  node: button.Element,
  oldProps: Partial<button.Props>,
  newProps: button.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyCommonProps(node, oldProps, newProps);
  if (newProps.type !== undefined) {
    node.type = newProps.type;
  }
  applyScrollableProps(node, newProps);
}
