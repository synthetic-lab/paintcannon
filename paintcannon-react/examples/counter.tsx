import React, {useState} from 'react';
import {Div, Span, render} from '../src/index.ts';

function Counter(): React.ReactElement {
  const [count, setCount] = useState(0);

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
    >
      <Span style={{color: '#38bdf8'}}>paintcannon-react</Span>
      <Div
        style={{
          border: 'chunky-rounded',
          borderColor: '#fb923c',
          backgroundColor: count % 2 === 0 ? '#0f172a' : '#7c2d12',
          color: '#f8fafc',
          padding: 1,
          cursor: 'pointer',
        }}
        onClick={() => {
          setCount(value => value + 1);
        }}
      >
        Clicked {count} times
      </Div>
    </Div>
  );
}

render(<Counter />, {
  alternateScreen: true,
  captureMouse: true,
});
