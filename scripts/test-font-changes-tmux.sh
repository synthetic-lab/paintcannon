#!/usr/bin/env bash
# Opt-in integration test. Build paintcannon first; requires tmux and Python 3.
# Simulates terminal identities, not their font rendering. Not part of CI.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v tmux >/dev/null
exec python3 - <<'PY'
import json
import os
import pathlib
import pty
import select
import signal
import subprocess
import tempfile
import time
import uuid

binding = pathlib.Path('paintcannon/dist/main.js').resolve()
assert binding.is_file(), 'Run npm run build:debug --workspace paintcannon first'
socket = 'paintcannon-font-changes-' + uuid.uuid4().hex
tmux = ['tmux', '-L', socket, '-f', '/dev/null']
env = {key: value for key, value in os.environ.items()
       if key not in ('TMUX', 'TMUX_PANE', 'NERD_FONT')}
env.update(TERM='xterm-256color', TERM_PROGRAM='Apple_Terminal')
clients = []

def interrupted(signum, frame):
    raise KeyboardInterrupt

signal.signal(signal.SIGTERM, interrupted)

def run(*args):
    return subprocess.check_output(tmux + list(args), env=env, text=True)

def attach(program, term='xterm-256color'):
    master, slave = pty.openpty()
    tty = os.ttyname(slave)
    try:
        process = subprocess.Popen(tmux + ['attach-session', '-t', 'fonts'],
                                   stdin=slave, stdout=slave, stderr=slave,
                                   env={**env, 'TERM_PROGRAM': program, 'TERM': term})
    finally:
        os.close(slave)
    clients.append((master, process, tty, program, bytearray()))
    return tty

def drain():
    for master, process, _, program, output in clients:
        if select.select([master], [], [], 0)[0]:
            try:
                output.extend(os.read(master, 65536))
                if b'\x1b[>q' in output or b'\x1b[>0q' in output:
                    # Simulate these emulators' XTVERSION replies to tmux itself.
                    if program in ('ghostty', 'WezTerm'):
                        os.write(master, b'\x1bP>|' + program.encode() + b' 1.0\x1b\\')
                    output.clear()
                del output[:-16]
            except OSError:
                pass

with tempfile.TemporaryDirectory(prefix='paintcannon-font-changes-') as directory:
    log = pathlib.Path(directory) / 'events.jsonl'
    app = '''
        import { appendFileSync } from 'node:fs';
        import { PaintCannon } from %s;
        const pc = new PaintCannon({ alternateScreen: true });
        const log = event => appendFileSync(%s, JSON.stringify(event) + '\\n');
        for (const type of ['nerdfont-powerline-extended-change', 'symbols-for-legacy-computing-change']) {
          pc.font.addEventListener(type, event => log({ type: event.type, supported: event.supported }));
        }
        pc.transaction(() => {
          const root = pc.createElement('div');
          root.style.display = 'flex';
          root.style.flexDirection = 'column';
          for (const border of ['chunky', 'chunky-rounded', 'chunky-best-effort']) {
            const box = pc.createElement('div');
            box.style.width = 12;
            box.style.height = 3;
            box.style.flexShrink = 0;
            box.style.border = border;
            box.style.borderColor = '#00ff00';
            root.appendChild(box);
          }
          pc.setRoot(root);
        });
        process.on('SIGTERM', () => pc.stop());
        log({ type: 'ready' });
    ''' % (json.dumps(binding.as_uri()), json.dumps(str(log)))

    def wait_for(nerd, legacy, corners):
        deadline = time.monotonic() + 8
        state = {'nerdfont-powerline-extended-change': False,
                 'symbols-for-legacy-computing-change': False}
        screen = ''
        while time.monotonic() < deadline:
            drain()
            if log.exists():
                for line in log.read_text().splitlines():
                    event = json.loads(line)
                    if event['type'] in state:
                        state[event['type']] = event['supported']
            if list(state.values()) == [nerd, legacy]:
                screen = run('capture-pane', '-p', '-t', 'fonts:0.0')
                rows = screen.splitlines()
                if len(rows) >= 7 and [rows[row][:1] for row in (0, 3, 6)] == corners:
                    return
            time.sleep(.05)
        screen = run('capture-pane', '-p', '-t', 'fonts:0.0')
        live = run('list-clients', '-F', '#{client_control_mode}\t#{W:#{P:#{pane_id} }}\t#{client_termname}\t#{client_termtype}\t#{I/e:TERM_PROGRAM}')
        history = log.read_text() if log.exists() else '(no log)'
        raise AssertionError(f'Expected {nerd, legacy, corners}; got {state}; clients={live!r}; log={history!r}; screen={screen!r}')

    try:
        run('new-session', '-d', '-s', 'fonts', '-x', '80', '-y', '24',
            'node', '--input-type=module', '-e', app)
        run('set-option', '-g', 'allow-passthrough', 'on')
        ghostty = attach('ghostty')
        wait_for(True, True, ['\ue0ba', '\U0001fb41', '\U0001fb41'])
        print('PASS: live Ghostty client overrides stale Apple Terminal server environment', flush=True)

        apple = attach('Apple_Terminal')
        wait_for(False, False, ['\u2588'] * 3)
        print('PASS: attaching a less capable client downgrades JS events and actual border cells', flush=True)

        run('detach-client', '-t', apple)
        wait_for(True, True, ['\ue0ba', '\U0001fb41', '\U0001fb41'])
        print('PASS: detaching it restores capabilities and borders', flush=True)

        run('detach-client', '-t', ghostty)
        attach('WezTerm')
        wait_for(True, False, ['\ue0ba', '\u2588', '\ue0ba'])
        print('PASS: reattaching from WezTerm independently updates legacy glyph support', flush=True)
    finally:
        subprocess.run(tmux + ['kill-server'], env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for master, process, _, _, _ in clients:
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            os.close(master)
PY
