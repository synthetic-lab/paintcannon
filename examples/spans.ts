import { PaintCannon } from '../index';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'flex';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'black';

const paragraph = pc.createElement('div');
paragraph.style.width = '58px';
paragraph.style.height = '8px';
paragraph.style.backgroundColor = 'blue';

paragraph.appendChild(pc.createTextNode('Inline text can contain '));

const hot = pc.createElement('span');
hot.style.backgroundColor = 'red';
hot.appendChild(pc.createTextNode('styled spans'));
paragraph.appendChild(hot);

paragraph.appendChild(pc.createTextNode(' that keep flowing with surrounding text and wrap across terminal cells. '));

const cool = pc.createElement('span');
cool.style.backgroundColor = 'cyan';
cool.appendChild(pc.createTextNode('Nested '));

const nested = pc.createElement('span');
nested.style.backgroundColor = 'magenta';
nested.appendChild(pc.createTextNode('inline'));
cool.appendChild(nested);

cool.appendChild(pc.createTextNode(' spans'));
paragraph.appendChild(cool);
paragraph.appendChild(pc.createTextNode(' work too.'));

root.appendChild(paragraph);

pc.requestAnimationFrame(() => {
  pc.requestAnimationFrame(() => pc.stop());
});
