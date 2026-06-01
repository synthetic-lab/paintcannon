import React from 'react';
import {Div, Span, render, useAnimation, useApp} from '../src/index.ts';

const spinnerFrames = ['|', '/', '-', '\\'];
const dots = ['', '.', '..', '...'];

function SpinnerDemo(): React.ReactElement {
  const {exit} = useApp();
  const {frame, time, delta} = useAnimation();
  const slow = useAnimation({interval: 240});
  const spinnerFrame = Math.floor(time / 90);
  const progress = Math.round(((Math.sin(time / 700) + 1) / 2) * 24);

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
        backgroundColor: '#020617',
        color: '#e2e8f0',
      }}
      onKeyDown={event => {
        if (event.key === 'Escape' || event.key === 'q' || (event.ctrlKey && event.code === 'KeyC')) {
          event.preventDefault();
          exit();
        }
      }}
    >
      <Span style={{color: '#38bdf8'}}>paintcannon-react useAnimation</Span>
      <Div
        style={{
          display: 'flex',
          flexDirection: 'row',
          gap: 1,
          padding: '1 2',
          border: 'rounded',
          borderColor: '#334155',
          backgroundColor: '#0f172a',
          color: '#f8fafc',
        }}
      >
        <Span style={{color: '#fb923c'}}>{spinnerFrames[spinnerFrame % spinnerFrames.length]}</Span>
        <Span>loading</Span>
        <Span style={{width: 3}}>{dots[slow.frame % dots.length]}</Span>
      </Div>
      <Div style={{color: '#cbd5e1'}}>
        [{repeat('#', progress)}{repeat('-', 24 - progress)}] frame={frame} delta={Math.round(delta)}ms
      </Div>
      <Div style={{color: '#64748b'}}>q, Escape, or Ctrl-C exits</Div>
    </Div>
  );
}

const root = render(<SpinnerDemo />, {
  alternateScreen: true,
  captureCtrlC: true,
});

root.waitUntilExit().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});

function repeat(value: string, count: number): string {
  return value.repeat(Math.max(0, count));
}
