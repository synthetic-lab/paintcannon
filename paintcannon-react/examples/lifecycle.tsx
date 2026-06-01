import React from 'react';
import {PaintCannon} from 'paintcannon';
import {Div, Span, render, useApp} from '../src';

function LifecycleDemo(): React.ReactElement {
  const {exit} = useApp();

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
          exit('goodbye from the main screen');
        }
      }}
    >
      <Span style={{color: '#38bdf8'}}>paintcannon-react lifecycle</Span>
      <Div
        style={{
          border: 'rounded',
          borderColor: '#334155',
          backgroundColor: '#0f172a',
          color: '#f8fafc',
          padding: '1 2',
        }}
      >
        Press q, Escape, or Ctrl-C to exit alt-screen.
      </Div>
    </Div>
  );
}

async function main(): Promise<void> {
  const root = render(<LifecycleDemo />, {
    alternateScreen: true,
    captureCtrlC: true,
  });
  const message = await root.waitUntilExit();

  const pc = new PaintCannon({ fps: 10 });
  const line = pc.createElement('div');
  line.style.width = '100%';
  line.style.height = 1;
  line.style.backgroundColor = '#111827';
  line.style.color = '#f9fafb';
  line.appendChild(pc.createTextNode(String(message ?? 'goodbye')));
  pc.setRoot(line);
  pc.render();
  pc.stop();
}

main().catch((error: unknown) => {
  console.error(error);
  process.exit(1);
});
