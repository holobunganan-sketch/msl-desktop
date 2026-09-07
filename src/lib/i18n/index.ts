import { invoke } from "@tauri-apps/api/core";
import { writable, get } from "svelte/store";
import { enUS } from "./en-US";
import { zhCN } from "./zh-CN";

export type Locale = "zh-CN" | "en-US";
export type TranslationKey = keyof typeof zhCN;
export const locale = writable<Locale>("zh-CN");

let activeLocale: Locale = "zh-CN";
let initialized = false;
locale.subscribe((value) => {
  activeLocale = value;
  if (typeof document !== "undefined") {
    document.documentElement.lang = value;
    document.title = value === "zh-CN" ? "MSL 工作台" : "MSL Workbench";
  }
});

export function isLocale(value: unknown): value is Locale {
  return value === "zh-CN" || value === "en-US";
}

export function t(key: TranslationKey, params: Record<string, string | number> = {}, forcedLocale?: Locale): string {
  const selected = forcedLocale ?? activeLocale;
  const dictionary = selected === "en-US" ? enUS : zhCN;
  let value = dictionary[key] ?? zhCN[key] ?? key;
  for (const [name, replacement] of Object.entries(params)) {
    value = value.replaceAll(`{${name}}`, String(replacement));
  }
  return value;
}

export async function initializeLocale(): Promise<Locale> {
  if (initialized) return get(locale);
  initialized = true;
  try {
    const saved = await invoke<string | null>("app_settings_get", { key: "locale" });
    const next = isLocale(saved) ? saved : "zh-CN";
    locale.set(next);
  } catch {
    locale.set("zh-CN");
  }
  return get(locale);
}

export async function setLocale(next: Locale): Promise<void> {
  locale.set(next);
  try {
    await invoke("app_settings_set", { key: "locale", value: next });
  } catch {
    // 浏览器预览或旧核心不支持设置时，保留当前会话语言。
  }
}

export function formatDateTime(timestamp: number | null | undefined, forcedLocale?: Locale): string {
  if (!timestamp) return "";
  return new Intl.DateTimeFormat(forcedLocale ?? activeLocale, {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(new Date(timestamp * 1000));
}

export function formatDate(timestamp: number | null | undefined, forcedLocale?: Locale): string {
  if (!timestamp) return "";
  return new Intl.DateTimeFormat(forcedLocale ?? activeLocale, { dateStyle: "medium" }).format(
    new Date(timestamp * 1000)
  );
}

export function translateStatus(status: string | null | undefined, forcedLocale?: Locale): string {
  if (!status) return "—";
  const key = `status.${status}` as TranslationKey;
  return key in zhCN ? t(key, {}, forcedLocale) : status;
}

export function translateKind(kind: string | null | undefined, forcedLocale?: Locale): string {
  if (!kind) return "—";
  const key = `kind.${kind}` as TranslationKey;
  return key in zhCN ? t(key, {}, forcedLocale) : kind;
}

void initializeLocale();
