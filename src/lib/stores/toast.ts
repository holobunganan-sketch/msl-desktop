import { writable } from "svelte/store";

export type ToastKind = "success" | "error" | "info";
export type Toast = { id: number; kind: ToastKind; message: string; dismissible: boolean };

export const toasts = writable<Toast[]>([]);
let nextId = 1;

export function addToast(message: string, kind: ToastKind = "info", duration = 4500): number {
  const id = nextId++;
  // A single in-flow notification cannot pile up over the user's work.
  // Persistent AI job history remains available in Background tasks.
  toasts.set([{ id, kind, message, dismissible: true }]);
  if (duration > 0) setTimeout(() => dismissToast(id), duration);
  return id;
}

export function dismissToast(id: number): void {
  toasts.update((items) => items.filter((item) => item.id !== id));
}
