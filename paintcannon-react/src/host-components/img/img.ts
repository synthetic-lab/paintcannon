import type { ImageElement } from "paintcannon";
import type { CommonProps, HostComponent } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = Omit<CommonProps, "children"> & { src: string };
export type Element = ImageElement;

export const type = typeString("paintcannon.img");
export const Component = type as unknown as HostComponent<Props, Element>;
