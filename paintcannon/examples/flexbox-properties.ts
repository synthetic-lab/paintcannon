import { PaintCannon } from '../index.ts';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.width = '100%';
root.style.height = '100%';
root.style.gap = '1px';
root.style.backgroundColor = 'black';

const growRow = pc.createElement('div');
growRow.style.display = 'flex';
growRow.style.flexDirection = 'row';
growRow.style.alignItems = 'center';
growRow.style.justifyContent = 'space-between';
growRow.style.width = '100%';
growRow.style.height = '7px';
growRow.style.columnGap = '2px';
growRow.style.backgroundColor = 'blue';

const fixed = box('fixed\nbasis 14', 'cyan');
fixed.style.flexBasis = '14px';
fixed.style.flexGrow = 0;
fixed.style.flexShrink = 0;

const growOne = box('grow 1\nbasis 0', 'green');
growOne.style.flex = '1 1 0px';

const growTwo = box('grow 2\nbasis 0', 'yellow');
growTwo.style.flexGrow = 2;
growTwo.style.flexShrink = 1;
growTwo.style.flexBasis = '0px';

growRow.appendChild(fixed);
growRow.appendChild(growOne);
growRow.appendChild(growTwo);

const wrapRow = pc.createElement('div');
wrapRow.style.display = 'flex';
wrapRow.style.flexFlow = 'row wrap';
wrapRow.style.alignContent = 'space-around';
wrapRow.style.alignItems = 'center';
wrapRow.style.justifyContent = 'center';
wrapRow.style.width = '100%';
wrapRow.style.height = '10px';
wrapRow.style.gap = '1px 3px';
wrapRow.style.backgroundColor = 'magenta';

for (let index = 1; index <= 8; index++) {
  const item = box(`wrap ${index}`, index % 2 === 0 ? 'red' : 'white');
  item.style.width = '15px';
  item.style.height = index === 3 ? '5px' : '3px';
  if (index === 3) {
    item.style.alignSelf = 'flex-end';
  }
  wrapRow.appendChild(item);
}

const reverseRow = pc.createElement('div');
reverseRow.style.display = 'flex';
reverseRow.style.flexDirection = 'row-reverse';
reverseRow.style.justifyContent = 'space-evenly';
reverseRow.style.alignItems = 'center';
reverseRow.style.width = '100%';
reverseRow.style.height = '5px';
reverseRow.style.backgroundColor = 'cyan';

reverseRow.appendChild(label('A'));
reverseRow.appendChild(label('B'));
reverseRow.appendChild(label('C'));

root.appendChild(growRow);
root.appendChild(wrapRow);
root.appendChild(reverseRow);

paintOnce();

function box(text: string, color: string) {
  const element = pc.createElement('div');
  element.style.display = 'flex';
  element.style.justifyContent = 'center';
  element.style.alignItems = 'center';
  element.style.height = '100%';
  element.style.backgroundColor = color;
  element.appendChild(pc.createTextNode(text));
  return element;
}

function label(text: string) {
  const element = pc.createElement('div');
  element.style.display = 'flex';
  element.style.justifyContent = 'center';
  element.style.alignItems = 'center';
  element.style.width = '12px';
  element.style.height = '3px';
  element.style.backgroundColor = 'blue';
  element.appendChild(pc.createTextNode(`reverse ${text}`));
  return element;
}

function paintOnce() {
  pc.requestAnimationFrame(() => {
    pc.requestAnimationFrame(() => {
      pc.stop();
    });
  });
}
