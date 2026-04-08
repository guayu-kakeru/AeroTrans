import type {
  ApiConfig,
  Settings,
  TranslationRequest,
  TranslationResult,
  VocabularyItem
} from "../types";

const isTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error("当前运行于浏览器模式，Tauri 命令不可用。");
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const tauriService = {
  async loadSettings(): Promise<Settings> {
    return invokeCommand<Settings>("load_settings");
  },

  async saveSettings(settings: Settings): Promise<void> {
    await invokeCommand("save_settings", { settings });
  },

  async checkShortcutConflict(shortcut: string): Promise<{ conflict: boolean; reason?: string }> {
    return invokeCommand("check_shortcut_conflict", { shortcut });
  },

  async loadApiConfig(): Promise<ApiConfig> {
    return invokeCommand<ApiConfig>("load_api_config");
  },

  async saveApiConfig(config: ApiConfig): Promise<void> {
    await invokeCommand("save_api_config", { config });
  },

  async saveApiKey(apiKey: string): Promise<void> {
    await invokeCommand("save_api_key", { apiKey });
  },

  async hasApiKey(): Promise<boolean> {
    return invokeCommand<boolean>("has_api_key");
  },

  async translate(req: TranslationRequest): Promise<TranslationResult> {
    return invokeCommand<TranslationResult>("translate_text", { request: req });
  },

  async addVocabulary(item: Omit<VocabularyItem, "id" | "createdAt">): Promise<number> {
    return invokeCommand<number>("add_vocabulary", { item });
  },

  async listVocabulary(): Promise<VocabularyItem[]> {
    return invokeCommand<VocabularyItem[]>("list_vocabulary");
  },

  async deleteVocabulary(id: number): Promise<void> {
    await invokeCommand("delete_vocabulary", { id });
  },

  async generateMemory(id: number): Promise<string> {
    return invokeCommand<string>("generate_memory_for_word", { id });
  },

  async regenerateMemory(id: number): Promise<string> {
    return invokeCommand<string>("regenerate_memory_for_word", { id });
  },

  async readClipboardText(): Promise<string | null> {
    return invokeCommand<string | null>("read_clipboard_text");
  },

  async healthCheck(): Promise<string> {
    return invokeCommand<string>("health_check");
  }
};
