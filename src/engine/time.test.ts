import { expect, test } from "bun:test";
import { clock, parseClock } from "./time.ts";

test("clock and its inverse", () => {
  expect(clock(7)).toBe("0:07");
  expect(clock(3723)).toBe("1:02:03");
  expect(parseClock("12:48")).toBe(768);
  expect(parseClock("1:02:03")).toBe(3723);
  expect(parseClock("45")).toBe(45);
  expect(parseClock("1h02")).toBe(3720);
  expect(parseClock("soon")).toBeNull();
});
