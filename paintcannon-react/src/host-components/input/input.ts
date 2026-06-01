import type {InputElement as PaintInputElement} from 'paintcannon';
import type {HostComponent, TextControlProps} from '../shared.ts';
import {typeString} from '../shared.ts';

export type Props = TextControlProps & {
  type?: 'text';
};
export type Element = PaintInputElement;

export const type = typeString('paintcannon.input');
export const Component = type as unknown as HostComponent<Props, Element>;
