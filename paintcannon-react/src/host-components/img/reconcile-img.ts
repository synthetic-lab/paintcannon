import type { PaintCannon } from "paintcannon";
import * as img from "./img.ts";
import type { MountedComponent } from "../mounted.ts";
import type { ApplyCommonProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: img.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof img> {
  const node = paintCannon.createElement("img");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: img.type, props, children: new Set(), node };
}

export function applyProps(
  node: img.Element,
  oldProps: Partial<img.Props>,
  newProps: img.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyCommonProps(node, oldProps, newProps);
  if (newProps.src !== oldProps.src) node.src = newProps.src;
}
