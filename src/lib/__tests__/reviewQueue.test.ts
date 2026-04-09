import { describe, expect, it } from "vitest";
import { createReviewQueue, nextIndex, prevIndex, removeAt } from "../reviewQueue";

describe("reviewQueue", () => {
  it("cycles next index", () => {
    expect(nextIndex(0, 3)).toBe(1);
    expect(nextIndex(2, 3)).toBe(0);
  });

  it("cycles previous index", () => {
    expect(prevIndex(0, 3)).toBe(2);
    expect(prevIndex(2, 3)).toBe(1);
  });

  it("removes current and focuses next item", () => {
    const state = createReviewQueue(["a", "b", "c"], 1);
    const updated = removeAt(state, 1);
    expect(updated.items).toEqual(["a", "c"]);
    expect(updated.index).toBe(1);
  });

  it("returns empty focus when removing last item", () => {
    const state = createReviewQueue(["x"], 0);
    const updated = removeAt(state, 0);
    expect(updated.items).toEqual([]);
    expect(updated.index).toBe(-1);
  });

  it("keeps index in valid range after continuous next", () => {
    let idx = 0;
    for (let i = 0; i < 20; i += 1) {
      idx = nextIndex(idx, 3);
      expect(idx).toBeGreaterThanOrEqual(0);
      expect(idx).toBeLessThan(3);
    }
  });

  it("keeps index in valid range after continuous previous", () => {
    let idx = 2;
    for (let i = 0; i < 20; i += 1) {
      idx = prevIndex(idx, 3);
      expect(idx).toBeGreaterThanOrEqual(0);
      expect(idx).toBeLessThan(3);
    }
  });

  it("moves focus to new last item when deleting tail in-focus", () => {
    const state = createReviewQueue(["a", "b", "c"], 2);
    const updated = removeAt(state, 2);
    expect(updated.items).toEqual(["a", "b"]);
    expect(updated.index).toBe(1);
  });

  it("shifts focus left when deleting an item before current", () => {
    const state = createReviewQueue(["a", "b", "c", "d"], 2);
    const updated = removeAt(state, 0);
    expect(updated.items).toEqual(["b", "c", "d"]);
    expect(updated.index).toBe(1);
  });

  it("ignores invalid delete index", () => {
    const state = createReviewQueue(["a", "b"], 1);
    const updated = removeAt(state, 99);
    expect(updated.items).toEqual(["a", "b"]);
    expect(updated.index).toBe(1);
  });
});
