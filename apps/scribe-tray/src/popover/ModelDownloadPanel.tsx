import type { ModelDownload } from "./types";

const ANNOUNCEMENT_STEP_PERCENT = 10;

interface ModelDownloadPanelProps {
  readonly download: ModelDownload;
  readonly onRetry: () => void;
}

export function ModelDownloadPanel({ download, onRetry }: ModelDownloadPanelProps) {
  if (download.state === "ready") {
    return null;
  }

  return (
    <section className="model-download" aria-label="Speech model">
      {download.state === "downloading" ? (
        <Progress percent={download.percent} />
      ) : (
        <Failure onRetry={onRetry} />
      )}
    </section>
  );
}

function Progress({ percent }: Readonly<{ percent: number }>) {
  const announcedPercent = percent - (percent % ANNOUNCEMENT_STEP_PERCENT);

  return (
    <>
      <p className="model-download__title">Downloading speech model</p>
      <div className="model-download__bar">
        <progress aria-label="Speech model download" max={100} value={percent} />
        <span className="model-download__percent" aria-hidden="true">
          {percent}%
        </span>
      </div>
      <p className="visually-hidden" role="status" aria-live="polite">
        Downloading speech model, {announcedPercent}%
      </p>
    </>
  );
}

function Failure({ onRetry }: Readonly<{ onRetry: () => void }>) {
  return (
    <div className="model-download__failure">
      <p className="model-download__title" role="status" aria-live="polite">
        Download failed
      </p>
      <button className="button" type="button" onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}
