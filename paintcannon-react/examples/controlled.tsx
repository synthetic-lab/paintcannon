import React, {useState} from 'react';
import type {PaintChangeEvent} from 'paintcannon';
import {Div, Input, Span, Textarea, render, useApp} from '../src/index.ts';

interface ControlledState {
  value: string;
  updates: number;
}

function ControlledDemo(): React.ReactElement {
  const {exit} = useApp();
  const [input, setInput] = useState<ControlledState>(() => emptyState());
  const [textarea, setTextarea] = useState<ControlledState>(() => emptyState());

  return (
    <Div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 1,
        backgroundColor: '#111827',
        color: '#e5e7eb',
      }}
      onKeyDown={event => {
        if (event.key === 'Escape' || (event.ctrlKey && event.code === 'KeyC')) {
          event.preventDefault();
          exit();
        }
      }}
    >
      <Span style={{color: '#93c5fd'}}>paintcannon-react controlled input demo</Span>
      <Span style={{color: '#94a3b8'}}>React-style controlled value with onChange. Escape or Ctrl-C exits.</Span>
      <Field
        label="input"
        state={input}
        setState={setInput}
        placeholder="type here"
      />
      <Field
        label="textarea"
        state={textarea}
        setState={setTextarea}
        placeholder="multi-line typing"
        multiline
      />
    </Div>
  );
}

function Field({
  label,
  state,
  setState,
  placeholder,
  multiline = false,
}: {
  label: string;
  state: ControlledState;
  setState: React.Dispatch<React.SetStateAction<ControlledState>>;
  placeholder: string;
  multiline?: boolean;
}): React.ReactElement {
  const inputStyle = {
    width: 48,
    minHeight: multiline ? 5 : 3,
    backgroundColor: '#020617',
    color: '#f8fafc',
    placeholderColor: '#64748b',
    border: 'rounded',
    borderColor: '#64748b',
  };
  const handleChange = (event: PaintChangeEvent): void => {
    setState(current => ({
      value: event.target.value,
      updates: current.updates + 1,
    }));
  };

  return (
    <Div style={{display: 'flex', flexDirection: 'row', alignItems: 'center', gap: 2}}>
      <Div style={{width: 10, color: '#cbd5e1'}}>{label}</Div>
      {multiline ? (
        <Textarea
          value={state.value}
          placeholder={placeholder}
          style={inputStyle}
          onChange={handleChange}
        />
      ) : (
        <Input
          autoFocus
          value={state.value}
          placeholder={placeholder}
          style={inputStyle}
          onChange={handleChange}
        />
      )}
      <Div style={{width: 18, color: '#a7f3d0'}}>
        updates={state.updates} chars={Array.from(state.value).length}
      </Div>
    </Div>
  );
}

function emptyState(): ControlledState {
  return {
    value: '',
    updates: 0,
  };
}

const root = render(<ControlledDemo />, {
  alternateScreen: true,
  captureCtrlC: true,
});

root.waitUntilExit().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
