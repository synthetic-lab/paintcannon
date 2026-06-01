import * as hostComponents from './host-components/index.ts';

export {
  useAnimation,
  useApp,
  type AnimationOptions,
  type AnimationResult,
  type PaintCannonReactApp,
} from './hooks/index.ts';

export {
  createRoot,
  render,
  type CreateRootOptions,
  type PaintCannonReactRoot,
} from './reconciler.ts';

export type DivProps = hostComponents.div.Props;
export type DivElement = hostComponents.div.Element;
export const Div = hostComponents.div.Component;
export type SpanProps = hostComponents.span.Props;
export type SpanElement = hostComponents.span.Element;
export const Span = hostComponents.span.Component;
export type FormProps = hostComponents.form.Props;
export type FormElement = hostComponents.form.Element;
export const Form = hostComponents.form.Component;
export type ButtonProps = hostComponents.button.Props;
export type ButtonElement = hostComponents.button.Element;
export const Button = hostComponents.button.Component;
export type InputProps = hostComponents.input.Props;
export type InputElement = hostComponents.input.Element;
export const Input = hostComponents.input.Component;
export type TextareaProps = hostComponents.textarea.Props;
export type TextareaElement = hostComponents.textarea.Element;
export const Textarea = hostComponents.textarea.Component;
