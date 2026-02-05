import { createSignal } from "solid-js";
import { tauriService } from "../tauri/services";
import { emit, listen } from "@tauri-apps/api/event";

export type ThemeMode = "dark" | "light" | "system";
export type ThemeColor = 
  | "neutral" | "blue" | "emerald" | "orange" | "rose" 
  | "violet" | "teal" | "zinc" | "indigo" | "fuchsia" 
  | "slate" | "stone";
export type ThemeFontSize = "small" | "medium" | "large";

export interface AppearanceState {
  mode: ThemeMode;
  theme: ThemeColor;
  radius: number; // in px
  fontSize: ThemeFontSize;
}

const DEFAULT_STATE: AppearanceState = {
  mode: "system",
  theme: "neutral",
  radius: 6,
  fontSize: "medium",
};

const [appearance, setAppearance] = createSignal<AppearanceState>(DEFAULT_STATE);

// Event name for cross-window sync
const SYNC_EVENT = "appearance:sync";

export const appearanceActions = {
  initialize: async () => {
    // Load individual settings to match "GeneralPanel" pattern
    const [mode, theme, radius, fontSize] = await Promise.all([
      tauriService.getSetting("appearance_mode"),
      tauriService.getSetting("appearance_theme"),
      tauriService.getSetting("appearance_radius"),
      tauriService.getSetting("appearance_font_size"),
    ]);

    setAppearance({
      mode: (mode as ThemeMode) ?? DEFAULT_STATE.mode,
      theme: (theme as ThemeColor) ?? DEFAULT_STATE.theme,
      radius: (radius as number) ?? DEFAULT_STATE.radius,
      fontSize: (fontSize as ThemeFontSize) ?? DEFAULT_STATE.fontSize,
    });

    appearanceActions.apply();

    // Listen for changes from other windows
    listen(SYNC_EVENT, (event: any) => {
      setAppearance(event.payload);
      appearanceActions.apply();
    });
  },

  update: async (updates: Partial<AppearanceState>) => {
    const next = { ...appearance(), ...updates };
    setAppearance(next);
    appearanceActions.apply();

    // Persist individually
    const promises: Promise<void>[] = [];
    if (updates.mode) promises.push(tauriService.setSetting("appearance_mode", updates.mode));
    if (updates.theme) promises.push(tauriService.setSetting("appearance_theme", updates.theme));
    if (updates.radius !== undefined) promises.push(tauriService.setSetting("appearance_radius", updates.radius));
    if (updates.fontSize) promises.push(tauriService.setSetting("appearance_font_size", updates.fontSize));

    await Promise.all(promises);
    
    // Notify other windows
    await emit(SYNC_EVENT, next);
  },

  apply: () => {
    const state = appearance();
    const root = document.documentElement;

    // Apply Mode
    let mode = state.mode;
    if (mode === "system") {
      mode = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    root.setAttribute("data-theme", mode);

    // Apply Theme Color
    const themes: ThemeColor[] = ["neutral", "blue", "emerald", "orange", "rose", "violet", "teal", "zinc", "indigo", "fuchsia", "slate", "stone"];
    themes.forEach(t => root.classList.remove(`theme-${t}`));
    root.classList.add(`theme-${state.theme}`);

    // Apply Radius
    root.style.setProperty("--radius-m", `${state.radius}px`);
    root.style.setProperty("--radius-s", `${Math.max(0, state.radius - 2)}px`);
    root.style.setProperty("--radius-xs", `${Math.max(0, state.radius - 4)}px`);
    root.style.setProperty("--radius-l", `${state.radius + 2}px`);

    // Apply Font Size
    root.classList.remove("font-size-small", "font-size-medium", "font-size-large");
    root.classList.add(`font-size-${state.fontSize}`);
  }
};

// Initial listener for system theme changes
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if (appearance().mode === "system") {
    appearanceActions.apply();
  }
});

export { appearance };
