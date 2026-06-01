import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { PaintCannon } from '../index.ts';

const pngPath = join('/tmp', 'paintcannon-image-demo.png');
const pngBase64 =
  'iVBORw0KGgoAAAANSUhEUgAAABAAAAAICAIAAAB/FOjAAAAAI0lEQVR4nGNguPOf4c7//xoB/zUCiGKTrIEEpWA26RqGgR8AqQbUgRGi6GsAAAAASUVORK5CYII=';
writeFileSync(pngPath, Buffer.from(pngBase64, 'base64'));

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
root.style.width = '100%';
root.style.height = '100%';
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.alignItems = 'center';
root.style.justifyContent = 'center';
root.style.gap = 1;
root.style.backgroundColor = '#101820';

const label = pc.createElement('div');
label.style.color = '#e8f1f2';
label.appendChild(pc.createTextNode('half-block and ASCII image rendering'));

const row = pc.createElement('div');
row.style.display = 'flex';
row.style.flexDirection = 'row';
row.style.gap = 4;

const halfBlockColumn = imageColumn('half-block default', 'half-block');
const asciiColumn = imageColumn('ASCII fallback', 'ascii');
row.appendChild(halfBlockColumn);
row.appendChild(asciiColumn);

const hint = pc.createElement('div');
hint.style.color = '#90a4ae';
hint.appendChild(pc.createTextNode('press q or Escape to exit'));

root.appendChild(label);
root.appendChild(row);
root.appendChild(hint);
pc.setRoot(root);
pc.render();

pc.addEventListener('keydown', (event) => {
  if (event.key === 'q' || event.key === 'Escape') {
    pc.stop();
    process.exit(0);
  }
});

function imageColumn(labelText: string, rendering: 'ascii' | 'half-block') {
  const column = pc.createElement('div');
  column.style.display = 'flex';
  column.style.flexDirection = 'column';
  column.style.alignItems = 'center';
  column.style.gap = 1;

  const title = pc.createElement('div');
  title.style.color = '#e8f1f2';
  title.appendChild(pc.createTextNode(labelText));

  const img = pc.createElement('img');
  img.src = pngPath;
  img.style.width = 28;
  img.style.height = 10;
  img.style.imageRendering = rendering;

  column.appendChild(title);
  column.appendChild(img);
  return column;
}
