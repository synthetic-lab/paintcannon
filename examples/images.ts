import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { PaintCannon } from '../index';

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
label.appendChild(pc.createTextNode('ASCII image rendering'));

const img = pc.createElement('img');
img.src = pngPath;
img.style.width = 32;
img.style.height = 12;

const hint = pc.createElement('div');
hint.style.color = '#90a4ae';
hint.appendChild(pc.createTextNode('press q or Escape to exit'));

root.appendChild(label);
root.appendChild(img);
root.appendChild(hint);
pc.setRoot(root);
pc.render();

pc.addEventListener('keydown', (event) => {
  if (event.key === 'q' || event.key === 'Escape') {
    pc.stop();
    process.exit(0);
  }
});
