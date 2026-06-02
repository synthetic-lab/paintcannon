import type { SpanElement as PaintSpanElement } from "paintcannon";
import type { CommonProps, HostComponent, Scrollable } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = CommonProps & Partial<Scrollable>;
export type Element = PaintSpanElement;

export const type = typeString("paintcannon.span");
export const Component = type as unknown as HostComponent<Props, Element>;
