import type { ModelDownload } from "./types";

const ANNOUNCEMENT_STEP_PERCENT = 10;
const PREPARING = "Preparing speech model";
const DOWNLOADING = "Downloading speech model";
const FAILED = "Download failed";

type Unfinished = Exclude<ModelDownload, { state: "ready" }>;

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
      <p className="visually-hidden" role="status" aria-live="polite">
        {announcementFor(download)}
      </p>
    </section>
  );
}

function announcementFor(download: Unfinished): string {
  if (download.state === "failed") {
    return FAILED;
  }
  if (download.percent === null) {
    return PREPARING;
  }
  return `${DOWNLOADING}, ${download.percent - (download.percent % ANNOUNCEMENT_STEP_PERCENT)}%`;
}

function Progress({ percent }: Readonly<{ percent: number | null }>) {
  return (
    <>
      <p className="model-download__title">{percent === null ? PREPARING : DOWNLOADING}</p>
      <div className="model-download__bar">
        <progress aria-label="Speech model download" max={100} value={percent ?? undefined} />
        {percent === null ? null : (
          <span className="model-download__percent" aria-hidden="true">
            {percent}%
          </span>
        )}
      </div>
    </>
  );
}

function Failure({ onRetry }: Readonly<{ onRetry: () => void }>) {
  return (
    <div className="model-download__failure">
      <p className="model-download__title">{FAILED}</p>
      <button className="button" type="button" onClick={onRetry}>
        Retry
      </button>
    </div>
  );
}
