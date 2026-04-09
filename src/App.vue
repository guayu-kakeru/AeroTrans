<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { defaultApiConfig, defaultSettings } from "./stores/appState";
import { tauriService } from "./services/tauri";
import { defaultClipboardPolicy, shouldReadClipboard } from "./lib/clipboardPolicy";
import { createReviewQueue, nextIndex, prevIndex, removeAt, type ReviewQueueState } from "./lib/reviewQueue";
import type { TranslationProvider, TranslationResult, VocabularyItem } from "./types";

type TabName = "spotlight" | "companion" | "vocabulary" | "settings";

const tabs: { key: TabName; label: string }[] = [
  { key: "spotlight", label: "翻译" },
  { key: "companion", label: "Companion" },
  { key: "vocabulary", label: "单词本" },
  { key: "settings", label: "设置" }
];

const providerLabels: Record<TranslationProvider, string> = {
  google: "Google",
  youdao: "有道词典",
  mymemory: "MyMemory",
  ai: "AI"
};

const sourceLabels: Record<TranslationResult["source"], string> = {
  google: "Google",
  local: "词典回退",
  youdao: "有道词典",
  mymemory: "MyMemory",
  ai: "AI",
  public: "公共翻译回退",
  fallback: "兜底翻译"
};

const activeTab = ref<TabName>("spotlight");
const status = ref("初始化中...");
const runningInBrowser = ref(false);
const windowLabel = ref("browser");
const windowMaximized = ref(false);
const windowAlwaysOnTop = ref(false);
const readingCompactMode = ref(false);
const showClipboardHelp = ref(false);
const capturingShortcut = ref<"spotlight" | "selection" | null>(null);

const MAIN_NORMAL_SIZE = { width: 680, height: 540 };
const MAIN_COMPACT_SIZE = { width: 340, height: 220 };

function normalizeProvider(provider: string | undefined): TranslationProvider {
  if (provider === "youdao" || provider === "mymemory" || provider === "ai" || provider === "google") {
    return provider;
  }
  return "youdao";
}

const freeModelPresets = [
  {
    name: "DeepSeek 免费线路",
    baseUrl: "https://api.deepseek.com/v1/chat/completions",
    model: "deepseek-chat"
  },
  {
    name: "Kimi 兼容接口",
    baseUrl: "https://api.moonshot.cn/v1/chat/completions",
    model: "moonshot-v1-8k"
  },
  {
    name: "OpenRouter 免费模型",
    baseUrl: "https://openrouter.ai/api/v1/chat/completions",
    model: "deepseek/deepseek-chat-v3-0324:free"
  }
] as const;

const inputText = ref("");
const contextText = ref("");
const translating = ref(false);
const translatingCompanion = ref(false);
const result = ref<TranslationResult | null>(null);
const errorMessage = ref("");

const settings = ref(defaultSettings());
const apiConfig = ref(defaultApiConfig());
const apiKeyInput = ref("");
const hasApiKey = ref(false);
const settingsMessage = ref("");
const shortcutStatusMessage = ref("");
const speaking = ref(false);

const vocabulary = ref<VocabularyItem[]>([]);
const queueState = ref<ReviewQueueState<VocabularyItem>>(createReviewQueue([]));
const vocabularyViewMode = ref<"review" | "list">("review");
const memoryLoading = ref(false);
const generatedMemory = ref("");

const companionRawText = ref("");
const companionResult = ref<TranslationResult | null>(null);
const lastClipboardReadMs = ref(0);
const lastClipboardText = ref("");
let companionTimer: number | undefined;
let keydownHandler: ((event: KeyboardEvent) => void) | undefined;
let selectionHotkeyUnlisten: (() => void) | undefined;

const isSpotlightWindow = computed(() => windowLabel.value === "spotlight");

const currentWord = computed(() => {
  if (queueState.value.index < 0 || queueState.value.index >= queueState.value.items.length) return null;
  return queueState.value.items[queueState.value.index];
});

const appShellStyle = computed(() => ({
  opacity: String(Math.max(0.2, Math.min(1, settings.value.companionOpacity / 100)))
}));

function detectDirection(text: string): "zh_to_en" | "en_to_zh" {
  return /[\u4e00-\u9fff]/.test(text) ? "zh_to_en" : "en_to_zh";
}

function buildBrowserTranslation(text: string, provider: TranslationProvider): TranslationResult {
  const direction = detectDirection(text);
  const glossary = text.split(/\s+/).filter(Boolean).slice(0, 3);
  const phonetics = /^[a-zA-Z-]+$/.test(text.trim()) ? ["/demo/"] : [];

  if (provider === "google") {
    return {
      detectedDirection: direction,
      translation: direction === "zh_to_en" ? `Google(browser): ${text}` : `Google(浏览器演示): ${text}`,
      glossary,
      phonetics,
      source: "google"
    };
  }
  if (provider === "youdao") {
    return {
      detectedDirection: direction,
      translation: direction === "zh_to_en" ? `Youdao(browser): ${text}` : `有道(浏览器演示): ${text}`,
      glossary,
      phonetics,
      source: "youdao"
    };
  }
  if (provider === "mymemory") {
    return {
      detectedDirection: direction,
      translation: direction === "zh_to_en" ? `MyMemory(browser): ${text}` : `MyMemory(浏览器演示): ${text}`,
      glossary,
      phonetics,
      source: "mymemory"
    };
  }
  return {
    detectedDirection: direction,
    translation: direction === "zh_to_en" ? `AI(browser): ${text}` : `AI（浏览器演示）: ${text}`,
    glossary,
    phonetics,
    source: "ai"
  };
}

async function withCurrentWindow<T>(runner: (win: import("@tauri-apps/api/window").Window) => Promise<T>): Promise<T | null> {
  if (runningInBrowser.value) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return runner(getCurrentWindow());
}

async function refreshMaximizedState() {
  const value = await withCurrentWindow((win) => win.isMaximized());
  if (typeof value === "boolean") {
    windowMaximized.value = value;
  }
}

async function refreshAlwaysOnTopState() {
  const value = await withCurrentWindow((win) => win.isAlwaysOnTop());
  if (typeof value === "boolean") {
    windowAlwaysOnTop.value = value;
  }
}

async function minimizeWindow() {
  await withCurrentWindow((win) => win.minimize());
}

async function toggleMaximizeWindow() {
  await withCurrentWindow((win) => win.toggleMaximize());
  await refreshMaximizedState();
}

async function toggleAlwaysOnTop() {
  if (runningInBrowser.value) return;
  const next = !windowAlwaysOnTop.value;
  await withCurrentWindow((win) => win.setAlwaysOnTop(next));
  windowAlwaysOnTop.value = next;
}

async function closeWindow() {
  if (runningInBrowser.value) {
    window.close();
    return;
  }
  if (windowLabel.value === "spotlight") {
    await tauriService.hideSpotlightWindow();
    return;
  }
  await tauriService.quitApp();
}

async function hideCurrentWindow() {
  if (runningInBrowser.value) return;
  if (windowLabel.value === "spotlight") {
    await tauriService.hideSpotlightWindow();
    return;
  }
  await withCurrentWindow((win) => win.hide());
}

async function startWindowDrag() {
  try {
    await withCurrentWindow((win) => win.startDragging());
  } catch (err) {
    console.warn("窗口拖拽失败", err);
  }
}

function shouldIgnoreDragTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName.toLowerCase();
  if (["p", "span", "h1", "h2", "h3", "h4", "h5", "h6", "li", "label", "small"].includes(tag)) {
    return true;
  }
  return Boolean(
    target.closest(
      "button, input, textarea, select, option, a, [contenteditable='true'], [data-no-drag='true'], [data-text-select='true']"
    )
  );
}

function handleGlobalMouseDown(event: MouseEvent) {
  if (event.button !== 0 || runningInBrowser.value) return;
  if (shouldIgnoreDragTarget(event.target)) return;
  void startWindowDrag();
}

async function applyReadingCompactMode(enabled: boolean) {
  if (runningInBrowser.value || windowLabel.value !== "main") return;

  const { LogicalSize } = await import("@tauri-apps/api/dpi");
  await withCurrentWindow(async (win) => {
    const size = enabled ? MAIN_COMPACT_SIZE : MAIN_NORMAL_SIZE;
    await win.setSize(new LogicalSize(size.width, size.height));
    await win.setAlwaysOnTop(enabled);
    if (enabled) {
      await win.center();
    }
    return Promise.resolve();
  });
}

async function toggleReadingCompactMode() {
  readingCompactMode.value = !readingCompactMode.value;
  await applyReadingCompactMode(readingCompactMode.value);
}

async function detectWindowLabel() {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    runningInBrowser.value = true;
    windowLabel.value = "browser";
    return;
  }

  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  windowLabel.value = getCurrentWindow().label;
}

async function bootstrap() {
  await detectWindowLabel();

  try {
    const health = await tauriService.healthCheck();
    status.value = health;
  } catch (err) {
    runningInBrowser.value = true;
    status.value = "浏览器演示模式（未连接 Tauri）";
    console.warn(err);
  }

  if (runningInBrowser.value) {
    const local = localStorage.getItem("aerotrans_settings");
    if (local) {
      const parsed = { ...defaultSettings(), ...JSON.parse(local) };
      parsed.translationProvider = normalizeProvider(parsed.translationProvider);
      settings.value = parsed;
    }

    const localApi = localStorage.getItem("aerotrans_api_config");
    if (localApi) apiConfig.value = JSON.parse(localApi);

    const localVocab = localStorage.getItem("aerotrans_vocabulary");
    if (localVocab) {
      vocabulary.value = JSON.parse(localVocab);
      queueState.value = createReviewQueue(vocabulary.value);
    }

    hasApiKey.value = Boolean(localStorage.getItem("aerotrans_api_key"));
    return;
  }

  settings.value = await tauriService.loadSettings();
  settings.value.translationProvider = normalizeProvider(settings.value.translationProvider);
  const recommendedPrompt = defaultSettings().memoryPrompt;
  if (
    !settings.value.memoryPrompt?.trim() ||
    settings.value.memoryPrompt.includes("????") ||
    settings.value.memoryPrompt.startsWith("You are a vocabulary memory coach")
  ) {
    settings.value.memoryPrompt = recommendedPrompt;
    try {
      await tauriService.saveSettings(settings.value);
    } catch (err) {
      console.warn("memory prompt auto-upgrade failed", err);
    }
  }
  apiConfig.value = await tauriService.loadApiConfig();
  hasApiKey.value = await tauriService.hasApiKey();
  try {
    await tauriService.syncSpotlightShortcut(settings.value.spotlightShortcut);
    await tauriService.syncSelectionShortcut(settings.value.selectionShortcut);
  } catch (err) {
    settingsMessage.value = `全局快捷键注册失败：${String(err)}`;
  }
  await refreshVocabulary();
  await refreshMaximizedState();
}

async function refreshVocabulary() {
  if (runningInBrowser.value) {
    const localVocab = localStorage.getItem("aerotrans_vocabulary");
    vocabulary.value = localVocab ? JSON.parse(localVocab) : [];
    queueState.value = createReviewQueue(vocabulary.value);
    return;
  }

  vocabulary.value = await tauriService.listVocabulary();
  queueState.value = createReviewQueue(vocabulary.value);
}

async function translateNow() {
  if (!inputText.value.trim()) return;

  translating.value = true;
  errorMessage.value = "";
  try {
    result.value = runningInBrowser.value
      ? buildBrowserTranslation(inputText.value, settings.value.translationProvider)
      : await tauriService.translate({
          text: inputText.value,
          context: contextText.value || null,
          provider: settings.value.translationProvider
        });
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    translating.value = false;
  }
}

function detectSpeakLanguage(text: string): string {
  return /[\u4e00-\u9fff]/.test(text) ? "zh-CN" : "en-US";
}

function speakText(text: string) {
  const content = text.trim();
  if (!content) return;

  if (!settings.value.ttsEnabled) {
    settingsMessage.value = "请先在设置中开启系统 TTS 再使用发音。";
    return;
  }

  if (typeof window === "undefined" || !("speechSynthesis" in window)) {
    settingsMessage.value = "当前环境不支持系统语音发音。";
    return;
  }

  window.speechSynthesis.cancel();
  const utterance = new SpeechSynthesisUtterance(content);
  utterance.lang = detectSpeakLanguage(content);
  utterance.rate = 0.95;
  utterance.pitch = 1;
  utterance.onstart = () => {
    speaking.value = true;
  };
  utterance.onend = () => {
    speaking.value = false;
  };
  utterance.onerror = () => {
    speaking.value = false;
  };
  window.speechSynthesis.speak(utterance);
}

async function addCurrentToVocabulary(starred = true) {
  if (!result.value || !inputText.value.trim()) return;

  const payload = {
    term: inputText.value.trim(),
    translation: result.value.translation,
    contextText: contextText.value || null,
    starred,
    aiMemory: null
  };

  if (runningInBrowser.value) {
    const next: VocabularyItem = {
      id: Date.now(),
      createdAt: new Date().toISOString(),
      ...payload
    };
    vocabulary.value.unshift(next);
    localStorage.setItem("aerotrans_vocabulary", JSON.stringify(vocabulary.value));
    queueState.value = createReviewQueue(vocabulary.value);
    return;
  }

  await tauriService.addVocabulary(payload);
  await refreshVocabulary();
}

function nextCard() {
  if (queueState.value.items.length === 0 || queueState.value.index < 0) return;
  queueState.value = {
    ...queueState.value,
    index: nextIndex(queueState.value.index, queueState.value.items.length)
  };
}

function prevCard() {
  if (queueState.value.items.length === 0 || queueState.value.index < 0) return;
  queueState.value = {
    ...queueState.value,
    index: prevIndex(queueState.value.index, queueState.value.items.length)
  };
}

async function deleteCurrentCard() {
  if (!currentWord.value) return;

  const currentIndex = queueState.value.index;
  const deleting = currentWord.value;

  if (runningInBrowser.value) {
    queueState.value = removeAt(queueState.value, currentIndex);
    vocabulary.value = queueState.value.items;
    localStorage.setItem("aerotrans_vocabulary", JSON.stringify(vocabulary.value));
    return;
  }

  await tauriService.deleteVocabulary(deleting.id);
  queueState.value = removeAt(queueState.value, currentIndex);
  vocabulary.value = queueState.value.items;
}

async function deleteVocabularyById(id: number) {
  if (runningInBrowser.value) {
    vocabulary.value = vocabulary.value.filter((item) => item.id !== id);
    localStorage.setItem("aerotrans_vocabulary", JSON.stringify(vocabulary.value));
    queueState.value = createReviewQueue(vocabulary.value);
    return;
  }

  await tauriService.deleteVocabulary(id);
  await refreshVocabulary();
}

async function generateMemory(regenerate = false) {
  if (!currentWord.value) return;

  memoryLoading.value = true;
  generatedMemory.value = "";
  try {
    generatedMemory.value = runningInBrowser.value
      ? `${currentWord.value.term} /${currentWord.value.term}/ ${currentWord.value.translation}
旧词联想：${currentWord.value.term.slice(0, 2) || currentWord.value.term} + ${currentWord.value.term.slice(2) || "音"}
记忆场景：读文章看到 ${currentWord.value.term}，马上想到“${currentWord.value.translation}”。
场景短句：I see ${currentWord.value.term}, I think ${currentWord.value.translation}.`
      : regenerate
        ? await tauriService.regenerateMemory(currentWord.value.id)
        : await tauriService.generateMemory(currentWord.value.id);

    currentWord.value.aiMemory = generatedMemory.value;
  } finally {
    memoryLoading.value = false;
  }
}

async function saveAllSettings() {
  settingsMessage.value = "";
  shortcutStatusMessage.value = "";
  try {
    if (!runningInBrowser.value) {
      await tauriService.saveSettings(settings.value);
    } else {
      localStorage.setItem("aerotrans_settings", JSON.stringify(settings.value));
    }

    settingsMessage.value = "设置已保存";
    shortcutStatusMessage.value = "快捷键已保存并生效";
  } catch (err) {
    const msg = String(err);
    settingsMessage.value = msg;
    shortcutStatusMessage.value = `快捷键保存失败：${msg}`;
  }
}

async function saveApiSettings() {
  settingsMessage.value = "";
  if (!apiConfig.value.baseUrl.trim() || !apiConfig.value.model.trim()) {
    settingsMessage.value = "API URL 和模型名不能为空";
    return;
  }

  if (runningInBrowser.value) {
    localStorage.setItem("aerotrans_api_config", JSON.stringify(apiConfig.value));
    if (apiKeyInput.value.trim()) {
      localStorage.setItem("aerotrans_api_key", apiKeyInput.value.trim());
      hasApiKey.value = true;
      apiKeyInput.value = "";
    }
    settingsMessage.value = "API 配置已保存（浏览器模式）";
    return;
  }

  await tauriService.saveApiConfig(apiConfig.value);
  if (apiKeyInput.value.trim()) {
    await tauriService.saveApiKey(apiKeyInput.value.trim());
    hasApiKey.value = true;
    apiKeyInput.value = "";
  }
  settingsMessage.value = "API 配置已保存";
}

function applyModelPreset(index: number) {
  const preset = freeModelPresets[index];
  if (!preset) return;
  apiConfig.value.baseUrl = preset.baseUrl;
  apiConfig.value.model = preset.model;
  settingsMessage.value = `已应用预设：${preset.name}`;
}

function normalizeMainKey(event: KeyboardEvent): string | null {
  const key = event.key;
  if (!key) return null;

  if (key === "Control" || key === "Shift" || key === "Alt" || key === "Meta") {
    return null;
  }
  if (key === " ") return "Space";
  if (/^F\d{1,2}$/i.test(key)) return key.toUpperCase();
  if (key.length === 1) return key.toUpperCase();

  const mapping: Record<string, string> = {
    Escape: "Esc",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    Insert: "Insert"
  };
  return mapping[key] ?? null;
}

function startShortcutCapture(target: "spotlight" | "selection") {
  capturingShortcut.value = target;
  shortcutStatusMessage.value = `请按下${target === "spotlight" ? "Spotlight" : "划词翻译"}快捷键组合（建议至少含 Ctrl/Alt/Shift）`;
}

async function applyCapturedShortcut(value: string) {
  if (!capturingShortcut.value) return;
  if (!/(Ctrl|Alt|Shift|Meta)\+/.test(value)) {
    shortcutStatusMessage.value = "快捷键至少包含一个修饰键（Ctrl/Alt/Shift/Meta）";
    return;
  }
  const target = capturingShortcut.value;
  try {
    if (target === "spotlight") {
      settings.value.spotlightShortcut = value;
    } else {
      settings.value.selectionShortcut = value;
    }

    if (!runningInBrowser.value) {
      await tauriService.saveSettings(settings.value);
    } else {
      localStorage.setItem("aerotrans_settings", JSON.stringify(settings.value));
    }
    shortcutStatusMessage.value = `已记录并生效：${value}`;
    settingsMessage.value = "设置已保存";
  } catch (err) {
    shortcutStatusMessage.value = `快捷键应用失败：${String(err)}`;
  } finally {
    capturingShortcut.value = null;
  }
}

async function toggleSpotlightWindow() {
  if (runningInBrowser.value) {
    settingsMessage.value = "浏览器模式下无法打开独立 Spotlight 窗口";
    return;
  }

  try {
    await tauriService.toggleSpotlightWindow();
  } catch (err) {
    settingsMessage.value = `Spotlight 打开失败: ${String(err)}`;
  }
}

async function updateCompanion() {
  if (!settings.value.companionEnabled || translatingCompanion.value || runningInBrowser.value) return;

  const now = Date.now();
  const text = await tauriService.readClipboardText();
  if (!shouldReadClipboard(defaultClipboardPolicy(), now, lastClipboardReadMs.value, text)) return;

  const normalized = (text ?? "").trim();
  if (!normalized || normalized === lastClipboardText.value) return;

  translatingCompanion.value = true;
  try {
    companionRawText.value = normalized;
    lastClipboardText.value = normalized;
    lastClipboardReadMs.value = now;
    companionResult.value = await tauriService.translate({
      text: normalized,
      context: null,
      provider: settings.value.translationProvider
    });
  } catch (err) {
    console.warn(err);
  } finally {
    translatingCompanion.value = false;
  }
}

async function handleSelectionTranslateTriggered() {
  if (windowLabel.value !== "main") return;
  const text = await tauriService.readClipboardText();
  const normalized = (text ?? "").trim();
  if (!normalized) {
    settingsMessage.value = "未读取到选中文本。请先选中文本，再按一次划词快捷键。";
    return;
  }

  activeTab.value = "spotlight";
  inputText.value = normalized;
  await translateNow();
}

function mountSpotlightKeybind() {
  keydownHandler = (event: KeyboardEvent) => {
    if (capturingShortcut.value) {
      event.preventDefault();
      const mainKey = normalizeMainKey(event);
      if (!mainKey) return;

      const parts: string[] = [];
      if (event.altKey) parts.push("Alt");
      if (event.ctrlKey) parts.push("Ctrl");
      if (event.shiftKey) parts.push("Shift");
      if (event.metaKey) parts.push("Meta");
      parts.push(mainKey);
      void applyCapturedShortcut(parts.join("+"));
      return;
    }

    if (!isSpotlightWindow.value) return;
    if (event.key === "Escape") {
      event.preventDefault();
      void hideCurrentWindow();
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void translateNow();
    }
  };

  window.addEventListener("keydown", keydownHandler);
}

onMounted(async () => {
  await bootstrap();
  mountSpotlightKeybind();

  if (!isSpotlightWindow.value) {
    companionTimer = window.setInterval(updateCompanion, 420);
    await applyReadingCompactMode(readingCompactMode.value);
  }

  await refreshAlwaysOnTopState();

  if (!runningInBrowser.value && windowLabel.value === "main") {
    const { listen } = await import("@tauri-apps/api/event");
    selectionHotkeyUnlisten = await listen("selection_translate_triggered", async () => {
      await handleSelectionTranslateTriggered();
    });
  }
});

onUnmounted(() => {
  if (typeof window !== "undefined" && "speechSynthesis" in window) {
    window.speechSynthesis.cancel();
  }
  if (companionTimer) window.clearInterval(companionTimer);
  if (keydownHandler) window.removeEventListener("keydown", keydownHandler);
  if (selectionHotkeyUnlisten) {
    selectionHotkeyUnlisten();
    selectionHotkeyUnlisten = undefined;
  }
});
</script>

<template>
  <div class="min-h-screen p-3 text-ink" :style="appShellStyle" @mousedown.left="handleGlobalMouseDown">
    <template v-if="isSpotlightWindow">
      <div class="mx-auto mt-2 w-full max-w-[520px] rounded-2xl border border-slate-200 bg-white/86 p-3 shadow-float">
        <div class="mb-2 flex items-center justify-between gap-2">
          <div class="cursor-move select-none" data-tauri-drag-region @mousedown.left="startWindowDrag">
            <p class="text-xs uppercase tracking-wide text-slate-500">AeroTrans Spotlight</p>
          </div>
          <div class="flex items-center gap-2">
            <select v-model="settings.translationProvider" class="rounded-md border border-slate-200 px-2 py-1 text-xs">
              <option value="google">Google</option>
              <option value="youdao">有道</option>
              <option value="mymemory">MyMemory</option>
              <option value="ai">AI</option>
            </select>
            <button class="rounded-md bg-slate-900 px-2 py-1 text-xs text-white" @click="hideCurrentWindow">隐藏</button>
          </div>
        </div>

        <div class="grid gap-2">
          <input
            v-model="inputText"
            placeholder="输入单词或短句，回车翻译，Esc 隐藏..."
            class="rounded-lg border border-slate-200 px-3 py-2 outline-none ring-teal-400 focus:ring"
            @keydown.enter.prevent="translateNow"
          />
        </div>

        <div v-if="result" class="mt-4 rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm">
          <p class="text-slate-600">{{ result.translation }}</p>
          <p v-if="result.phonetics.length" class="mt-1 text-xs text-slate-500">
            音标：{{ result.phonetics.join(" / ") }}
          </p>
          <p class="mt-1 text-xs text-slate-500">来源：{{ sourceLabels[result.source] }}</p>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="mx-auto w-full max-w-4xl overflow-hidden rounded-2xl border border-slate-200 bg-white/82 shadow-float">
        <header class="flex items-center justify-between border-b border-slate-200 px-3 py-2" data-tauri-drag-region>
          <div class="cursor-move select-none" data-tauri-drag-region @mousedown.left="startWindowDrag">
            <h1 class="text-base font-semibold">AeroTrans</h1>
            <p class="text-xs text-slate-500">极简 AI 翻译与词汇学习</p>
          </div>
          <div class="flex items-center gap-2">
            <span class="rounded-md bg-teal-50 px-2 py-1 text-xs text-teal-700">{{ status }}</span>
            <button
              class="h-7 w-7 rounded-md bg-slate-100 text-slate-700"
              :title="windowAlwaysOnTop ? '取消置顶' : '窗口置顶'"
              @click="toggleAlwaysOnTop"
            >
              {{ windowAlwaysOnTop ? "📌" : "📍" }}
            </button>
            <button class="h-7 w-7 rounded-md bg-slate-100 text-slate-700" @click="minimizeWindow">─</button>
            <button class="h-7 w-7 rounded-md bg-slate-100 text-slate-700" @click="toggleMaximizeWindow">{{ windowMaximized ? "❐" : "□" }}</button>
            <button class="h-7 w-7 rounded-md bg-rose-500 text-white" @click="closeWindow">×</button>
          </div>
        </header>

        <main class="p-4">
          <template v-if="readingCompactMode">
            <section class="rounded-xl border border-slate-200 bg-card/90 p-3">
              <div class="mb-2 flex items-center justify-between">
                <p class="text-xs text-slate-500">阅读小窗模式</p>
                <div class="flex items-center gap-2">
                  <button class="rounded-md bg-slate-200 px-2 py-1 text-xs text-slate-700" @click="toggleAlwaysOnTop">
                    {{ windowAlwaysOnTop ? "取消置顶" : "窗口置顶" }}
                  </button>
                  <button class="rounded-md bg-slate-900 px-2 py-1 text-xs text-white" @click="toggleReadingCompactMode">
                    返回完整模式
                  </button>
                </div>
              </div>
              <div class="grid gap-2">
                <input
                  v-model="inputText"
                  placeholder="输入单词或句子"
                  class="rounded-lg border border-slate-200 px-3 py-2 outline-none ring-teal-400 focus:ring"
                  @keydown.enter.prevent="translateNow"
                />
                <button class="rounded-lg bg-accent px-3 py-2 text-sm text-white" :disabled="translating" @click="translateNow">
                  {{ translating ? "翻译中..." : "翻译" }}
                </button>
              </div>
              <div v-if="result" class="mt-3 rounded-lg border border-slate-200 bg-slate-50 p-3">
                <div class="flex items-center justify-between gap-2">
                  <p class="text-sm text-slate-700">{{ result.translation }}</p>
                  <button
                    class="rounded-md border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700"
                    :disabled="speaking"
                    @click="speakText(result.translation)"
                  >
                    {{ speaking ? "发音中..." : "发音" }}
                  </button>
                </div>
                <p v-if="result.phonetics.length" class="mt-1 text-xs text-slate-500">
                  音标：{{ result.phonetics.join(" / ") }}
                </p>
              </div>
            </section>
          </template>

          <template v-else>
            <nav class="mb-4 flex flex-wrap gap-2">
              <button
                v-for="tab in tabs"
                :key="tab.key"
                class="rounded-lg px-3 py-2 text-sm transition"
                :class="activeTab === tab.key ? 'bg-ink text-white' : 'bg-slate-100 text-slate-700 hover:bg-slate-200'"
                @click="activeTab = tab.key"
              >
                {{ tab.label }}
              </button>
            </nav>

          <section v-if="activeTab === 'spotlight'" class="rounded-xl border border-slate-200 bg-card/90 p-4">
            <div class="mb-4 flex flex-wrap items-center justify-between gap-2">
              <h2 class="text-lg font-semibold">翻译</h2>
              <button class="rounded-lg bg-slate-900 px-3 py-2 text-xs text-white" @click="toggleSpotlightWindow">打开独立 Spotlight 窗口</button>
            </div>

            <div class="grid gap-3">
              <label class="text-sm">
                翻译源
                <select v-model="settings.translationProvider" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2">
                  <option value="youdao">有道词典</option>
                  <option value="google">Google 翻译</option>
                  <option value="mymemory">MyMemory</option>
                  <option value="ai">AI 翻译（语境）</option>
                </select>
              </label>
              <input
                v-model="inputText"
                placeholder="输入单词或句子"
                class="rounded-lg border border-slate-200 px-3 py-2 outline-none ring-teal-400 focus:ring"
                @keydown.enter.prevent="translateNow"
              />
              <div class="flex flex-wrap gap-2">
                <button class="rounded-lg bg-accent px-4 py-2 text-white" :disabled="translating" @click="translateNow">
                  {{ translating ? "翻译中..." : "翻译" }}
                </button>
                <button class="rounded-lg bg-warm px-4 py-2 text-white" :disabled="!result" @click="addCurrentToVocabulary(true)">
                  加入单词本
                </button>
              </div>
              <textarea
                v-model="contextText"
                rows="3"
                placeholder="可选：补充上下文，AI 模式会使用"
                class="rounded-lg border border-slate-200 px-3 py-2 outline-none ring-teal-400 focus:ring"
              />
            </div>

            <p v-if="errorMessage" class="mt-2 text-sm text-rose-600">{{ errorMessage }}</p>

            <div v-if="result" class="mt-4 rounded-xl border border-slate-200 bg-slate-50 p-4">
              <div class="mb-2 text-xs text-slate-500">
                方向：{{ result.detectedDirection }} · 来源：{{ sourceLabels[result.source] }}
                <span v-if="result.source !== settings.translationProvider">（已自动回退）</span>
              </div>
              <div class="flex items-center justify-between gap-2">
                <p class="text-base">{{ result.translation }}</p>
                <button
                  class="rounded-md border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700"
                  :disabled="speaking"
                  @click="speakText(result.translation)"
                >
                  {{ speaking ? "发音中..." : "发音" }}
                </button>
              </div>
              <p v-if="result.phonetics.length" class="mt-1 text-xs text-slate-500">
                音标：{{ result.phonetics.join(" / ") }}
              </p>
              <div class="mt-3 flex flex-wrap gap-2">
                <span
                  v-for="item in result.glossary"
                  :key="item"
                  class="inline-flex items-center gap-1 rounded-full bg-white px-3 py-1 text-xs text-slate-600"
                >
                  <span>{{ item }}</span>
                  <button class="rounded bg-slate-100 px-1 py-0.5 text-[10px]" :disabled="speaking" @click="speakText(item)">发音</button>
                </span>
              </div>
            </div>
          </section>

          <section v-if="activeTab === 'companion'" class="rounded-xl border border-slate-200 bg-card/90 p-4">
            <div class="mb-4 flex items-center justify-between gap-2">
              <h2 class="text-lg font-semibold">沉浸阅读（Companion）</h2>
              <div class="flex flex-wrap items-center gap-2">
                <label class="inline-flex items-center gap-2 text-sm">
                  <input v-model="settings.companionEnabled" type="checkbox" class="h-4 w-4" />
                  开启剪贴板监听
                </label>
                <button
                  class="rounded-full border border-slate-300 px-2 py-0.5 text-xs text-slate-600"
                  @click="showClipboardHelp = !showClipboardHelp"
                >
                  ?
                </button>
                <button class="rounded-lg bg-slate-900 px-3 py-2 text-xs text-white" @click="toggleReadingCompactMode">
                  {{ readingCompactMode ? "退出阅读小窗模式" : "开启阅读小窗模式" }}
                </button>
              </div>
            </div>
            <div v-if="showClipboardHelp" class="mb-3 rounded-lg border border-slate-200 bg-slate-50 p-3 text-xs text-slate-600">
              该功能会按节流策略读取剪贴板中的纯文本，并自动翻译短词/短句。为减少系统隐私提示，建议仅在阅读场景中开启。
            </div>

            <div class="rounded-xl border border-slate-200 bg-gradient-to-br from-cyan-50 to-amber-50 p-4">
              <p class="mb-2 text-xs uppercase tracking-wide text-slate-500">Companion Preview</p>
              <p class="text-sm text-slate-600">{{ companionRawText || "复制文本后自动刷新（Tauri 模式）" }}</p>
              <p class="mt-2 text-base">{{ companionResult?.translation || "暂无翻译结果" }}</p>
              <p v-if="companionResult?.phonetics?.length" class="mt-1 text-xs text-slate-500">
                音标：{{ companionResult.phonetics.join(" / ") }}
              </p>
            </div>

            <div v-if="readingCompactMode" class="mt-3 rounded-xl border border-teal-200 bg-teal-50 p-3 text-sm text-teal-900">
              阅读小窗模式已开启：窗口已缩小并置顶。你可以在外部阅读时复制/划词，结果会在此自动刷新。
            </div>

            <div class="mt-4 grid gap-3 md:grid-cols-2">
              <label class="text-sm">
                窗口透明度（{{ settings.companionOpacity }}%）
                <input v-model.number="settings.companionOpacity" class="mt-2 w-full" type="range" min="20" max="100" />
                <p class="mt-1 text-xs text-slate-500">该透明度会作用于整个窗口（包含标题栏和内容）。</p>
                <p class="mt-1 text-xs text-slate-500">这是可透视到底层页面的透明，不是毛玻璃模糊。</p>
              </label>
              <label class="inline-flex items-center gap-2 text-sm">
                <input v-model="settings.companionMouseThrough" type="checkbox" class="h-4 w-4" />
                鼠标穿透（用于 Companion 常驻窗）
              </label>
            </div>
          </section>

          <section v-if="activeTab === 'vocabulary'" class="rounded-xl border border-slate-200 bg-card/90 p-4">
            <div class="mb-3 flex items-center justify-between gap-2">
              <h2 class="text-lg font-semibold">单词本</h2>
              <div class="flex gap-2">
                <button
                  class="rounded-lg px-3 py-1 text-xs"
                  :class="vocabularyViewMode === 'review' ? 'bg-slate-900 text-white' : 'bg-slate-200 text-slate-700'"
                  @click="vocabularyViewMode = 'review'"
                >
                  复习模式
                </button>
                <button
                  class="rounded-lg px-3 py-1 text-xs"
                  :class="vocabularyViewMode === 'list' ? 'bg-slate-900 text-white' : 'bg-slate-200 text-slate-700'"
                  @click="vocabularyViewMode = 'list'"
                >
                  列表模式
                </button>
              </div>
            </div>

            <div v-if="vocabularyViewMode === 'review'">
              <div v-if="!currentWord" class="rounded-xl border border-dashed border-slate-300 p-6 text-center text-slate-500">
                当前单词本为空，请先在翻译页添加词条。
              </div>
              <div v-else class="grid gap-3">
                <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
                  <div class="mb-2 text-xs text-slate-500">第 {{ queueState.index + 1 }} / {{ queueState.items.length }} 张</div>
                  <h3 class="text-2xl font-semibold">{{ currentWord.term }}</h3>
                  <p class="mt-2 text-xs text-slate-500">复习模式默认不显示释义，先回忆后再看列表核对。</p>
                </div>

                <div class="flex flex-wrap gap-2">
                  <button class="rounded-lg bg-slate-200 px-3 py-2 text-sm" @click="prevCard">上一张</button>
                  <button class="rounded-lg bg-slate-900 px-3 py-2 text-sm text-white" @click="nextCard">下一张</button>
                  <button class="rounded-lg bg-rose-500 px-3 py-2 text-sm text-white" @click="deleteCurrentCard">删除当前</button>
                  <button class="rounded-lg bg-cyan-600 px-3 py-2 text-sm text-white" :disabled="memoryLoading" @click="generateMemory(false)">
                    {{ memoryLoading ? "生成中..." : "生成联想记忆" }}
                  </button>
                  <button class="rounded-lg bg-amber-500 px-3 py-2 text-sm text-white" :disabled="memoryLoading" @click="generateMemory(true)">
                    重新生成
                  </button>
                </div>

                <div class="rounded-xl bg-amber-50 p-4 text-sm text-amber-900">
                  {{ generatedMemory || currentWord.aiMemory || "暂无联想记忆内容" }}
                </div>
              </div>
            </div>

            <div v-else class="grid gap-2">
              <div v-if="vocabulary.length === 0" class="rounded-xl border border-dashed border-slate-300 p-6 text-center text-slate-500">
                暂无单词。
              </div>
              <div v-for="item in vocabulary" :key="item.id" class="rounded-xl border border-slate-200 bg-white/80 p-3">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <p class="text-lg font-semibold text-slate-900">{{ item.term }}</p>
                    <p class="mt-1 text-sm text-slate-700">{{ item.translation }}</p>
                    <p v-if="item.aiMemory" class="mt-2 text-xs text-amber-900">{{ item.aiMemory }}</p>
                  </div>
                  <button class="rounded-md bg-rose-500 px-2 py-1 text-xs text-white" @click="deleteVocabularyById(item.id)">
                    删除
                  </button>
                </div>
              </div>
            </div>
          </section>

          <section v-if="activeTab === 'settings'" class="grid gap-4">
            <div class="rounded-xl border border-slate-200 bg-card/90 p-4">
              <div class="mb-3 flex items-center justify-between gap-2">
                <h2 class="text-lg font-semibold">快捷键与体验设置</h2>
                <span v-if="shortcutStatusMessage" class="text-xs text-teal-700">{{ shortcutStatusMessage }}</span>
              </div>
              <div class="grid gap-3 md:grid-cols-2">
                <div class="text-sm">
                  <p>Spotlight 快捷键（全局）</p>
                  <div class="mt-1 flex gap-2">
                    <input
                      :value="settings.spotlightShortcut"
                      readonly
                      class="w-full rounded-lg border border-slate-200 px-3 py-2"
                      @click="startShortcutCapture('spotlight')"
                    />
                    <button class="rounded-lg bg-slate-200 px-3 py-2 text-xs" @click="startShortcutCapture('spotlight')">
                      {{ capturingShortcut === "spotlight" ? "按键中..." : "录制" }}
                    </button>
                  </div>
                </div>
                <div class="text-sm">
                  <p>划词翻译快捷键（单次触发）</p>
                  <div class="mt-1 flex gap-2">
                    <input
                      :value="settings.selectionShortcut"
                      readonly
                      class="w-full rounded-lg border border-slate-200 px-3 py-2"
                      @click="startShortcutCapture('selection')"
                    />
                    <button class="rounded-lg bg-slate-200 px-3 py-2 text-xs" @click="startShortcutCapture('selection')">
                      {{ capturingShortcut === "selection" ? "按键中..." : "录制" }}
                    </button>
                  </div>
                </div>
                <label class="inline-flex items-center gap-2 text-sm">
                  <input v-model="settings.ttsEnabled" type="checkbox" class="h-4 w-4" />
                  启用系统 TTS
                </label>
                <label class="text-sm">
                  收录策略
                  <select v-model="settings.collectionMode" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2">
                    <option value="silent_all">全量静默收录</option>
                    <option value="manual_star">仅手动星标收录</option>
                  </select>
                </label>
              </div>

              <div class="mt-4 flex flex-wrap gap-2">
                <button class="rounded-lg bg-ink px-4 py-2 text-sm text-white" @click="saveAllSettings">保存设置</button>
                <button class="rounded-lg bg-slate-900 px-4 py-2 text-sm text-white" @click="toggleSpotlightWindow">测试 Spotlight 弹窗</button>
              </div>
              <p class="mt-2 text-xs text-slate-500">划词快捷键默认 Alt+Shift+L。先选中文本，按一次快捷键即可自动粘贴到翻译页并翻译。</p>
            </div>

            <div class="rounded-xl border border-slate-200 bg-card/90 p-4">
              <h3 class="mb-3 text-lg font-semibold">翻译引擎与 API 设置</h3>
              <div class="grid gap-3 md:grid-cols-2">
                <label class="text-sm">
                  默认翻译源
                  <select v-model="settings.translationProvider" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2">
                    <option value="youdao">有道词典</option>
                    <option value="google">Google 翻译</option>
                    <option value="mymemory">MyMemory</option>
                    <option value="ai">AI 翻译（OpenAI 兼容）</option>
                  </select>
                </label>
                <div class="rounded-lg bg-slate-50 p-3 text-xs text-slate-600">
                  API Key 不会写入 SQLite，使用系统凭据管理器（Windows Credential Manager / macOS Keychain）。
                </div>
                <label class="text-sm">
                  API Base URL
                  <input v-model="apiConfig.baseUrl" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
                </label>
                <label class="text-sm">
                  模型名称
                  <input v-model="apiConfig.model" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
                </label>
                <div class="text-sm md:col-span-2">
                  <p>免费模型预设</p>
                  <div class="mt-2 flex flex-wrap gap-2">
                    <button
                      v-for="(preset, index) in freeModelPresets"
                      :key="preset.name"
                      class="rounded-lg bg-slate-200 px-3 py-2 text-xs text-slate-700"
                      @click="applyModelPreset(index)"
                    >
                      {{ preset.name }}
                    </button>
                  </div>
                </div>
                <label class="text-sm md:col-span-2">
                  API Key（仅输入时覆盖）
                  <input v-model="apiKeyInput" type="password" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
                </label>
                <label class="text-sm md:col-span-2">
                  自动生成记忆提示词
                  <textarea
                    v-model="settings.memoryPrompt"
                    rows="6"
                    class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                    placeholder="输入用于生成单词联想记忆的系统提示词"
                  />
                </label>
              </div>

              <div class="mt-4 flex flex-wrap items-center gap-2">
                <button class="rounded-lg bg-accent px-4 py-2 text-sm text-white" @click="saveApiSettings">保存 API 配置</button>
                <span class="text-sm text-slate-500">API Key 状态：{{ hasApiKey ? "已配置" : "未配置" }}</span>
              </div>
            </div>

            <p v-if="settingsMessage" class="text-sm text-teal-700">{{ settingsMessage }}</p>
          </section>
          </template>
        </main>
      </div>
    </template>
  </div>
</template>
