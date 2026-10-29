import { expect, test } from "bun:test";
import { engineProblem, engineSize, resolveEngine } from "./engines.ts";

test("auto picks Parakeet for its languages and Whisper beyond them", () => {
  expect(resolveEngine("auto", "auto")).toEqual({ kind: "nemo", model: "parakeet" });
  // A named language Canary knows is forced through Canary; Parakeet would guess it.
  expect(resolveEngine("auto", "fr")).toEqual({ kind: "nemo", model: "canary" });
  expect(resolveEngine("auto", "it")).toEqual({ kind: "nemo", model: "parakeet" });
  expect(resolveEngine("auto", "ja")).toEqual({ kind: "whisper", preset: "balanced" });
  expect(resolveEngine("whisper-fast", "fr")).toEqual({ kind: "whisper", preset: "fast" });
});

test("sizes and language problems", () => {
  expect(engineSize({ kind: "nemo", model: "parakeet" })).toBeGreaterThan(600);
  expect(engineSize({ kind: "cloud" })).toBe(0);
  expect(engineProblem({ kind: "nemo", model: "canary" }, "it")).toBe("canary-language");
  expect(engineProblem({ kind: "nemo", model: "canary" }, "fr")).toBeNull();
  expect(engineProblem({ kind: "whisper", preset: "fast" }, "auto")).toBe("whisper-auto");
});
