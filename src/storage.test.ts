import { afterEach, describe, expect, test } from "bun:test";
import { readLocal, writeLocal } from "./storage.ts";

const original = globalThis.localStorage;
const install = (storage: Partial<Storage> | undefined) => Object.defineProperty(globalThis, "localStorage", { value: storage, configurable: true, writable: true });
afterEach(() => install(original));

describe("storage on a shared origin", () => {
  test("a full storage makes writes report false instead of throwing", () => {
    install({
      getItem: () => null,
      setItem: () => {
        throw new DOMException("quota", "QuotaExceededError");
      },
    });
    expect(writeLocal("verbatim-engine", "auto")).toBe(false);
  });

  test("a blocked or missing storage reads as empty", () => {
    install({
      getItem: () => {
        throw new DOMException("denied", "SecurityError");
      },
    });
    expect(readLocal("verbatim-engine")).toBeNull();
    install(undefined);
    expect(readLocal("verbatim-engine")).toBeNull();
    expect(writeLocal("verbatim-engine", "auto")).toBe(false);
  });

  test("a working storage keeps the value", () => {
    const m = new Map<string, string>();
    install({ getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v) });
    expect(writeLocal("verbatim-engine", "auto")).toBe(true);
    expect(readLocal("verbatim-engine")).toBe("auto");
  });
});
