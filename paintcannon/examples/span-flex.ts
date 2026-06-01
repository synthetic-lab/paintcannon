import { PaintCannon } from '../index';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.alignItems = 'center';
root.style.justifyContent = 'center';
root.style.gap = '1';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = '#020617';
root.style.color = '#e2e8f0';
pc.setRoot(root);

const title = pc.createElement('span');
title.style.color = '#38bdf8';
title.appendChild(pc.createTextNode('direct span flex child'));

const button = pc.createElement('div');
button.style.border = 'chunky-rounded';
button.style.borderColor = '#fb923c';
button.style.backgroundColor = '#0f172a';
button.style.color = '#f8fafc';
button.style.padding = '1 4';
button.appendChild(pc.createTextNode('The span above should be visible'));

root.appendChild(title);
root.appendChild(button);

let frames = 0;
function tick() {
  frames += 1;
  if (frames >= 90) {
    pc.stop();
    return;
  }
  pc.requestAnimationFrame(tick);
}

tick();
