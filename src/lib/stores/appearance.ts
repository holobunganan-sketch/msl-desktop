import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";

export type FontSize = "compact" | "standard" | "large" | "xlarge";
export type AppTheme = "mist" | "sage" | "ivory" | "contrast";

const FONT_SIZES: FontSize[] = ["compact", "standard", "large", "xlarge"];
const THEMES: AppTheme[] = ["mist", "sage", "ivory", "contrast"];

export const fontSize = writable<FontSize>("standard");
export const appTheme = writable<AppTheme>("mist");

function isFontSize(value: unknown): value is FontSize { return FONT_SIZES.includes(value as FontSize); }
function isTheme(value: unknown): value is AppTheme { return THEMES.includes(value as AppTheme); }

function applyFontSize(value: FontSize) {
  if (typeof document !== "undefined") document.documentElement.dataset.fontSize = value;
  fontSize.set(value);
}

function applyTheme(value: AppTheme) {
  if (typeof document !== "undefined") document.documentElement.dataset.theme = value;
  appTheme.set(value);
}

export async function initializeAppearance(): Promise<void> {
  applyFontSize("standard");
  applyTheme("mist");
  try {
    const [savedFont, savedTheme] = await Promise.all([
      invoke<string | null>("app_settings_get", { key: "appearance_font_scale" }),
      invoke<string | null>("app_settings_get", { key: "appearance_theme" })
    ]);
    applyFontSize(isFontSize(savedFont) ? savedFont : "standard");
    applyTheme(isTheme(savedTheme) ? savedTheme : "mist");
  } catch {
    // Browser preview and older cores keep the safe defaults.
  }
}

export async function setFontSize(value: FontSize): Promise<void> {
  applyFontSize(value);
  try { await invoke("app_settings_set", { key: "appearance_font_scale", value }); } catch { /* preview */ }
}

export async function setTheme(value: AppTheme): Promise<void> {
  applyTheme(value);
  try { await invoke("app_settings_set", { key: "appearance_theme", value }); } catch { /* preview */ }
}

if (typeof document !== "undefined") {
  applyFontSize("standard");
  applyTheme("mist");
}

