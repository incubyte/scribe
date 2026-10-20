import { invoke } from "@tauri-apps/api/core";
import { expecting } from "../shared/guards";
import { isArrayBuffer, isRegionCompletion, isRegionContext } from "./guards";
import type { PixelRect } from "./types";

export const fetchRegionContext = () =>
  invoke<unknown>("region_context").then(expecting(isRegionContext, "region context"));

export const fetchRegionFrame = () =>
  invoke<unknown>("region_frame").then(expecting(isArrayBuffer, "region frame"));

export const completeRegion = (area: PixelRect) =>
  invoke<unknown>("complete_region", { area }).then(expecting(isRegionCompletion, "region completion"));

export const cancelRegion = () => invoke("cancel_region");
