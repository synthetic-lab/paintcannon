import { PaintCannon, type KeyboardEvent } from '../index';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';
root.style.width = '100%';
root.style.height = '100%';
root.style.gap = '1px';
root.style.backgroundColor = 'black';

const title = pc.createElement('div');
title.style.display = 'flex';
title.style.justifyContent = 'center';
title.style.alignItems = 'center';
title.style.width = '90px';
title.style.height = '3px';
title.style.backgroundColor = 'blue';
title.appendChild(
  pc.createTextNode(
    `press keys - q or Escape exits - kitty=${pc.kittyKeyboardEnabled ? 'yes' : 'no'}`,
  ),
);

const status = pc.createElement('div');
status.style.display = 'flex';
status.style.justifyContent = 'center';
status.style.alignItems = 'center';
status.style.width = '110px';
status.style.height = '3px';
status.style.backgroundColor = 'green';

const statusText = pc.createTextNode('waiting for keyboard input');
status.appendChild(statusText);

root.appendChild(title);
root.appendChild(status);

let eventCount = 0;

function onKeyboardEvent(event: KeyboardEvent) {
  eventCount += 1;
  statusText.nodeValue = `${eventCount}: type=${event.type} key=${event.key} code=${event.code} repeat=${event.repeat} alt=${event.altKey} meta=${event.metaKey} shift=${event.shiftKey}`;

  if (event.key === 'q' || event.key === 'Escape') {
    pc.stop();
  }
}

pc.addEventListener('keydown', onKeyboardEvent);
pc.addEventListener('keyup', onKeyboardEvent);

process.once('SIGINT', () => {
  pc.stop();
  process.exit(130);
});

function paint() {
  pc.requestAnimationFrame(paint);
}

pc.requestAnimationFrame(paint);
