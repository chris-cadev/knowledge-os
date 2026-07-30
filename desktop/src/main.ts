import { mount } from "svelte";
import App from "./App.svelte";
import { setupLogBridge } from "./lib/logger.js";

setupLogBridge().catch((err) => {
  console.error("Failed to setup log bridge:", err);
});

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
