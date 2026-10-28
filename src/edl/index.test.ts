import { beforeAll, expect, test } from "bun:test";
import { cliArgs, keeps, loadEdl, parseInstruction, parsePlan, planPrompt, remap, render, resolve } from "./index.ts";

beforeAll(async () => {
  await loadEdl(await Bun.file(new URL("./pkg/edl_bg.wasm", import.meta.url)).arrayBuffer());
});

const transcript = {
  text: "",
  duration: 20,
  segments: [
    { text: "Bonjour.", start: 0, end: 1 },
    { text: "Le prix.", start: 10, end: 12 },
  ],
  words: [],
  wordTimings: "exact" as const,
};

test("the page reads instructions with the CLI's engine", () => {
  const p = parseInstruction("je veux supprimer de 15:39 à 15:43 et les 5:42 dernières minutes");
  expect(p.unparsed).toEqual([]);
  const cuts = resolve(p.edits, 3600);
  expect(cuts.delete).toEqual([
    [939, 943],
    [3258, 3600],
  ]);
  expect(keeps(cuts)).toEqual([
    [0, 939],
    [943, 3258],
  ]);
  expect(cliArgs(cuts)).toBe("--delete 15:39-15:43 --delete 54:18-1:00:00");
});

test("exports and remapping cross the boundary as JSON", () => {
  expect(render({ name: "a.mp4", engine: "Parakeet", transcript }, "srt")).toContain("00:00:10,000 --> 00:00:12,000\nLe prix.");
  const edited = remap(transcript, { version: 1, duration: 20, delete: [[2, 8]] });
  expect(edited.segments[1]).toEqual({ text: "Le prix.", start: 4, end: 6 });
  expect(edited.duration).toBe(14);
});

test("model plans are asked and read the same way as in the CLI", () => {
  expect(planPrompt("coupe le prix", 20, transcript)).toContain("[0:10–0:12] Le prix.");
  expect(parsePlan('ok {"edits":[{"op":"delete","span":"first","seconds":5}],"fillers":true}').edits).toEqual([{ op: "delete", span: "first", seconds: 5 }]);
  expect(() => parsePlan("nothing")).toThrow();
});
