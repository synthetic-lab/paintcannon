import React from "react";
import type { PaintCannon } from "paintcannon";

type AnimationCallback = (currentTime: number) => void;
type AnimationSubscription = {
  startTime: number;
  unsubscribe(): void;
};
type AnimationContextValue = {
  subscribe(callback: AnimationCallback, interval: number | undefined): AnimationSubscription;
};

export type AnimationOptions = {
  interval?: number;
  isActive?: boolean;
};

export type AnimationResult = {
  readonly frame: number;
  readonly time: number;
  readonly delta: number;
  readonly reset: () => void;
};

export const AnimationContext = React.createContext<AnimationContextValue | undefined>(undefined);

const maximumTimerInterval = 2_147_483_647;
const zeroAnimationState: Omit<AnimationResult, "reset"> = { frame: 0, time: 0, delta: 0 };

export function useAnimation(options: AnimationOptions = {}): AnimationResult {
  const { interval, isActive = true } = options;
  const safeInterval = interval === undefined ? undefined : normalizeAnimationInterval(interval);
  const animation = React.useContext(AnimationContext);
  if (animation === undefined) {
    throw new Error("useAnimation() must be used inside a paintcannon-react render tree");
  }

  const [resetKey, setResetKey] = React.useState(0);
  const [state, setState] = React.useState(zeroAnimationState);
  const lastRenderTimeRef = React.useRef(0);
  const previousOptionsRef = React.useRef({ isActive, safeInterval, resetKey });
  const previousOptions = previousOptionsRef.current;
  const shouldReset =
    isActive &&
    (safeInterval !== previousOptions.safeInterval ||
      !previousOptions.isActive ||
      resetKey !== previousOptions.resetKey);
  const reset = React.useCallback(() => {
    setResetKey(value => value + 1);
  }, []);

  React.useLayoutEffect(() => {
    if (!isActive) {
      return undefined;
    }

    setState(zeroAnimationState);
    let startTime = 0;
    const subscription = animation.subscribe(currentTime => {
      const elapsed = currentTime - startTime;
      const delta = currentTime - lastRenderTimeRef.current;
      lastRenderTimeRef.current = currentTime;
      setState(previous => ({
        frame: safeInterval === undefined ? previous.frame + 1 : Math.floor(elapsed / safeInterval),
        time: elapsed,
        delta,
      }));
    }, safeInterval);

    startTime = subscription.startTime;
    lastRenderTimeRef.current = subscription.startTime;
    return subscription.unsubscribe;
  }, [animation, isActive, safeInterval, resetKey]);

  React.useLayoutEffect(() => {
    previousOptionsRef.current = { isActive, safeInterval, resetKey };
  }, [isActive, safeInterval, resetKey]);

  if (shouldReset) {
    return { ...zeroAnimationState, reset };
  }

  return { ...state, reset };
}

export class AnimationScheduler implements AnimationContextValue {
  private nextId = 1;
  private animationFrameId: number | undefined;
  private readonly subscribers = new Map<
    number,
    {
      callback: AnimationCallback;
      interval: number | undefined;
      nextTime: number;
    }
  >();

  subscribe(callback: AnimationCallback, interval: number | undefined): AnimationSubscription {
    const id = this.nextId;
    this.nextId += 1;
    const startTime = performance.now();
    this.subscribers.set(id, {
      callback,
      interval,
      nextTime: interval === undefined ? startTime : startTime + interval,
    });
    this.scheduleAnimationFrame();

    return {
      startTime,
      unsubscribe: () => {
        this.subscribers.delete(id);
        if (this.subscribers.size === 0) {
          this.cancelAnimationFrame();
        }
      },
    };
  }

  constructor(private readonly paintCannon: PaintCannon) {}

  stop(): void {
    this.cancelAnimationFrame();
    this.subscribers.clear();
  }

  private scheduleAnimationFrame(): void {
    if (this.animationFrameId !== undefined || this.subscribers.size === 0) {
      return;
    }

    this.animationFrameId = this.paintCannon.requestAnimationFrame(timestamp => {
      this.animationFrameId = undefined;
      this.tick(timestamp);
    });
  }

  private cancelAnimationFrame(): void {
    if (this.animationFrameId === undefined) {
      return;
    }

    this.paintCannon.cancelAnimationFrame(this.animationFrameId);
    this.animationFrameId = undefined;
  }

  private tick(currentTime: number): void {
    for (const subscriber of this.subscribers.values()) {
      if (subscriber.interval === undefined) {
        subscriber.callback(currentTime);
        continue;
      }

      if (subscriber.nextTime <= currentTime) {
        const intervalsElapsed = Math.max(
          1,
          Math.floor((currentTime - subscriber.nextTime) / subscriber.interval) + 1,
        );
        subscriber.nextTime += subscriber.interval * intervalsElapsed;
        subscriber.callback(currentTime);
      }
    }
    this.scheduleAnimationFrame();
  }
}

function normalizeAnimationInterval(interval: number): number {
  if (!Number.isFinite(interval)) {
    return maximumTimerInterval;
  }

  return Math.min(maximumTimerInterval, Math.max(1, interval));
}
