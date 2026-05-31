import { PaintCannon } from '../index';

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'black';
pc.setRoot(root);

const header = pc.createElement('div');
header.style.width = '100%';
header.style.height = '10%';
header.style.backgroundColor = 'cyan';

const status = pc.createTextNode('Percent scroll demo. Resize the terminal; wheel over the panel. Ctrl-C exits.');
header.appendChild(status);

const body = pc.createElement('div');
body.style.display = 'flex';
body.style.flexDirection = 'row';
body.style.width = '100%';
body.style.height = '90%';

const viewport = pc.createElement('div');
viewport.style.width = '85%';
viewport.style.height = '100%';
viewport.style.overflowY = 'scroll';
viewport.style.overflowX = 'hidden';
viewport.style.backgroundColor = 'blue';
viewport.style.selectionBackgroundColor = 'yellow';

const rail = pc.createElement('div');
rail.style.width = '15%';
rail.style.height = '100%';
rail.style.backgroundColor = 'magenta';

const content = pc.createElement('div');
content.style.display = 'flex';
content.style.flexDirection = 'column';
content.style.width = '100%';

const rowCount = 10_000;
for (let index = 1; index <= rowCount; index += 1) {
  const line = pc.createElement('div');
  line.style.width = '100%';
  line.appendChild(pc.createTextNode(`percent row ${String(index).padStart(2, '0')} - resize changes visible content`));
  content.appendChild(line);
}

const scrollbar = pc.createTextNode(scrollbarText(0, rowCount, 1));
rail.appendChild(scrollbar);

viewport.appendChild(content);
body.appendChild(viewport);
body.appendChild(rail);
root.appendChild(header);
root.appendChild(body);

viewport.addEventListener('scroll', (event) => {
  const clientHeight = viewport.clientHeight;
  status.nodeValue = `scrollTop=${event.scrollTop}/${event.scrollHeight}, clientHeight=${clientHeight}`;
  scrollbar.nodeValue = scrollbarText(event.scrollTop, event.scrollHeight, clientHeight);
});

function scrollbarText(scrollTop: number, scrollHeight: number, clientHeight: number): string {
  const height = Math.max(1, clientHeight);
  const max = Math.max(1, scrollHeight - clientHeight);
  const thumb = Math.min(height - 1, Math.floor((scrollTop / max) * (height - 1)));
  let text = '';
  for (let row = 0; row < height; row += 1) {
    text += row === thumb ? '#' : '|';
    if (row < height - 1) {
      text += '\n';
    }
  }
  return text;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
