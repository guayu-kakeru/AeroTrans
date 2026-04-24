import type { ApiConfig, Settings } from "../types";

const LEGACY_MEMORY_PROMPT_MARKERS = [
  "You are a vocabulary memory coach",
  "旧词拆分法",
  "按以下规则输出",
  "中文释义 + 记忆逻辑 + 场景短句",
];

const DEFAULT_MEMORY_PROMPT =
  "你是单词记忆教练，目标是让我一眼就能记住单词。请每次只输出 2 到 4 行简短联想内容，并且根据单词本身灵活变化结构：可以是画面、谐音、词形、对比、场景，或者极短的中英对照，但不要每次都套同一个模板。不要空谈方法，不要强行拆词，也不要机械重复“释义/联想/例句”结构，直接给我最好记的内容。";

export function shouldUpgradeMemoryPrompt(prompt: string | undefined): boolean {
  if (!prompt?.trim()) {
    return true;
  }

  return (
    prompt.includes("????") ||
    LEGACY_MEMORY_PROMPT_MARKERS.some((marker) => prompt.includes(marker))
  );
}

export const defaultSettings = (): Settings => ({
  spotlightShortcut: "Alt+Shift+F",
  selectionShortcut: "Alt+Shift+L",
  companionEnabled: false,
  companionOpacity: 88,
  companionMouseThrough: false,
  ttsEnabled: true,
  collectionMode: "manual_star",
  translationProvider: "youdao",
  memoryPrompt: DEFAULT_MEMORY_PROMPT,
});

export const defaultApiConfig = (): ApiConfig => ({
  baseUrl: "https://text.pollinations.ai/openai",
  model: "openai",
});
