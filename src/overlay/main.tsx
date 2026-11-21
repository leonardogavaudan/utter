import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../styles/base.css";
import "./overlay.css";
import { Overlay } from "./Overlay";

const root = document.getElementById("root");
if (!root) throw new Error("The overlay page is missing its #root element.");

createRoot(root).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
);
