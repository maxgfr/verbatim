import { expect, test } from "bun:test";
import { collapseLoops } from "./loops.ts";

test("a word looping more than four times is kept once", () => {
  expect(collapseLoops(`ouais c'est ça exactement en gros ${"euh, ".repeat(70)}`)).toBe("ouais c'est ça exactement en gros euh,");
  expect(collapseLoops("non non non non, pas ça")).toBe("non non non non, pas ça");
  expect(collapseLoops("the the the the the the end")).toBe("the end");
});
