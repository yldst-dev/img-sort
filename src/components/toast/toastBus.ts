export type ToastTone = "info" | "success" | "error" | "warning";

export interface ToastMessage {
  id: string;
  text: string;
  tone?: ToastTone;
  duration?: number;
}

type Subscriber = (msg: ToastMessage) => void;

class ToastBus {
  private listeners = new Set<Subscriber>();

  subscribe(fn: Subscriber) {
    this.listeners.add(fn);
    return () => {
      this.listeners.delete(fn);
    };
  }

  push(text: string, tone: ToastTone = "info", duration = 2200) {
    const message: ToastMessage = {
      id: crypto.randomUUID(),
      text,
      tone,
      duration,
    };
    this.listeners.forEach((l) => l(message));
  }
}

export const toastBus = new ToastBus();

export const toast = {
  info: (text: string) => toastBus.push(text, "info"),
  success: (text: string) => toastBus.push(text, "success"),
  error: (text: string) => toastBus.push(text, "error", 2800),
  warning: (text: string) => toastBus.push(text, "warning"),
};
