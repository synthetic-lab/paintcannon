import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { PaintCannon } from '../index.ts';

const pngPath = join('/tmp', 'paintcannon-image-scroll-demo.png');
const pngBase64 =
  'iVBORw0KGgoAAAANSUhEUgAAABAAAAAICAIAAAB/FOjAAAAAI0lEQVR4nGNguPOf4c7//xoB/zUCiGKTrIEEpWA26RqGgR8AqQbUgRGi6GsAAAAASUVORK5CYII=';
writeFileSync(pngPath, Buffer.from(pngBase64, 'base64'));

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
root.style.backgroundColor = '#0f172a';
pc.setRoot(root);

const header = pc.createElement('div');
header.style.width = '100%';
header.style.height = 2;
header.style.flexShrink = 0;
header.style.backgroundColor = '#1f2937';
header.style.color = '#e5e7eb';

const status = pc.createTextNode('Image scroll demo. Wheel over the panel. Ctrl-C exits.');
header.appendChild(status);

const body = pc.createElement('div');
body.style.display = 'flex';
body.style.flexDirection = 'row';
body.style.width = '100%';
body.style.flex = '1 1 0px';
body.style.minHeight = 0;
body.style.gap = 1;

const viewport = pc.createElement('div');
viewport.style.width = '80%';
viewport.style.height = '100%';
viewport.style.minHeight = 0;
viewport.style.overflowY = 'scroll';
viewport.style.overflowX = 'hidden';
viewport.style.backgroundColor = '#111827';
viewport.style.border = 'rounded';
viewport.style.borderColor = '#94a3b8';

const content = pc.createElement('div');
content.style.display = 'flex';
content.style.flexDirection = 'column';
content.style.width = '100%';
content.style.gap = 1;

pc.transaction(() => {
  for (let index = 1; index <= 8; index += 1) {
    content.appendChild(row(`before image row ${index}`));
  }

  const firstImage = imageBlock('image A should clip against the scroll viewport');
  content.appendChild(firstImage);

  for (let index = 1; index <= 14; index += 1) {
    content.appendChild(row(`middle row ${index}`));
  }

  const secondImage = imageBlock('image B should scroll out like normal text');
  content.appendChild(secondImage);

  for (let index = 1; index <= 18; index += 1) {
    content.appendChild(row(`after image row ${index}`));
  }
});

const rail = pc.createElement('div');
rail.style.width = '20%';
rail.style.height = '100%';
rail.style.backgroundColor = '#020617';
rail.style.color = '#cbd5e1';
rail.style.whiteSpace = 'pre';

const scrollbar = pc.createTextNode(scrollbarText(0, 1, 1));
rail.appendChild(scrollbar);

viewport.appendChild(content);
body.appendChild(viewport);
body.appendChild(rail);
root.appendChild(header);
root.appendChild(body);

viewport.addEventListener('scroll', (event) => {
  updateScrollbar(event.scrollTop, event.scrollHeight, viewport.clientHeight, rail.clientHeight);
});

pc.addEventListener('resize', () => {
  updateScrollbar(viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight, rail.clientHeight);
});

pc.addEventListener('keydown', (event) => {
  if (event.key === 'q' || event.key === 'Escape') {
    pc.stop();
    process.exit(0);
  }
});

updateScrollbar(viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight, rail.clientHeight);
pc.render();

function row(text: string) {
  const element = pc.createElement('div');
  element.style.width = '100%';
  element.style.color = '#cbd5e1';
  element.appendChild(pc.createTextNode(text));
  return element;
}

function imageBlock(label: string) {
  const block = pc.createElement('div');
  block.style.display = 'flex';
  block.style.flexDirection = 'column';
  block.style.width = '100%';
  block.style.gap = 1;
  block.style.backgroundColor = '#1e293b';
  block.style.color = '#f8fafc';

  const title = pc.createElement('div');
  title.style.width = '100%';
  title.appendChild(pc.createTextNode(label));

  const image = pc.createElement('img');
  image.src = pngPath;
  image.style.width = 48;
  image.style.height = 14;

  block.appendChild(title);
  block.appendChild(image);
  return block;
}

function updateScrollbar(scrollTop: number, scrollHeight: number, clientHeight: number, railHeight: number): void {
  status.nodeValue = `scrollTop=${scrollTop}/${scrollHeight}, clientHeight=${clientHeight}`;
  scrollbar.nodeValue = scrollbarText(scrollTop, scrollHeight, railHeight);
}

function scrollbarText(scrollTop: number, scrollHeight: number, clientHeight: number): string {
  const height = Math.max(1, clientHeight);
  const max = Math.max(1, scrollHeight - clientHeight);
  const thumb = Math.min(height - 1, Math.floor((scrollTop / max) * (height - 1)));
  let text = '';
  for (let rowIndex = 0; rowIndex < height; rowIndex += 1) {
    text += rowIndex === thumb ? '#' : '|';
    if (rowIndex < height - 1) {
      text += '\n';
    }
  }
  return text;
}

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
