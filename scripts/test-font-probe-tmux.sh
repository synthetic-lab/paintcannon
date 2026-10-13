#!/usr/bin/env bash
# Opt-in integration test: requires tmux and Python 3, not part of CI.
set -euo pipefail
command -v tmux >/dev/null
exec python3 - <<'PY'
import os
import pty
import select
import signal
import subprocess
import time
import uuid

socket = 'paintcannon-font-' + uuid.uuid4().hex
os.environ.pop('TMUX', None)
tmux = ['tmux', '-L', socket, '-f', '/dev/null']
def interrupted(signum, frame):
    raise KeyboardInterrupt
signal.signal(signal.SIGTERM, interrupted)
child = r'''
import os, select, time, tty
tty.setraw(0)
os.write(1, b'PROBE-READY')
while os.read(0, 1) != b'G': pass
payload = b'\x1b_25a1;q;cp=e0a0\x1b\\\x1b[5n'
os.write(1, b'\x1bPtmux;' + payload.replace(b'\x1b', b'\x1b\x1b') + b'\x1b\\')
data = b''
status_received = False
deadline = time.monotonic() + 5
while time.monotonic() < deadline:
    if select.select([0], [], [], .05)[0]:
        data += os.read(0, 4096)
        if b'\x1b[0n' in data and not status_received:
            status_received = True
            os.write(1, b'STATUS-RECEIVED')
        if status_received and b'\x1b_25a1;q;cp=e0a0;status=system\x1b\\' in data:
            os.write(1, b'HEARTBEAT-RECEIVED')
            time.sleep(2)
            break
else:
    os.write(1, b'HEARTBEAT-TIMEOUT')
    time.sleep(2)
'''
master, slave = pty.openpty()
process = None
try:
    subprocess.run(tmux + ['new-session', '-d', '-s', 'probe',
                          'python3', '-c', child], check=True)
    subprocess.run(tmux + ['set-option', '-g', 'allow-passthrough', 'on'], check=True)
    process = subprocess.Popen(tmux + ['attach-session', '-t', 'probe'],
                               stdin=slave, stdout=slave, stderr=slave,
                               env={**os.environ, 'TERM': 'xterm-256color'})
    os.close(slave)
    slave = None
    output = b''
    deadline = time.monotonic() + 5
    while b'PROBE-READY' not in output and time.monotonic() < deadline:
        if select.select([master], [], [], .05)[0]:
            output += os.read(master, 65536)
    assert b'PROBE-READY' in output, 'probe did not start: ' + repr(output)
    os.write(master, b'G')
    output = b''
    deadline = time.monotonic() + 5
    while b'\x1b[5n' not in output and time.monotonic() < deadline:
        if select.select([master], [], [], .05)[0]:
            output += os.read(master, 65536)
    assert b'\x1b_25a1;q;cp=e0a0\x1b\\\x1b[5n' in output, repr(output)
    # Simulate a remote terminal. A locally answered query would return too early.
    until = time.monotonic() + .15
    while time.monotonic() < until:
        if select.select([master], [], [], .01)[0]:
            output += os.read(master, 65536)
    assert b'STATUS-RECEIVED' not in output, 'tmux answered locally'
    os.write(master, b'\x1b_25a1;q;cp=e0a0;status=system\x1b\\\x1b[0n')
    deadline = time.monotonic() + 3
    while b'HEARTBEAT-RECEIVED' not in output and time.monotonic() < deadline:
        if select.select([master], [], [], .05)[0]:
            output += os.read(master, 65536)
    assert b'HEARTBEAT-RECEIVED' in output, 'outer terminal reply did not reach pane: ' + repr(output)
    print('PASS: Rio query and DSR reach the outer terminal; both delayed replies reach the pane.')
finally:
    subprocess.run(tmux + ['kill-server'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if process is not None:
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
    os.close(master)
    if slave is not None:
        os.close(slave)
PY
