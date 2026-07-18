import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { mock } from "antipattern";
import { PaintCannon, paintCannonDeps } from "../main.ts";
import { createMockNativeBinding, type MockNativePaintCannon } from "./mock-native.ts";

let restores: Array<() => void> = [];
let mockNativeInstances: MockNativePaintCannon[] = [];

beforeEach(() => {
  mockNativeInstances = [];
  restores = [
    mock(paintCannonDeps, "loadNativeBinding", () => createMockNativeBinding(mockNativeInstances)),
  ];
});

afterEach(() => {
  for (const restore of restores.reverse()) {
    restore();
  }
  restores = [];
});

describe("process lifecycle", () => {
  it("keeps rendering after an application handles an unhandled rejection notification", () => {
    const paintCannon = new PaintCannon();
    const native = mockNativeInstances.at(-1)!;
    const applicationHandler = (): void => {};
    process.on("unhandledRejection", applicationHandler);

    try {
      process.emit("unhandledRejection", new Error("handled by application"), Promise.resolve());

      expect(native.stopCalls).toBe(0);
      expect(() => {
        paintCannon.beginTransaction();
        paintCannon.commitTransaction();
      }).not.toThrow();
    } finally {
      process.off("unhandledRejection", applicationHandler);
      paintCannon.stop();
    }
  });

  it("keeps rendering when an uncaught exception is only being monitored", () => {
    const paintCannon = new PaintCannon();
    const native = mockNativeInstances.at(-1)!;

    try {
      process.emit("uncaughtExceptionMonitor", new Error("monitored"), "uncaughtException");

      expect(native.stopCalls).toBe(0);
      expect(() => {
        paintCannon.beginTransaction();
        paintCannon.commitTransaction();
      }).not.toThrow();
    } finally {
      paintCannon.stop();
    }
  });
});
