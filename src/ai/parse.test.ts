import { expect, test } from "bun:test";
import { batches, jsonArray, parseChapters, parseLines, timedLines } from "./parse.ts";

test("finds the JSON array inside prose and fences", () => {
  expect(jsonArray('Here:\n```json\n["a", "b"]\n```')).toEqual(["a", "b"]);
  expect(jsonArray("no array")).toBeNull();
  expect(jsonArray("[not json]")).toBeNull();
});

test("chapters: times as m:ss or seconds, sorted, inside the media, no duplicates", () => {
  const text = '[{"start":"2:00","title":"Prix"},{"start":0,"title":"Accueil"},{"start":"0:00.4","title":"Doublon"},{"start":"9:99","title":"x"},{"start":900,"title":"Trop tard"},{"start":30,"title":"  "}]';
  expect(parseChapters(text, 600)).toEqual([
    { start: 0, title: "Accueil" },
    { start: 120, title: "Prix" },
  ]);
});

test("lines must match the segment count", () => {
  expect(parseLines('["un", "deux"]', 2)).toEqual(["un", "deux"]);
  expect(parseLines('["un"]', 2)).toBeNull();
  expect(parseLines('["un", 2]', 2)).toBeNull();
});

test("timed lines and batches", () => {
  expect(timedLines([{ text: " Bonjour ", start: 65, end: 66 }])).toBe("[1:05] Bonjour\n");
  expect(timedLines([{ text: "a".repeat(50), start: 0, end: 1 }], 10)).toContain("truncated");
  expect(batches([1, 2, 3, 4, 5], 2)).toEqual([[1, 2], [3, 4], [5]]);
});
