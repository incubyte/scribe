import type { LiveLine } from "./feedReducer";

interface LiveCaptionProps {
  readonly caption: string | null;
  readonly latestLine: LiveLine | null;
}

const LISTENING = "Listening…";

export function LiveCaption({ caption, latestLine }: LiveCaptionProps) {
  return (
    <div className="caption">
      <p className={caption === null ? "caption__text caption__text--waiting" : "caption__text"} aria-hidden="true">
        {caption ?? LISTENING}
      </p>
      <p className="visually-hidden" aria-live="polite">
        {latestLine ? `${latestLine.speaker}: ${latestLine.text}` : null}
      </p>
    </div>
  );
}
