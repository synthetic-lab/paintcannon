import type { AnchorElement } from "paintcannon";
import type { CommonProps, HostComponent, Scrollable } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = CommonProps & Partial<Scrollable> & { href?: string };
export type Element = AnchorElement;
export const type = typeString("paintcannon.a");
export const Component = type as unknown as HostComponent<Props, Element>;
