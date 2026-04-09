import { describe, expect, it } from "vitest";
import { defaultApiConfig, defaultSettings } from "../appState";

describe("appState defaults", () => {
  it("uses youdao provider and default hotkeys", () => {
    const settings = defaultSettings();
    expect(settings.translationProvider).toBe("youdao");
    expect(settings.spotlightShortcut).toBe("Alt+Shift+F");
    expect(settings.selectionShortcut).toBe("Alt+Shift+L");
    expect(settings.memoryPrompt).toContain("\u8054\u60f3\u8bb0\u5fc6\u6cd5");
  });

  it("uses free-model preset defaults for AI provider", () => {
    const api = defaultApiConfig();
    expect(api.baseUrl).toBe("https://openrouter.ai/api/v1/chat/completions");
    expect(api.model).toContain(":free");
  });
});