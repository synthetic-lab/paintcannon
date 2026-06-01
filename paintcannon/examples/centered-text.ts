import { PaintCannon } from '../index.ts';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'flex';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'blue';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';

const text = pc.createTextNode('centered with flexbox');
root.appendChild(text);

pc.requestAnimationFrame(() => {
  pc.requestAnimationFrame(() => {
    pc.stop();
  });
});
