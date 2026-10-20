export interface Size {
  readonly width: number;
  readonly height: number;
}

export interface FramePoint {
  readonly x: number;
  readonly y: number;
}

export interface PixelRect extends FramePoint, Size {}

export interface ClientPoint {
  readonly clientX: number;
  readonly clientY: number;
}

export interface RegionContext {
  readonly frame_width: number;
  readonly frame_height: number;
  readonly outlines: readonly PixelRect[];
}

export type RegionCompletion = { readonly outcome: "captured" } | { readonly outcome: "too_small" };
