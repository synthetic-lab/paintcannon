import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { mock } from "antipattern";
import { PaintCannon, paintCannonDeps, type Notification } from "../main.ts";
import {
  createMockNativeBinding,
  keyDown,
  keyboardInput,
  type MockNativePaintCannon,
} from "./mock-native.ts";

let pc: PaintCannon;
let native: MockNativePaintCannon;
let restore: () => void;

beforeEach(() => {
  const instances: MockNativePaintCannon[] = [];
  restore = mock(paintCannonDeps, "loadNativeBinding", () => createMockNativeBinding(instances));
  pc = new PaintCannon();
  native = instances[0];
});
afterEach(() => {
  pc.stop();
  restore();
});

it("uses a scoped DOM constructor, cloned data, and asynchronous delivery", async () => {
  const data = { count: 1 };
  const notification = new pc.Notification("Complete", {
    body: "Build finished",
    tag: "build",
    data,
    silent: true,
    requireInteraction: true,
  });
  expect(notification.title).toBe("Complete");
  expect(notification.body).toBe("Build finished");
  expect(notification.data).toEqual(data);
  expect(notification.data).not.toBe(data);
  expect(native.notifications).toEqual([]);
  const show = vi.fn();
  notification.onshow = show;
  await Promise.resolve();
  expect(native.notifications).toEqual([
    {
      id: 1,
      options: {
        title: "Complete",
        body: "Build finished",
        tag: "build",
        silent: true,
        requireInteraction: true,
      },
    },
  ]);
  expect(show).not.toHaveBeenCalled();
  native.events.push({ kind: "notification", notification: { id: 1, type: "show" } });
  native.notifyEvents();
  expect(show).toHaveBeenCalledOnce();
  expect(show.mock.calls[0][0].target).toBe(notification);
});

it("auto-grants for an available backend and denies unavailable backends without prompting", async () => {
  expect(pc.Notification.permission).toBe("default");
  expect(pc.Notification.maxActions).toBe(0);
  expect(await pc.Notification.requestPermission()).toBe("granted");
  expect(pc.Notification.permission).toBe("granted");
  native.notificationsSupported = false;
  expect(await pc.Notification.requestPermission()).toBe("denied");
  expect(pc.Notification.permission).toBe("denied");
  native.notificationsSupported = true;
  expect(await pc.Notification.requestPermission()).toBe("granted");
  expect(native.notifications).toEqual([]);
  const notification = new pc.Notification("Title");
  expect(notification.body).toBe("");
  expect(notification.tag).toBe("");
  expect(notification.data).toBe(null);
  expect(notification.silent).toBe(null);
  expect(notification.requireInteraction).toBe(false);
});

it("queries programmatic dismissal separately from notification delivery", async () => {
  expect(await pc.Notification.requestPermission()).toBe("granted");
  expect(await pc.Notification.supportsClose()).toBe(false);
  native.notificationsCanClose = true;
  expect(await pc.Notification.supportsClose()).toBe(true);
  expect(native.notifications).toEqual([]);
  expect(native.closedNotifications).toEqual([]);
  pc.stop();
  expect(await pc.Notification.supportsClose()).toBe(false);
});

it("returns false if stopped during a dismissal-support query", async () => {
  let complete!: (supported: boolean) => void;
  vi.spyOn(native, "notificationSupportsClose").mockImplementation(
    () =>
      new Promise(resolve => {
        complete = resolve;
      }),
  );
  const query = pc.Notification.supportsClose();
  pc.stop();
  complete(true);
  expect(await query).toBe(false);
});

it("dispatches clicks in native order with keyboard events and supports once/removal", async () => {
  const notification = new pc.Notification("Title");
  await Promise.resolve();
  const received: string[] = [];
  pc.addEventListener("keydown", event => received.push(event.key));
  const removed = vi.fn();
  notification.addEventListener("click", removed);
  notification.removeEventListener("click", removed);
  notification.addEventListener(
    "click",
    event => {
      expect(event.target).toBe(notification);
      event.preventDefault();
      expect(event.defaultPrevented).toBe(true);
      received.push("click");
    },
    { once: true },
  );
  native.events.push(
    keyboardInput(keyDown("a")),
    { kind: "notification", notification: { id: 1, type: "click" } },
    keyboardInput(keyDown("b")),
    { kind: "notification", notification: { id: 1, type: "click" } },
  );
  native.notifyEvents();
  expect(received).toEqual(["a", "click", "b"]);
  expect(removed).not.toHaveBeenCalled();
});

it("can cancel before sending, and closes only once after sending", async () => {
  const cancelled = new pc.Notification("Never sent");
  cancelled.close();
  await Promise.resolve();
  expect(native.notifications).toEqual([]);
  const notification = new pc.Notification("Sent");
  await Promise.resolve();
  const close = vi.fn();
  notification.onclose = close;
  notification.close();
  expect(native.closedNotifications).toEqual([2]);
  expect(close).not.toHaveBeenCalled();
  native.events.push(
    { kind: "notification", notification: { id: 2, type: "close" } },
    { kind: "notification", notification: { id: 2, type: "close" } },
  );
  native.notifyEvents();
  notification.close();
  expect(close).toHaveBeenCalledOnce();
  expect(native.closedNotifications).toEqual([2]);
});

it("turns native failures into asynchronous error events and releases the registration", async () => {
  const notification = new pc.Notification("Failure");
  const error = vi.fn();
  notification.onerror = error;
  await Promise.resolve();
  native.events.push({
    kind: "notification",
    notification: { id: 1, type: "error", error: "notify-send missing" },
  });
  native.notifyEvents();
  expect(error.mock.calls[0][0].error.message).toBe("notify-send missing");
  expect(await pc.Notification.requestPermission()).toBe("granted");
  native.events.push({ kind: "notification", notification: { id: 1, type: "show" } });
  const show = vi.fn();
  notification.onshow = show;
  native.notifyEvents();
  expect(show).not.toHaveBeenCalled();
});

it("queues an error when native submission throws without crashing construction", async () => {
  vi.spyOn(native, "showNotification").mockImplementation(() => {
    throw new Error("queue full");
  });
  const notification = new pc.Notification("Title");
  const error = vi.fn();
  notification.onerror = error;
  await Promise.resolve();
  expect(error.mock.calls[0][0].error.message).toBe("queue full");
});

it("stopping cancels pending work and stops existing notification handles", async () => {
  const notification: Notification = new pc.Notification("Cancelled");
  pc.stop();
  await Promise.resolve();
  expect(native.notifications).toEqual([]);
  notification.close();
  expect(native.closedNotifications).toEqual([]);
  expect(await pc.Notification.requestPermission()).toBe("denied");
  expect(() => new pc.Notification("Stopped")).toThrow("stopped");
});

it("rejects non-cloneable data like the DOM", () => {
  expect(() => new pc.Notification("Title", { data: () => {} })).toThrow();
});
