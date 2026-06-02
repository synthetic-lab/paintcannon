import type { ButtonElement as PaintButtonElement } from "paintcannon";
import type { CommonProps, HostComponent, Scrollable } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = CommonProps &
  Partial<Scrollable> & {
    type?: "submit" | "button";
  };
export type Element = PaintButtonElement;

export const type = typeString("paintcannon.button");
export const Component = type as unknown as HostComponent<Props, Element>;
