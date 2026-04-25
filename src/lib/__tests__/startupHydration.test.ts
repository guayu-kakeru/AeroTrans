import { describe, expect, it } from "vitest";
import {
  INTERACTIVE_BOOTSTRAP_STATUS,
  resolveHydratedValue,
} from "../startupHydration";

describe("startupHydration", () => {
  it("uses loaded values before the user edits anything", () => {
    expect(resolveHydratedValue("default", "loaded", false)).toBe("loaded");
  });

  it("preserves user edits while bootstrap is still running", () => {
    expect(resolveHydratedValue("user draft", "loaded", true)).toBe(
      "user draft",
    );
  });

  it("starts with an interactive bootstrap status", () => {
    expect(INTERACTIVE_BOOTSTRAP_STATUS).toBe("可用，正在同步配置...");
  });
});
