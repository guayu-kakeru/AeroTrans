<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { defaultApiConfig, defaultSettings } from "./stores/appState";
import { tauriService } from "./services/tauri";
import { defaultClipboardPolicy, shouldReadClipboard } from "./lib/clipboardPolicy";
import { createReviewQueue, nextIndex, prevIndex, removeAt, type ReviewQueueState } from "./lib/reviewQueue";
import type { TranslationResult, VocabularyItem } from "./types";

type TabName = "spotlight" | "companion" | "vocabulary" | "settings";

const tabs: { key: TabName; label: string }[] = [
  { key: "spotlight", label: "Spotlight" },
  { key: "companion", label: "Companion" },
  { key: "vocabulary", label: "单词本" },
  { key: "settings", label: "设置" }
];

const activeTab = ref<TabName>("spotlight");
const status = ref("初始化中...");
const runningInBrowser = ref(false);

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

const vocabulary = ref<VocabularyItem[]>([]);
const queueState = ref<ReviewQueueState<VocabularyItem>>(createReviewQueue([]));
const memoryLoading = ref(false);
const generatedMemory = ref("");

const companionRawText = ref("");
const companionResult = ref<TranslationResult | null>(null);
const lastClipboardReadMs = ref(0);
const lastClipboardText = ref("");
let companionTimer: number | undefined;

const currentWord = computed(() => {
  if (queueState.value.index < 0 || queueState.value.index >= queueState.value.items.length) return null;
  return queueState.value.items[queueState.value.index];
});

const companionCardStyle = computed(() => ({
  opacity: String(Math.max(0.1, Math.min(1, settings.value.companionOpacity / 100)))
}));

function detectDirection(text: string): "zh_to_en" | "en_to_zh" {
  return /[\u4e00-\u9fff]/.test(text) ? "zh_to_en" : "en_to_zh";
}

function fallbackTranslate(text: string): TranslationResult {
  const direction = detectDirection(text);
  return {
    detectedDirection: direction,
    translation:
      direction === "zh_to_en"
        ? `Fallback translation: ${text}`
        : `兜底翻译：${text}`,
    glossary: [text.slice(0, 20)],
    source: "fallback"
  };
}

async function bootstrap() {
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
    if (local) settings.value = JSON.parse(local);
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
  apiConfig.value = await tauriService.loadApiConfig();
  hasApiKey.value = await tauriService.hasApiKey();
  await refreshVocabulary();
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
      ? fallbackTranslate(inputText.value)
      : await tauriService.translate({ text: inputText.value, context: contextText.value || null });
  } catch (err) {
    errorMessage.value = String(err);
  } finally {
    translating.value = false;
  }
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

async function generateMemory(regenerate = false) {
  if (!currentWord.value) return;
  memoryLoading.value = true;
  generatedMemory.value = "";
  try {
    generatedMemory.value = runningInBrowser.value
      ? `词根联想：${currentWord.value.term} -> demo memory`
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
  try {
    if (!runningInBrowser.value) {
      const first = await tauriService.checkShortcutConflict(settings.value.spotlightShortcut);
      if (first.conflict) throw new Error(first.reason ?? "主快捷键冲突");

      const second = await tauriService.checkShortcutConflict(settings.value.selectionShortcut);
      if (second.conflict) throw new Error(second.reason ?? "划词快捷键冲突");

      await tauriService.saveSettings(settings.value);
    } else {
      localStorage.setItem("aerotrans_settings", JSON.stringify(settings.value));
    }
    settingsMessage.value = "设置已保存";
  } catch (err) {
    settingsMessage.value = String(err);
  }
}

async function saveApiSettings() {
  settingsMessage.value = "";
  if (!apiConfig.value.baseUrl.trim() || !apiConfig.value.model.trim()) {
    settingsMessage.value = "API URL 和模型名称不能为空";
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

async function updateCompanion() {
  if (!settings.value.companionEnabled || translatingCompanion.value) return;
  if (runningInBrowser.value) return;

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
    companionResult.value = await tauriService.translate({ text: normalized, context: null });
  } catch (err) {
    console.warn(err);
  } finally {
    translatingCompanion.value = false;
  }
}

onMounted(async () => {
  await bootstrap();
  companionTimer = window.setInterval(updateCompanion, 420);
});

onUnmounted(() => {
  if (companionTimer) window.clearInterval(companionTimer);
});
</script>

<template>
  <div class="min-h-screen p-6 text-ink">
    <header class="mx-auto mb-6 max-w-6xl rounded-2xl bg-white/80 p-5 shadow-float backdrop-blur">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 class="text-2xl font-semibold">AeroTrans</h1>
          <p class="text-sm text-slate-600">极简 AI 翻译与词汇记忆工具</p>
        </div>
        <div class="rounded-xl bg-teal-50 px-3 py-2 text-sm text-teal-700">{{ status }}</div>
      </div>
      <nav class="mt-4 flex flex-wrap gap-2">
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
    </header>

    <main class="mx-auto grid max-w-6xl gap-6">
      <section v-if="activeTab === 'spotlight'" class="rounded-2xl bg-card/90 p-6 shadow-float">
        <h2 class="mb-4 text-xl font-semibold">极速查词（Spotlight）</h2>
        <div class="grid gap-3">
          <input
            v-model="inputText"
            placeholder="输入单词或句子"
            class="rounded-xl border border-slate-200 px-4 py-3 outline-none ring-teal-400 focus:ring"
          />
          <textarea
            v-model="contextText"
            rows="3"
            placeholder="可选：粘贴上下文句子，提升 AI 释义准确度"
            class="rounded-xl border border-slate-200 px-4 py-3 outline-none ring-teal-400 focus:ring"
          />
          <div class="flex flex-wrap gap-2">
            <button class="rounded-lg bg-accent px-4 py-2 text-white" :disabled="translating" @click="translateNow">
              {{ translating ? "翻译中..." : "翻译" }}
            </button>
            <button class="rounded-lg bg-warm px-4 py-2 text-white" :disabled="!result" @click="addCurrentToVocabulary(true)">
              加入单词本
            </button>
          </div>
          <p v-if="errorMessage" class="text-sm text-rose-600">{{ errorMessage }}</p>
        </div>

        <div v-if="result" class="mt-6 rounded-xl border border-slate-200 bg-slate-50 p-4">
          <div class="mb-2 text-sm text-slate-500">
            方向：{{ result.detectedDirection }} · 来源：{{ result.source }}
          </div>
          <p class="text-lg">{{ result.translation }}</p>
          <div class="mt-3 flex flex-wrap gap-2">
            <span v-for="item in result.glossary" :key="item" class="rounded-full bg-white px-3 py-1 text-xs text-slate-600">
              {{ item }}
            </span>
          </div>
        </div>
      </section>

      <section v-if="activeTab === 'companion'" class="rounded-2xl bg-card/90 p-6 shadow-float">
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-xl font-semibold">沉浸阅读（Companion）</h2>
          <label class="inline-flex items-center gap-2 text-sm">
            <input v-model="settings.companionEnabled" type="checkbox" class="h-4 w-4" />
            启用监听
          </label>
        </div>

        <div class="rounded-2xl border border-slate-200 bg-gradient-to-br from-cyan-50 to-amber-50 p-4" :style="companionCardStyle">
          <p class="mb-2 text-xs uppercase tracking-wide text-slate-500">Companion Overlay Preview</p>
          <p class="text-sm text-slate-600">{{ companionRawText || "复制文本后将自动刷新（Tauri 模式）" }}</p>
          <p class="mt-3 text-base">{{ companionResult?.translation || "暂无翻译结果" }}</p>
        </div>

        <div class="mt-4 grid gap-3 md:grid-cols-2">
          <label class="text-sm">
            窗口透明度（{{ settings.companionOpacity }}%）
            <input v-model.number="settings.companionOpacity" class="mt-2 w-full" type="range" min="20" max="100" />
          </label>
          <label class="inline-flex items-center gap-2 text-sm">
            <input v-model="settings.companionMouseThrough" type="checkbox" class="h-4 w-4" />
            鼠标穿透
          </label>
        </div>
      </section>

      <section v-if="activeTab === 'vocabulary'" class="rounded-2xl bg-card/90 p-6 shadow-float">
        <h2 class="mb-4 text-xl font-semibold">单词本与复习卡片</h2>
        <div v-if="!currentWord" class="rounded-xl border border-dashed border-slate-300 p-8 text-center text-slate-500">
          当前单词本为空，先在 Spotlight 页添加词条。
        </div>
        <div v-else class="grid gap-4">
          <div class="rounded-xl border border-slate-200 bg-slate-50 p-4">
            <div class="mb-2 text-xs text-slate-500">第 {{ queueState.index + 1 }} / {{ queueState.items.length }} 张</div>
            <h3 class="text-xl font-semibold">{{ currentWord.term }}</h3>
            <p class="mt-2 text-slate-700">{{ currentWord.translation }}</p>
            <p v-if="currentWord.contextText" class="mt-2 text-sm text-slate-500">上下文：{{ currentWord.contextText }}</p>
          </div>

          <div class="flex flex-wrap gap-2">
            <button class="rounded-lg bg-slate-200 px-3 py-2 text-sm" @click="prevCard">上一张</button>
            <button class="rounded-lg bg-slate-900 px-3 py-2 text-sm text-white" @click="nextCard">下一张</button>
            <button class="rounded-lg bg-rose-500 px-3 py-2 text-sm text-white" @click="deleteCurrentCard">删除当前</button>
            <button class="rounded-lg bg-cyan-600 px-3 py-2 text-sm text-white" :disabled="memoryLoading" @click="generateMemory(false)">
              {{ memoryLoading ? "生成中..." : "生成记忆法" }}
            </button>
            <button class="rounded-lg bg-amber-500 px-3 py-2 text-sm text-white" :disabled="memoryLoading" @click="generateMemory(true)">
              重新生成
            </button>
          </div>

          <div class="rounded-xl bg-amber-50 p-4 text-sm text-amber-900">
            {{ generatedMemory || currentWord.aiMemory || "暂无记忆法内容" }}
          </div>
        </div>
      </section>

      <section v-if="activeTab === 'settings'" class="rounded-2xl bg-card/90 p-6 shadow-float">
        <h2 class="mb-4 text-xl font-semibold">设置中心</h2>
        <div class="grid gap-4 md:grid-cols-2">
          <label class="text-sm">
            Spotlight 快捷键
            <input v-model="settings.spotlightShortcut" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
          </label>
          <label class="text-sm">
            划词翻译快捷键
            <input v-model="settings.selectionShortcut" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
          </label>
          <label class="text-sm">
            API Base URL
            <input v-model="apiConfig.baseUrl" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
          </label>
          <label class="text-sm">
            模型名称
            <input v-model="apiConfig.model" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
          </label>
          <label class="text-sm">
            API Key（仅输入时覆盖）
            <input v-model="apiKeyInput" type="password" class="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2" />
          </label>
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
          <button class="rounded-lg bg-accent px-4 py-2 text-sm text-white" @click="saveApiSettings">保存 API 配置</button>
          <span class="self-center text-sm text-slate-500">
            API Key 状态：{{ hasApiKey ? "已配置" : "未配置" }}
          </span>
        </div>

        <p v-if="settingsMessage" class="mt-3 text-sm text-teal-700">{{ settingsMessage }}</p>
      </section>
    </main>
  </div>
</template>
