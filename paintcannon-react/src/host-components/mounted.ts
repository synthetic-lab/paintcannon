import type { PaintElement, TextNode } from "paintcannon";
import * as button from "./button/button.ts";
import * as div from "./div/div.ts";
import * as form from "./form/form.ts";
import * as input from "./input/input.ts";
import * as span from "./span/span.ts";
import * as textarea from "./textarea/textarea.ts";
import type { HostComponent, HostType } from "./shared.ts";

export type HostProps =
  | div.Props
  | span.Props
  | form.Props
  | button.Props
  | input.Props
  | textarea.Props;

export type MountedHostElement<Type extends HostType, Props, Element extends PaintElement> = {
  kind: "element";
  type: Type;
  props: Props;
  children: HostNode[];
  node: Element;
};

export type MountedComponent<Module> = Module extends {
  type: infer Type extends HostType;
  Component: HostComponent<infer Props, infer Element extends PaintElement>;
}
  ? MountedHostElement<Type, Props, Element>
  : never;

export type HostElement =
  | MountedComponent<typeof div>
  | MountedComponent<typeof span>
  | MountedComponent<typeof form>
  | MountedComponent<typeof button>
  | MountedComponent<typeof input>
  | MountedComponent<typeof textarea>;

export type HostText = {
  kind: "text";
  text: string;
  node: TextNode;
};

export type HostNode = HostElement | HostText;

const hostPropsForType = {
  [div.type]: undefined as unknown as div.Props,
  [span.type]: undefined as unknown as span.Props,
  [form.type]: undefined as unknown as form.Props,
  [button.type]: undefined as unknown as button.Props,
  [input.type]: undefined as unknown as input.Props,
  [textarea.type]: undefined as unknown as textarea.Props,
} satisfies { [K in HostType]: HostProps };

export type HostPropsForType = typeof hostPropsForType;

export function castHostProps<Type extends HostType>(
  _type: Type,
  props: HostProps,
): HostPropsForType[Type] {
  return props as HostPropsForType[Type];
}
