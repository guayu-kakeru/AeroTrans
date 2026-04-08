export interface ApiConfig {
  baseUrl: string;
  model: string;
}

export interface Settings {
  spotlightShortcut: string;
  selectionShortcut: string;
  companionEnabled: boolean;
  companionOpacity: number;
  companionMouseThrough: boolean;
  ttsEnabled: boolean;
  collectionMode: "silent_all" | "manual_star";
}

export interface TranslationRequest {
  text: string;
  context?: string | null;
}

export interface TranslationResult {
  detectedDirection: "zh_to_en" | "en_to_zh";
  translation: string;
  glossary: string[];
  source: "ai" | "fallback";
}

export interface VocabularyItem {
  id: number;
  term: string;
  translation: string;
  contextText?: string | null;
  starred: boolean;
  aiMemory?: string | null;
  createdAt: string;
}
