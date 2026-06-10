import { PaintCannon, type KeyboardEvent } from "../main.ts";

const pc = new PaintCannon({ fps: 30 });

const ballStepMs = 90;
const paddleCellsPerSecond = 24;
const fallbackPaddleStep = 2;
const winningScore = 10;

let { cols: width, rows: height } = pc.terminalSize;
let paddleHeight = paddleHeightFor(height);
let leftY = Math.floor((height - paddleHeight) / 2);
let rightY = leftY;
let ballX = Math.floor(width / 2);
let ballY = Math.floor(height / 2);
let ballVx = 1;
let ballVy = 1;
let leftScore = 0;
let rightScore = 0;
let lastBallStep = 0;
let lastFrame = 0;
let winner: "left" | "right" | undefined;
const heldKeys = new Set<string>();

const root = pc.createElement("div");
pc.setRoot(root);
root.style.width = "100%";
root.style.height = "100%";
root.style.backgroundColor = "black";

const panel = pc.createElement("div");
panel.style.backgroundColor = "black";
panel.style.width = `${width}px`;
panel.style.height = `${height}px`;
panel.style.whiteSpace = "pre";

const frame = pc.createTextNode(renderFrame());
panel.appendChild(frame);
root.appendChild(panel);

pc.addEventListener("keydown", onKeyDown);
if (pc.kittyKeyboardEnabled) {
  pc.addEventListener("keyup", onKeyUp);
}
pc.requestAnimationFrame(tick);

process.once("SIGINT", () => {
  pc.stop();
  process.exit(130);
});

function onKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    pc.stop();
    return;
  }

  if (pc.kittyKeyboardEnabled) {
    heldKeys.add(event.code);
  } else {
    movePaddleForKeydown(event.code);
  }
}

function onKeyUp(event: KeyboardEvent) {
  heldKeys.delete(event.code);
}

function tick(timestamp: number) {
  if (winner) {
    frame.nodeValue = `${winner === "left" ? "Left" : "Right"} side won ${leftScore}-${rightScore}`;
    pc.requestAnimationFrame(() => {
      setTimeout(() => pc.stop(), 750);
    });
    return;
  }

  resizeToTerminal();
  if (pc.kittyKeyboardEnabled) {
    moveHeldPaddles(timestamp);
  }
  if (timestamp - lastBallStep >= ballStepMs) {
    stepBall();
    lastBallStep = timestamp;
  }

  frame.nodeValue = renderFrame();
  pc.requestAnimationFrame(tick);
}

function moveHeldPaddles(timestamp: number) {
  const elapsedSeconds = lastFrame === 0 ? 0 : (timestamp - lastFrame) / 1000;
  lastFrame = timestamp;
  const amount = paddleCellsPerSecond * elapsedSeconds;

  if (heldKeys.has("KeyW")) {
    leftY -= amount;
  }
  if (heldKeys.has("KeyS")) {
    leftY += amount;
  }
  if (heldKeys.has("ArrowUp")) {
    rightY -= amount;
  }
  if (heldKeys.has("ArrowDown")) {
    rightY += amount;
  }

  leftY = clamp(leftY, 1, height - paddleHeight - 1);
  rightY = clamp(rightY, 1, height - paddleHeight - 1);
}

function movePaddleForKeydown(code: string) {
  switch (code) {
    case "KeyW":
      leftY -= fallbackPaddleStep;
      break;
    case "KeyS":
      leftY += fallbackPaddleStep;
      break;
    case "ArrowUp":
      rightY -= fallbackPaddleStep;
      break;
    case "ArrowDown":
      rightY += fallbackPaddleStep;
      break;
  }

  leftY = clamp(leftY, 1, height - paddleHeight - 1);
  rightY = clamp(rightY, 1, height - paddleHeight - 1);
}

function stepBall() {
  ballX += ballVx;
  ballY += ballVy;

  if (ballY <= 0) {
    ballY = 0;
    ballVy = 1;
  } else if (ballY >= height - 1) {
    ballY = height - 1;
    ballVy = -1;
  }

  const leftPaddleY = Math.round(leftY);
  const rightPaddleY = Math.round(rightY);

  if (ballX === 1 && ballVx < 0 && ballY >= leftPaddleY && ballY < leftPaddleY + paddleHeight) {
    ballX = 1;
    ballVx = 1;
    ballVy = paddleBounce(ballY, leftPaddleY);
  }

  if (
    ballX === width - 2 &&
    ballVx > 0 &&
    ballY >= rightPaddleY &&
    ballY < rightPaddleY + paddleHeight
  ) {
    ballX = width - 2;
    ballVx = -1;
    ballVy = paddleBounce(ballY, rightPaddleY);
  }

  if (ballX < 0) {
    rightScore += 1;
    resetBall(-1);
  } else if (ballX >= width) {
    leftScore += 1;
    resetBall(1);
  }

  if (leftScore >= winningScore) {
    winner = "left";
  } else if (rightScore >= winningScore) {
    winner = "right";
  }
}

function resetBall(direction: number) {
  ballX = Math.floor(width / 2);
  ballY = Math.floor(height / 2);
  ballVx = direction;
  ballVy = Math.random() > 0.5 ? 1 : -1;
  leftY = Math.floor((height - paddleHeight) / 2);
  rightY = leftY;
}

function paddleBounce(y: number, paddleY: number) {
  const hit = y - paddleY;
  if (hit === 0) return -1;
  if (hit === paddleHeight - 1) return 1;
  return ballVy;
}

function renderFrame() {
  const rows = Array.from({ length: height }, () => Array.from({ length: width }, () => " "));

  for (let x = 0; x < width; x++) {
    rows[0][x] = "-";
    rows[height - 1][x] = "-";
  }

  for (let y = 1; y < height - 1; y++) {
    rows[y][Math.floor(width / 2)] = y % 2 === 0 ? ":" : " ";
  }

  for (let index = 0; index < paddleHeight; index++) {
    rows[Math.round(leftY) + index][0] = "#";
    rows[Math.round(rightY) + index][width - 1] = "#";
  }

  if (ballX >= 0 && ballX < width && ballY >= 0 && ballY < height) {
    rows[ballY][ballX] = "o";
  }

  const score = `Left ${leftScore}   Right ${rightScore}`;
  const scoreX = Math.floor((width - score.length) / 2);
  for (let i = 0; i < score.length; i++) {
    rows[1][scoreX + i] = score[i];
  }

  const help = "W/S vs Up/Down";
  const helpX = Math.floor((width - help.length) / 2);
  for (let i = 0; i < help.length; i++) {
    rows[height - 2][helpX + i] = help[i];
  }

  return rows.map(row => row.join("")).join("\n");
}

function resizeToTerminal() {
  const size = pc.terminalSize;
  const nextWidth = Math.max(20, size.cols);
  const nextHeight = Math.max(10, size.rows);
  if (nextWidth === width && nextHeight === height) {
    return;
  }

  const oldWidth = width;
  const oldHeight = height;
  width = nextWidth;
  height = nextHeight;
  paddleHeight = paddleHeightFor(height);
  panel.style.width = `${width}px`;
  panel.style.height = `${height}px`;
  leftY = clamp(leftY, 1, height - paddleHeight - 1);
  rightY = clamp(rightY, 1, height - paddleHeight - 1);
  ballX = clamp(Math.round((ballX / oldWidth) * width), 1, width - 2);
  ballY = clamp(Math.round((ballY / oldHeight) * height), 1, height - 2);
}

function paddleHeightFor(rows: number) {
  return clamp(Math.floor(rows / 5), 3, 8);
}

function clamp(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, value));
}
