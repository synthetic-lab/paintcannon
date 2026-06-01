import { PaintCannon } from '../index.ts';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.width = '100%';
root.style.height = '100%';
root.style.padding = '1 2';
root.style.gap = '1';
root.style.backgroundColor = 'black';
pc.setRoot(root);

const title = pc.createElement('div');
title.style.height = '3';
title.style.borderBottom = 'solid';
title.style.borderColor = 'cyan';
title.appendChild(pc.createTextNode('Padding demo: shorthand and per-side padding'));

const shorthand = panel('padding: 1 4', 'blue');
shorthand.style.padding = '1 4';
shorthand.appendChild(
  pc.createTextNode('This text starts inside the padded content box, away from the border.'),
);

const perSide = panel('paddingTop/Right/Bottom/Left', 'green');
perSide.style.paddingTop = '1';
perSide.style.paddingRight = '8';
perSide.style.paddingBottom = '1';
perSide.style.paddingLeft = '2';
perSide.appendChild(pc.createTextNode('Left padding is small; right padding reduces the wrapping width.'));

root.appendChild(title);
root.appendChild(shorthand);
root.appendChild(perSide);

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

function panel(label: string, background: string) {
  const element = pc.createElement('div');
  element.style.width = '60';
  element.style.border = 'rounded';
  element.style.borderColor = 'white';
  element.style.backgroundColor = background;
  element.appendChild(pc.createTextNode(`${label}\n`));
  return element;
}
