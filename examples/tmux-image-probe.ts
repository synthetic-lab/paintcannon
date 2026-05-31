import { spawnSync } from 'node:child_process';

const ESC = '\u001b';
const CSI = `${ESC}[`;
const ST = `${ESC}\\`;

const imageWidth = 16;
const imageHeight = 8;
const imagePayload = checkerboardPayload(imageWidth, imageHeight);
const imageCommand = `${ESC}_Ga=T,q=2,f=24,s=${imageWidth},v=${imageHeight};${imagePayload}${ST}`;
const clearCommand = `${ESC}_Ga=d${ST}`;

process.stdout.write(`${CSI}?25l`);
process.stdout.write('\nKitty graphics passthrough test using the PaintCannon tmux escaping path.\n');
process.stdout.write(`TMUX=${process.env.TMUX ?? '(not set)'}\n`);
process.stdout.write(`tmux allow-passthrough: ${allowTmuxPassthrough()}\n\n`);
process.stdout.write('Expected: one red/cyan checkerboard below.\n\n');
process.stdout.write(writeTerminalSequence(imageCommand));

setTimeout(() => {
  process.stdout.write('\n\nClearing image.\n');
  process.stdout.write(writeTerminalSequence(clearCommand));
  process.stdout.write(`${CSI}?25h`);
}, 5000);

function writeTerminalSequence(sequence: string): string {
  if (!process.env.TMUX) {
    return sequence;
  }

  return tmuxWrap(sequence);
}

function tmuxWrap(sequence: string): string {
  return `${ESC}Ptmux;${doubleEsc(sequence)}${ST}`;
}

function doubleEsc(sequence: string): string {
  return sequence.replaceAll(ESC, `${ESC}${ESC}`);
}

function allowTmuxPassthrough(): string {
  if (!process.env.TMUX) {
    return 'not in tmux';
  }

  const show = spawnSync('tmux', ['show', '-Ap', 'allow-passthrough'], { encoding: 'utf8' });
  const value = show.stdout.trim();
  if (show.status === 0 && (value.endsWith(' on') || value.endsWith(' all'))) {
    return value;
  }

  const set = spawnSync('tmux', ['set', '-p', 'allow-passthrough', 'on'], { encoding: 'utf8' });
  if (set.status !== 0) {
    return `failed to set: ${set.stderr.trim() || set.error?.message || `exit ${set.status}`}`;
  }

  const after = spawnSync('tmux', ['show', '-Ap', 'allow-passthrough'], { encoding: 'utf8' });
  return after.stdout.trim() || 'set on';
}

function checkerboardPayload(width: number, height: number): string {
  const bytes = Buffer.alloc(width * height * 3);
  let offset = 0;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const cyan = (Math.floor(x / 2) + Math.floor(y / 2)) % 2 === 0;
      bytes[offset] = cyan ? 0 : 255;
      bytes[offset + 1] = cyan ? 220 : 40;
      bytes[offset + 2] = cyan ? 255 : 80;
      offset += 3;
    }
  }
  return bytes.toString('base64');
}
