import { PaintCannon } from '../index';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'grid';
root.style.width = '100%';
root.style.height = '100%';
root.style.gridTemplateColumns = '18px 1fr 2fr';
root.style.gridTemplateRows = '3px 1fr 4px';
root.style.gridAutoRows = '3px';
root.style.gridAutoColumns = '12px';
root.style.gridAutoFlow = 'row dense';
root.style.gap = '1px 2px';
root.style.justifyItems = 'stretch';
root.style.alignItems = 'stretch';
root.style.backgroundColor = 'black';

const header = panel('header: grid-column 1 / 4', 'blue');
header.style.gridColumn = '1 / 4';
header.style.gridRow = '1 / 2';
header.style.justifySelf = 'stretch';
header.style.alignSelf = 'stretch';

const sidebar = panel('sidebar', 'cyan');
sidebar.style.gridColumn = '1 / 2';
sidebar.style.gridRow = '2 / 3';

const main = panel('main spans two columns', 'green');
main.style.gridColumn = '2 / 4';
main.style.gridRow = '2 / 3';

const footer = panel('footer', 'magenta');
footer.style.gridColumn = '1 / 4';
footer.style.gridRow = '3 / 4';

const autoOne = panel('auto row', 'yellow');
const autoTwo = panel('span 2 cols', 'red');
autoTwo.style.gridColumn = 'span 2';

root.appendChild(header);
root.appendChild(sidebar);
root.appendChild(main);
root.appendChild(footer);
root.appendChild(autoOne);
root.appendChild(autoTwo);

paintOnce();

function panel(text: string, color: string) {
  const element = pc.createElement('div');
  element.style.display = 'flex';
  element.style.justifyContent = 'center';
  element.style.alignItems = 'center';
  element.style.backgroundColor = color;
  element.appendChild(pc.createTextNode(text));
  return element;
}

function paintOnce() {
  pc.requestAnimationFrame(() => {
    pc.requestAnimationFrame(() => {
      pc.stop();
    });
  });
}
