import type {DivElement as PaintDivElement} from 'paintcannon';
import type {CommonProps, HostComponent, Scrollable} from '../shared.ts';
import {typeString} from '../shared.ts';

export type Props = CommonProps & Partial<Scrollable>;
export type Element = PaintDivElement;

export const type = typeString('paintcannon.div');
export const Component = type as unknown as HostComponent<Props, Element>;
