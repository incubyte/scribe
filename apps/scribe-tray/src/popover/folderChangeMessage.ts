import type { FolderChange } from "./types";

export const RECORDING_REASON = "Stop recording to change the folder.";
export const CHANGE_FAILURE_MESSAGE = "The folder could not be changed.";
export const FOLDER_UNAVAILABLE_MESSAGE =
  "Scribe could not read the transcripts folder setting.";

const savedIn = (folder: string) => `Transcripts will be saved in ${folder}.`;

const movedCount = (moved: number) =>
  moved === 1 ? "Moved 1 transcript." : `Moved ${moved} transcripts.`;

export function describeFolderChange(change: FolderChange): string | null {
  if (change.outcome === "recording") {
    return RECORDING_REASON;
  }
  if (change.outcome !== "changed") {
    return null;
  }
  return change.moved > 0
    ? `${savedIn(change.folder)} ${movedCount(change.moved)}`
    : savedIn(change.folder);
}
