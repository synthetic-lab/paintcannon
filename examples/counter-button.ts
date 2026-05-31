import { PaintCannon } from '../index';

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  captureCtrlC: false,
  fps: 30,
});

let count = 0;

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';
root.style.gap = '1';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'black';
pc.setRoot(root);

const label = pc.createTextNode('Count: 0');

const button = pc.createElement('div');
button.style.display = 'flex';
button.style.justifyContent = 'center';
button.style.alignItems = 'center';
button.style.width = '24';
button.style.height = '5';
button.style.border = 'chunky-rounded';
button.style.borderColor = 'blue';
button.style.backgroundColor = 'blue';
button.style.color = 'white';
button.style.cursor = 'pointer';
button.appendChild(pc.createTextNode('Increment'));

button.addEventListener('mouseenter', () => {
  button.style.backgroundColor = 'cyan';
  button.style.borderColor = 'cyan';
  button.style.color = 'black';
});

button.addEventListener('mouseleave', () => {
  button.style.backgroundColor = 'blue';
  button.style.borderColor = 'blue';
  button.style.color = 'white';
});

button.addEventListener('click', () => {
  count += 1;
  label.nodeValue = `Count: ${count}`;
});

root.appendChild(label);
root.appendChild(button);

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
