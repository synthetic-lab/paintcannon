import type { PaintCannon } from "paintcannon";
import * as form from "./form.ts";
import type { MountedComponent } from "../mounted.ts";
import { type ApplyCommonProps, applyScrollableElementProps } from "../reconcile-shared.ts";

export function create(
  paintCannon: PaintCannon,
  props: form.Props,
  applyCommonProps: ApplyCommonProps,
): MountedComponent<typeof form> {
  const node = paintCannon.createElement("form");
  applyProps(node, {}, props, applyCommonProps);
  return { kind: "element", type: form.type, props, children: [], node };
}

export function applyProps(
  node: form.Element,
  oldProps: Partial<form.Props>,
  newProps: form.Props,
  applyCommonProps: ApplyCommonProps,
): void {
  applyScrollableElementProps(node, oldProps, newProps, applyCommonProps);
}
