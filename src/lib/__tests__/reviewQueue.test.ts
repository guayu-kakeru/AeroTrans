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
});
