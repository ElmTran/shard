import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../styles/settings.css";
import { SettingsPage } from "./Settings";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <SettingsPage />
  </StrictMode>,
);
