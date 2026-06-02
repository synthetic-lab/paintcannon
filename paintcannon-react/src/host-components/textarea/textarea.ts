import type { TextAreaElement as PaintTextAreaElement } from "paintcannon";
import type { HostComponent, Scrollable, TextControlProps } from "../shared.ts";
import { typeString } from "../shared.ts";

export type Props = TextControlProps &
  Partial<Scrollable> & {
    type?: never;
  };
export type Element = PaintTextAreaElement;

export const type = typeString("paintcannon.textarea");
export const Component = type as unknown as HostComponent<Props, Element>;
