import type { PaintCannon } from "paintcannon";
import * as div from "./div.ts";
import type { MountedComponent } from "../mounted.ts";
import { type ApplyCommonProps, applyScrollableElementProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: div.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof div> {
  const node = paintCannon.createElement("div");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: div.type, props, children: [], node };
}

export function applyProps(
  node: div.Element,
  oldProps: Partial<div.Props>,
  newProps: div.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyScrollableElementProps(node, oldProps, newProps, applyCommonProps);
}
