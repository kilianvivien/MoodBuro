import { invoke, isTauri } from "@tauri-apps/api/core";
export interface Site {
  id: string;
  wwwroot: string;
  name: string;
  userId: number;
  fullName: string;
}
export const desktop = isTauri();
export const ipc = {
  sites: () => (desktop ? invoke<Site[]>("sites_list") : Promise.resolve([])),
  addSite: (url: string) =>
    desktop
      ? invoke<void>("site_add", { url })
      : Promise.reject(
          new Error("Open the desktop app with pnpm tauri dev to sign in."),
        ),
  switchSite: (id: string) => invoke<void>("site_switch", { id }),
  onboarding: () =>
    desktop ? invoke<void>("show_onboarding") : Promise.resolve(),
  nav: (action: "back" | "forward" | "reload" | "open_in_browser") =>
    invoke<void>("nav", { action }),
};
