export interface ClipboardPolicy {
  minIntervalMs: number;
  minLength: number;
  maxLength: number;
}

export function defaultClipboardPolicy(): ClipboardPolicy {
  return {
    minIntervalMs: 350,
    minLength: 1,
    maxLength: 512
  };
}

export function shouldReadClipboard(
  policy: ClipboardPolicy,
  nowMs: number,
  lastReadMs: number,
  value: string | null
): boolean {
  if (nowMs - lastReadMs < policy.minIntervalMs) return false;
  if (typeof value !== "string") return false;

  const text = value.trim();
  if (text.length < policy.minLength) return false;
  if (text.length > policy.maxLength) return false;

  return true;
}
