import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { IdleNotice } from "./idle-notice/IdleNotice";
import { Popover } from "./popover/Popover";
import { PrivacyStatement } from "./privacy/PrivacyStatement";
import { RegionSelector } from "./region-selector/RegionSelector";
import "./theme/tokens.css";
import "./theme/primitives.css";
import "./theme/popover.css";

const ROUTES: Readonly<Record<string, () => React.JSX.Element>> = {
  "#idle-notice": IdleNotice,
  "#popover": Popover,
  "#privacy": PrivacyStatement,
  "#region-selector": RegionSelector,
};

const Route = ROUTES[window.location.hash] ?? PrivacyStatement;

const root = document.getElementById("root");
if (root === null) {
  throw new Error("Scribe's page has no root element");
}

createRoot(root).render(
  <StrictMode>
    <Route />
  </StrictMode>,
);
