// === Knowledge OS — Frontend Logger ===
// Listens to backend log events and forwards them to browser console

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

const LEVEL_COLORS: Record<string, string> = {
  ERROR: "#e53935",
  WARN: "#fb8c00",
  INFO: "#43a047",
  DEBUG: "#1e88e5",
  TRACE: "#757575",
};

export async function setupLogBridge(): Promise<UnlistenFn | undefined> {
  if (typeof window === "undefined") return;

  const unlisten = await listen<LogEntry>("log-entry", (event) => {
    const { level, target, message, timestamp } = event.payload;
    const color = LEVEL_COLORS[level] || "#757575";
    const prefix = `%c[${timestamp.split("T")[1]?.split(".")[0] || timestamp}] ${level} ${target}%c`;
    const style = `color: ${color}; font-weight: bold;`;
    const reset = "color: inherit;";

    switch (level) {
      case "ERROR":
        console.error(prefix + " " + message, style, reset);
        break;
      case "WARN":
        console.warn(prefix + " " + message, style, reset);
        break;
      case "INFO":
        console.info(prefix + " " + message, style, reset);
        break;
      case "DEBUG":
        console.debug(prefix + " " + message, style, reset);
        break;
      case "TRACE":
        console.debug(prefix + " " + message, style, reset);
        break;
      default:
        console.log(prefix + " " + message, style, reset);
    }
  });

  console.log("%c[LOGGER] Log bridge initialized - receiving backend logs", "color: #43a047; font-weight: bold;");

  return unlisten;
}
