import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { PrivacyStatement } from "./privacy/PrivacyStatement";
import "./theme/tokens.css";
import "./theme/primitives.css";

const ROUTES: Readonly<Record<string, () => React.JSX.Element>> = {
  "#privacy": PrivacyStatement,
};

const Route = ROUTES[window.location.hash] ?? PrivacyStatement;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Route />
  </StrictMode>,
);
