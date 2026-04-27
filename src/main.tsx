import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/globals.css";
import { useSettingsStore } from "./stores/settingsStore";

// 应用主题到 DOM
function applyTheme(theme: string) {
  const isDark = theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", isDark);
  localStorage.setItem("theme", theme);
}

// 初始化
const initialTheme = localStorage.getItem("theme") || "dark";
applyTheme(initialTheme);

// 订阅 store 变化
useSettingsStore.subscribe((state) => {
  applyTheme(state.theme);
});

// 监听系统主题变化（跟随系统模式）
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  const current = useSettingsStore.getState().theme;
  if (current === "system") applyTheme("system");
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
