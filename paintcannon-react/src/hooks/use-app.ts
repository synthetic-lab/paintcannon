import React from "react";
import type { PaintCannon } from "paintcannon";

export type PaintCannonReactApp = {
  readonly paintCannon: PaintCannon;
  exit(errorOrResult?: unknown): void;
  waitUntilExit(): Promise<unknown>;
};

export const AppContext = React.createContext<PaintCannonReactApp | undefined>(undefined);

export function useApp(): PaintCannonReactApp {
  const app = React.useContext(AppContext);
  if (app === undefined) {
    throw new Error("useApp() must be used inside a paintcannon-react render tree");
  }
  return app;
}
