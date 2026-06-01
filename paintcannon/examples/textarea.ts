import { PaintCannon, type KeyboardEvent } from '../index.ts';

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';
root.style.width = '100%';
root.style.height = '100%';
root.style.gap = 1;
root.style.backgroundColor = '#0f172a';
root.style.color = '#e2e8f0';
pc.setRoot(root);

const title = pc.createElement('div');
title.style.color = '#93c5fd';
title.appendChild(pc.createTextNode('Textarea: auto height, minHeight=5. Enter inserts a newline. Escape exits.'));

const textarea = pc.createElement('textarea');
textarea.value = 'paintcannon textarea with enough text to soft-wrap inside a forty-eight cell box';
textarea.placeholder = 'type a long message here';
textarea.cursorToEnd();
textarea.style.width = 48;
textarea.style.minHeight = 5;
textarea.style.backgroundColor = '#020617';
textarea.style.color = '#f8fafc';
textarea.style.placeholderColor = '#64748b';
textarea.style.border = 'rounded';
textarea.style.borderColor = '#64748b';

const status = pc.createElement('div');
status.style.width = 48;
status.style.height = 1;
status.style.color = '#cbd5e1';
status.appendChild(pc.createTextNode('type long lines or press Enter to add rows'));

root.appendChild(title);
root.appendChild(textarea);
root.appendChild(status);
textarea.focus();
pc.render();

pc.addEventListener('keydown', (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    pc.stop();
    process.exit(0);
  }

});

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
