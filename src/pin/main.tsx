import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../styles/app.css";
import { Pin } from "./Pin";

// 贴图窗口是透明的（关闭时淡出），页面背景也要透明
document.documentElement.classList.add("transparent-page");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Pin />
  </StrictMode>,
);
