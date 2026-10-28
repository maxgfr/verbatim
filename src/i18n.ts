// English and French, following the browser until the user picks one.
// Strings live next to the code that shows them, as t("English", "Français")
// pairs, so a translation never drifts from its context.

import { useSyncExternalStore } from "react";

export type Lang = "en" | "fr";

const KEY = "verbatim-lang";
const listeners = new Set<() => void>();

function initial(): Lang {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === "en" || saved === "fr") return saved;
  } catch {}
  return navigator.language?.toLowerCase().startsWith("fr") ? "fr" : "en";
}

let lang: Lang = typeof navigator === "undefined" ? "en" : initial();
if (typeof document !== "undefined") document.documentElement.lang = lang;

export function setLang(next: Lang) {
  lang = next;
  document.documentElement.lang = next;
  try {
    localStorage.setItem(KEY, next);
  } catch {}
  for (const fn of listeners) fn();
}

export const currentLang = () => lang;

export function useLang() {
  const l = useSyncExternalStore(
    (fn) => {
      listeners.add(fn);
      return () => listeners.delete(fn);
    },
    () => lang,
  );
  return { lang: l, t: (en: string, fr: string) => (l === "fr" ? fr : en) };
}

/** For code outside components (errors, titles). */
export const tr = (en: string, fr: string) => (lang === "fr" ? fr : en);

const units = { en: ["B", "KB", "MB", "GB"], fr: ["o", "Ko", "Mo", "Go"] };

export function bytes(n: number, l: Lang = lang): string {
  let i = 0;
  let v = n;
  while (v >= 1000 && i < 3) {
    v /= 1000;
    i++;
  }
  const digits = i === 0 || v >= 100 ? 0 : v >= 10 ? 1 : 2;
  return `${new Intl.NumberFormat(l, { maximumFractionDigits: digits }).format(v)} ${units[l][i]}`;
}

/** A language code as a name in the interface's language: "fr" → "French" / "français". */
export function languageLabel(code: string, l: Lang = lang): string {
  try {
    return new Intl.DisplayNames([l], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
}

export const dateTime = (ms: number, l: Lang = lang) => new Intl.DateTimeFormat(l, { dateStyle: "medium", timeStyle: "short" }).format(ms);
