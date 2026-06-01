import React, {useState} from 'react';
import {Div, Span, render} from '../src/index.ts';

let root: ReturnType<typeof render> | undefined;

function KeyboardDemo(): React.ReactElement {
  const [lastEvent, setLastEvent] = useState('press a key');
  const [keydowns, setKeydowns] = useState(0);
  const [keyups, setKeyups] = useState(0);

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
        setKeydowns(value => value + 1);
        setLastEvent(`keydown key=${event.key} code=${event.code} repeat=${event.repeat ? 'yes' : 'no'}`);
        if (event.key === 'Escape') {
          event.preventDefault();
          root?.paintCannon.stop();
          process.exit(0);
        }
      }}
      onKeyUp={event => {
        setKeyups(value => value + 1);
        setLastEvent(`keyup key=${event.key} code=${event.code}`);
      }}
    >
      <Span style={{color: '#38bdf8'}}>paintcannon-react keyboard events</Span>
      <Div
        style={{
          width: 58,
          padding: '1 2',
          border: 'rounded',
          borderColor: '#334155',
          backgroundColor: '#0f172a',
          color: '#f8fafc',
        }}
      >
        {lastEvent}
      </Div>
      <Div style={{color: '#cbd5e1'}}>
        keydown {keydowns} | keyup {keyups} | Escape exits
      </Div>
    </Div>
  );
}

root = render(<KeyboardDemo />, {
  alternateScreen: true,
});
