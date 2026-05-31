import { PaintCannon } from '../index';

const pc = new PaintCannon({
  alternateScreen: true,
  captureMouse: true,
  fps: 30,
});

const root = pc.createElement('div');
root.style.display = 'flex';
root.style.flexDirection = 'column';
root.style.justifyContent = 'center';
root.style.alignItems = 'center';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'black';
pc.setRoot(root);

const button = pc.createElement('div');
button.style.width = '24';
button.style.height = '5';
button.style.border = 'chunky-rounded';
button.style.borderColor = 'blue';
button.style.backgroundColor = 'blue';
button.appendChild(pc.createTextNode('  chunky rounded  '));
root.appendChild(button);

button.addEventListener('mouseenter', () => {
  button.style.borderColor = 'cyan';
  button.style.backgroundColor = 'cyan';
});

button.addEventListener('mouseleave', () => {
  button.style.borderColor = 'blue';
  button.style.backgroundColor = 'blue';
});

function tick() {
  pc.requestAnimationFrame(tick);
}

tick();
