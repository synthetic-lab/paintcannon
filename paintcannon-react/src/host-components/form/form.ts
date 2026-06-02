import type { FormElement as PaintFormElement } from "paintcannon";
import type { CommonProps, HostComponent, Scrollable } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = CommonProps & Partial<Scrollable>;
export type Element = PaintFormElement;

export const type = typeString("paintcannon.form");
export const Component = type as unknown as HostComponent<Props, Element>;
