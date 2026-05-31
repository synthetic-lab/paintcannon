import { PaintCannon } from '../index';

const pc = new PaintCannon({ fps: 30 });

const root = pc.createElement('div');
pc.setRoot(root);
root.style.display = 'flex';
root.style.flexDirection = 'row';
root.style.width = '100%';
root.style.height = '100%';
root.style.backgroundColor = 'black';

const left = pc.createElement('div');
left.style.display = 'flex';
left.style.flexDirection = 'column';
left.style.width = '30px';
left.style.height = '100%';
left.style.backgroundColor = 'blue';

const leftTop = pc.createElement('div');
leftTop.style.width = '30px';
leftTop.style.height = '33%';
leftTop.style.backgroundColor = 'cyan';

const leftBottom = pc.createElement('div');
leftBottom.style.width = '30px';
leftBottom.style.height = '67%';
leftBottom.style.backgroundColor = 'magenta';

left.appendChild(leftTop);
left.appendChild(leftBottom);

const right = pc.createElement('div');
right.style.display = 'flex';
right.style.flexDirection = 'column';
right.style.width = '100%';
right.style.height = '100%';
right.style.backgroundColor = 'green';

const rightTop = pc.createElement('div');
rightTop.style.width = '50px';
rightTop.style.height = '50%';
rightTop.style.backgroundColor = 'yellow';

const rightBottom = pc.createElement('div');
rightBottom.style.width = '50px';
rightBottom.style.height = '50%';
rightBottom.style.backgroundColor = 'red';

right.appendChild(rightTop);
right.appendChild(rightBottom);
root.appendChild(left);
root.appendChild(right);

let frames = 0;
const paint = () => {
  if (frames++ % 2 === 0) {
    rightBottom.style.backgroundColor = 'red';
  } else {
    rightBottom.style.backgroundColor = 'blue';
  }

  if (frames < 20) {
    pc.requestAnimationFrame(paint);
  } else {
    pc.stop();
  }
};

pc.requestAnimationFrame(paint);
