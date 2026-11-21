import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../styles/base.css";
import "./settings.css";
import { SettingsApp } from "./SettingsApp";

const root = document.getElementById("root");
if (!root) throw new Error("The settings page is missing its #root element.");

createRoot(root).render(
  <StrictMode>
    <SettingsApp />
  </StrictMode>,
);
