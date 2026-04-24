import { describe, expect, it } from "vitest";
import {
  defaultApiConfig,
  defaultSettings,
  shouldUpgradeMemoryPrompt,
} from "../appState";

describe("appState memory prompt helpers", () => {
  it("keeps default provider and hotkeys", () => {
    const settings = defaultSettings();
    expect(settings.translationProvider).toBe("youdao");
    expect(settings.spotlightShortcut).toBe("Alt+Shift+F");
    expect(settings.selectionShortcut).toBe("Alt+Shift+L");
  });

  it("upgrades legacy fixed-format prompts", () => {
    expect(
      shouldUpgradeMemoryPrompt(
        "You are a vocabulary memory coach. Return concise mnemonic content.",
      ),
    ).toBe(true);
    expect(
      shouldUpgradeMemoryPrompt(
        "你是专业单词记忆导师，请按以下规则输出，并使用旧词拆分法。",
      ),
    ).toBe(true);
  });

  it("keeps custom prompts intact", () => {
    expect(
      shouldUpgradeMemoryPrompt(
        "请用最自然的方式生成 2 到 3 行联想，避免固定模板。",
      ),
    ).toBe(false);
  });

  it("ships a varied default prompt", () => {
    expect(defaultSettings().memoryPrompt).toContain("不要每次都套同一个模板");
  });

  it("keeps free-model API defaults", () => {
    const api = defaultApiConfig();
    expect(api.baseUrl).toBe("https://text.pollinations.ai/openai");
    expect(api.model).toBe("openai");
  });
});
