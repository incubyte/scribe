import { useLayoutEffect, useRef } from "react";
import type { LiveLine, Partials } from "./feedReducer";
import type { Speaker } from "./types";

interface LiveTranscriptProps {
  readonly lines: readonly LiveLine[];
  readonly partials: Partials;
}

const BOTTOM_TOLERANCE_PX = 8;
const SPEAKERS: readonly Speaker[] = ["Me", "Them"];
const LINE_CLASS: Readonly<Record<Speaker, string>> = { Me: "line line--me", Them: "line line--them" };

function isAtBottom(region: HTMLDivElement): boolean {
  return region.scrollHeight - region.scrollTop - region.clientHeight <= BOTTOM_TOLERANCE_PX;
}

function usePinnedToBottom(lines: readonly LiveLine[], partials: Partials) {
  const region = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);

  useLayoutEffect(() => {
    if (region.current && pinned.current) {
      region.current.scrollTop = region.current.scrollHeight;
    }
  }, [lines, partials]);

  const followReader = () => {
    if (region.current) {
      pinned.current = isAtBottom(region.current);
    }
  };

  return { region, followReader };
}

export function LiveTranscript({ lines, partials }: LiveTranscriptProps) {
  const { region, followReader } = usePinnedToBottom(lines, partials);
  const latest = lines.at(-1);

  return (
    <div
      ref={region}
      className="transcript"
      role="region"
      aria-label="Live transcript"
      tabIndex={0}
      onScroll={followReader}
    >
      <ol className="transcript__lines">
        {lines.map((line) => (
          <li key={`${line.speaker}-${line.startMs}`} className={LINE_CLASS[line.speaker]}>
            <span className="line__speaker">{line.speaker}</span>
            <span className="line__text">{line.text}</span>
          </li>
        ))}
        {SPEAKERS.filter((speaker) => partials[speaker] !== undefined).map((speaker) => (
          <li key={`${speaker}-partial`} className={`${LINE_CLASS[speaker]} partial`} aria-hidden="true">
            <span className="line__speaker">{speaker}</span>
            <span className="line__text">{partials[speaker]}</span>
          </li>
        ))}
      </ol>
      <div className="visually-hidden" aria-live="polite">
        {latest ? `${latest.speaker}: ${latest.text}` : null}
      </div>
    </div>
  );
}
