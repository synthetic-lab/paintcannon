import { PaintCannon, type KeyboardEvent } from '../index';

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
title.appendChild(pc.createTextNode('Textarea: Shift-Enter inserts a newline. Enter submits. Escape exits.'));

const textarea = pc.createElement('textarea');
textarea.value = 'paintcannon textarea';
textarea.cursorToEnd();
textarea.style.width = 48;
textarea.style.minHeight = 5;
textarea.style.backgroundColor = '#020617';
textarea.style.color = '#f8fafc';
textarea.style.border = 'rounded';
textarea.style.borderColor = '#64748b';

const submitted = pc.createElement('div');
submitted.style.width = 48;
submitted.style.height = 3;
submitted.style.color = '#cbd5e1';
const submittedText = pc.createTextNode('submitted: ');
submitted.appendChild(submittedText);

root.appendChild(title);
root.appendChild(textarea);
root.appendChild(submitted);
textarea.focus();
pc.render();

pc.addEventListener('keydown', (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    pc.stop();
    process.exit(0);
  }

  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    submittedText.nodeValue = `submitted: ${textarea.value.replace(/\n/g, ' / ')}`;
    textarea.value = '';
  }
});

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
