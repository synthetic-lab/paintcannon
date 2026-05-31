import { spawnSync } from 'node:child_process';

const ESC = '\u001b';
const BEL = '\u0007';
const CSI = `${ESC}[`;
const ST = `${ESC}\\`;
const OSC = `${ESC}]`;

process.stdout.write(`${CSI}?25l`);
process.stdout.write('\nOSC 22 pointer test using the PaintCannon tmux escaping path.\n');
process.stdout.write(`TMUX=${process.env.TMUX ?? '(not set)'}\n`);
process.stdout.write(`tmux allow-passthrough: ${allowTmuxPassthrough()}\n\n`);
process.stdout.write('Expected: mouse cursor becomes a pointer for 5 seconds, then returns to default.\n');
process.stdout.write(writeTerminalSequence(osc22('pointer')));

setTimeout(() => {
  process.stdout.write(writeTerminalSequence(osc22('')));
  process.stdout.write(`${CSI}?25h`);
  process.stdout.write('\nDone.\n');
}, 5000);

function osc22(shape: string): string {
  return `${OSC}22;${shape}${BEL}`;
}

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
