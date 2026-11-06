import type {
  NotificationEvent as NativeNotificationEvent,
  PaintCannon as NativePaintCannon,
} from "#paintcannon-native";

export type NotificationPermission = "default" | "granted" | "denied";
export type NotificationOptions = {
  body?: string;
  tag?: string;
  data?: unknown;
  silent?: boolean | null;
  requireInteraction?: boolean;
};
export const NOTIFICATION_EVENT_TYPES = ["show", "click", "close", "error"] as const;
export type NotificationEventType = (typeof NOTIFICATION_EVENT_TYPES)[number];
export type NotificationEventListener = (event: NotificationEvent) => void;

export class NotificationEvent extends Event {
  declare readonly target: Notification;
  declare readonly currentTarget: Notification;
  readonly error: Error | undefined;

  constructor(type: NotificationEventType, error?: Error) {
    super(type, { cancelable: type === "click" });
    this.error = error;
  }
}

export type NotificationConstructor = {
  new (title: string, options?: NotificationOptions): Notification;
  readonly permission: NotificationPermission;
  readonly maxActions: 0;
  requestPermission(): Promise<NotificationPermission>;
  supportsClose(): Promise<boolean>;
};

const deliver = Symbol("deliver");
const dispose = Symbol("dispose");

export class Notification extends EventTarget {
  readonly title: string;
  readonly body: string;
  readonly tag: string;
  readonly data: unknown;
  readonly silent: boolean | null;
  readonly requireInteraction: boolean;
  onshow: NotificationEventListener | null = null;
  onclick: NotificationEventListener | null = null;
  onclose: NotificationEventListener | null = null;
  onerror: NotificationEventListener | null = null;
  private state: "pending" | "sent" | "shown" | "closed" = "pending";
  private readonly id: number;

  constructor(
    title: string,
    options: NotificationOptions,
    private readonly controller: NotificationController,
  ) {
    super();
    this.title = String(title);
    this.body = String(options.body ?? "");
    this.tag = String(options.tag ?? "");
    this.data = structuredClone(options.data ?? null);
    this.silent = options.silent ?? null;
    this.requireInteraction = options.requireInteraction ?? false;
    for (const type of NOTIFICATION_EVENT_TYPES) {
      super.addEventListener(type, event => {
        if (!(event instanceof NotificationEvent)) return;
        this[`on${type}`]?.call(this, event);
      });
    }
    this.id = controller.register(this);
    queueMicrotask(() => {
      if (this.state !== "pending") return;
      this.state = "sent";
      try {
        controller.binding.showNotification(this.id, {
          title: this.title,
          body: this.body,
          tag: this.tag,
          silent: this.silent ?? undefined,
          requireInteraction: this.requireInteraction,
        });
      } catch (error) {
        this[deliver]("error", error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  override addEventListener(
    type: NotificationEventType,
    listener: NotificationEventListener | EventListenerObject | null,
    options?: AddEventListenerOptions | boolean,
  ): void {
    if (listener !== null)
      super.addEventListener(type, listener as EventListener | EventListenerObject, options);
  }

  override removeEventListener(
    type: NotificationEventType,
    listener: NotificationEventListener | EventListenerObject | null,
    options?: EventListenerOptions | boolean,
  ): void {
    if (listener !== null)
      super.removeEventListener(type, listener as EventListener | EventListenerObject, options);
  }

  close(): void {
    if (this.state === "closed") return;
    if (this.state === "pending") {
      this[dispose]();
      return;
    }
    this.controller.binding.closeNotification(this.id);
  }

  [deliver](type: NotificationEventType, error?: Error): void {
    if (this.state === "closed") return;
    if (type === "show") this.state = "shown";
    if (type === "close" || type === "error") this[dispose]();
    this.dispatchEvent(new NotificationEvent(type, error));
  }

  [dispose](): void {
    this.state = "closed";
    this.controller.remove(this.id);
  }
}

export class NotificationController {
  readonly Notification: NotificationConstructor;
  private permission: NotificationPermission = "default";
  private nextId = 1;
  private stopped = false;
  private readonly notifications = new Map<number, Notification>();

  constructor(readonly binding: NativePaintCannon) {
    const controller = this;
    this.Notification = class extends Notification {
      constructor(title: string, options: NotificationOptions = {}) {
        super(title, options, controller);
      }
      static get permission(): NotificationPermission {
        return controller.permission;
      }
      static get maxActions(): 0 {
        return 0;
      }
      static async requestPermission(): Promise<NotificationPermission> {
        // No browser prompt: automatically grant if native delivery is available.
        if (controller.stopped) return "denied";
        const supported = await binding.notificationPermission();
        controller.permission = !controller.stopped && supported ? "granted" : "denied";
        return controller.permission;
      }
      static async supportsClose(): Promise<boolean> {
        if (controller.stopped) return false;
        const supported = await binding.notificationSupportsClose();
        return !controller.stopped && supported;
      }
    };
  }

  register(notification: Notification): number {
    if (this.stopped) throw new Error("paintcannon renderer has been stopped");
    const id = this.nextId++;
    this.notifications.set(id, notification);
    return id;
  }

  remove(id: number): void {
    this.notifications.delete(id);
  }

  dispatch(event: NativeNotificationEvent): void {
    const type = NOTIFICATION_EVENT_TYPES.find(type => type === event.type);
    if (type === undefined) throw new Error(`unknown notification event: ${event.type}`);
    this.notifications
      .get(event.id)
      ?.[deliver](type, event.error === undefined ? undefined : new Error(event.error));
  }

  stop(): void {
    this.stopped = true;
    this.permission = "denied";
    for (const notification of this.notifications.values()) notification[dispose]();
  }
}
