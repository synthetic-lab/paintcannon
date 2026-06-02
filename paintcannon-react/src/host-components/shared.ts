import type React from "react";
import type {
  CSSStyleProperties,
  ElementEventListenerFor,
  ElementEventType,
  PaintElement,
  PaintElementTagName,
} from "paintcannon";

export type HostTagName = Exclude<PaintElementTagName, "img">;
export type HostType = `paintcannon.${HostTagName}`;

export function typeString<Type extends HostType>(type: Type): Type {
  return type;
}

export type Scrollable = {
  scrollLeft: number;
  scrollTop: number;
};

export type HostComponent<Props, Element extends PaintElement> = React.ForwardRefExoticComponent<
  React.PropsWithoutRef<Props> & React.RefAttributes<Element>
>;

export const EVENT_PROP_NAMES = {
  keydown: "onKeyDown",
  keyup: "onKeyUp",
  click: "onClick",
  mouseenter: "onMouseEnter",
  mouseleave: "onMouseLeave",
  mousemove: "onMouseMove",
  focus: "onFocus",
  blur: "onBlur",
  submit: "onSubmit",
  change: "onChange",
  transitionstart: "onTransitionStart",
  transitionend: "onTransitionEnd",
  scroll: "onScroll",
} as const satisfies { [T in ElementEventType]: string };

export type EventPropName<T extends ElementEventType = ElementEventType> =
  (typeof EVENT_PROP_NAMES)[T];

type ElementEventProps = {
  [T in ElementEventType as EventPropName<T>]?: ElementEventListenerFor<T>;
};

export type CommonProps = ElementEventProps & {
  children?: React.ReactNode;
  style?: CSSStyleProperties;
};

export type TextControlProps = CommonProps & {
  value?: string;
  placeholder?: string;
  cursorPosition?: number;
  autoFocus?: boolean;
};
