import { useState } from "react";
import type { RecorderPhase } from "./types";

function announcementFor(previous: RecorderPhase, next: RecorderPhase): string | null {
  if (next === "recording") {
    return "Recording started";
  }
  return previous === "recording" ? "Recording stopped" : null;
}

export function usePhaseAnnouncement(phase: RecorderPhase): string {
  const [announcedPhase, setAnnouncedPhase] = useState(phase);
  const [announcement, setAnnouncement] = useState("");

  if (phase !== announcedPhase) {
    setAnnouncedPhase(phase);
    setAnnouncement(announcementFor(announcedPhase, phase) ?? announcement);
  }

  return announcement;
}
