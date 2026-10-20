import { invoke } from "@tauri-apps/api/core";

export const dismissIdleNotice = (dontShowAgain: boolean) =>
  invoke("dismiss_idle_notice", { dontShowAgain });
