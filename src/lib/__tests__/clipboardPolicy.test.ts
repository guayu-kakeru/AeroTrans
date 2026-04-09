import { describe, expect, it } from "vitest";
import { shouldReadClipboard, defaultClipboardPolicy } from "../clipboardPolicy";

describe("clipboardPolicy", () => {
  it("accepts short plain text", () => {
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, "hello world");
    expect(ok).toBe(true);
  });

  it("rejects too frequent polling", () => {
    const now = Date.now();
    const ok = shouldReadClipboard(defaultClipboardPolicy(), now, now - 50, "hello");
    expect(ok).toBe(false);
  });

  it("rejects oversized text", () => {
    const value = "a".repeat(2000);
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, value);
    expect(ok).toBe(false);
  });

  it("rejects non-text payload", () => {
    const ok = shouldReadClipboard(defaultClipboardPolicy(), Date.now(), 0, null);
    expect(ok).toBe(false);
  });
});
