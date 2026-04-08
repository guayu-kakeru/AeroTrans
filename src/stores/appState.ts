import type { ApiConfig, Settings } from "../types";

export const defaultSettings = (): Settings => ({
  spotlightShortcut: "Ctrl+Shift+Space",
  selectionShortcut: "Ctrl+Shift+D",
  companionEnabled: false,
  companionOpacity: 88,
  companionMouseThrough: false,
  ttsEnabled: true,
  collectionMode: "manual_star"
});

export const defaultApiConfig = (): ApiConfig => ({
  baseUrl: "https://api.openai.com/v1/chat/completions",
  model: "gpt-4o-mini"
});
