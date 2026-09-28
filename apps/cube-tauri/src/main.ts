import { mountCubeApp } from "./app";
import { createTauriApi } from "./tauri-api";
import "./styles.css";

const root = document.querySelector<HTMLElement>("#app");
if (!root) {
  throw new Error("missing #app");
}
mountCubeApp(root, createTauriApi());
