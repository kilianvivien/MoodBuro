import { listen } from "@tauri-apps/api/event";

/** Dev-only query flags for the browser preview (see docs/design-guidelines.md). */
export const previewFlags = (search = globalThis.location?.search ?? "") => {
  const params = new URLSearchParams(search);
  return { mac: params.has("mac"), demo: params.has("demo") };
};

/**
 * Native window behaviour the shell mirrors on <html>:
 * - `.desktop`: macOS app chrome (unified toolbar under the traffic lights).
 *   `?mac` forces it in the dev browser preview.
 * - `.fullscreen`: traffic lights are hidden, so the toolbar drops their inset.
 * - `[data-inactive]`: the window is not key; selection and toolbar text dim.
 * Release builds also drop the web context menu outside text fields.
 */
export function installWindowChrome(desktop: boolean) {
  const root = document.documentElement;
  const mac = /Mac/.test(navigator.platform || navigator.userAgent);
  if ((desktop && mac) || (import.meta.env.DEV && previewFlags().mac))
    root.classList.add("desktop");

  if (import.meta.env.PROD)
    document.addEventListener("contextmenu", (event) => {
      const target = event.target as Element | null;
      if (!target?.closest("input, textarea, [contenteditable='true']"))
        event.preventDefault();
    });

  if (!desktop) return;
  void listen<boolean>("window://focused", (event) =>
    root.toggleAttribute("data-inactive", !event.payload),
  );
  void listen<boolean>("window://fullscreen", (event) =>
    root.classList.toggle("fullscreen", event.payload),
  );
}
