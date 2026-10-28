import { lazy, Suspense, useEffect, useState, useSyncExternalStore } from "react";
import { Editor } from "./editor.tsx";
import { History } from "./history.tsx";
import { Home } from "./home.tsx";
import { setLang, useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";
import { Privacy } from "./privacy.tsx";

// The AI SDK and providers load only when someone opens the AI page.
const AiPage = lazy(() => import("./ai-page.tsx").then((m) => ({ default: m.AiPage })));

export type Route = { page: "home" } | { page: "doc"; id: string } | { page: "history" } | { page: "ai" } | { page: "privacy" };

function parse(hash: string): Route {
  const h = hash.replace(/^#\/?/, "");
  const [a, b] = h.split("/");
  if (a === "t" && b) return { page: "doc", id: b };
  if (a === "history" || a === "ai" || a === "privacy") return { page: a };
  return { page: "home" };
}

export const go = (path: string) => {
  location.hash = path;
};

function useRoute(): Route {
  const hash = useSyncExternalStore(
    (fn) => {
      addEventListener("hashchange", fn);
      return () => removeEventListener("hashchange", fn);
    },
    () => location.hash,
  );
  return parse(hash);
}

type Theme = "system" | "light" | "dark";
const THEME_KEY = "verbatim-theme";

function useTheme(): [Theme, () => void] {
  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem(THEME_KEY) as Theme | null) ?? "system");
  useEffect(() => {
    if (theme === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
    localStorage.setItem(THEME_KEY, theme);
  }, [theme]);
  const dark = theme === "dark" || (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  return [theme, () => setTheme(dark ? "light" : "dark")];
}

export function App() {
  const route = useRoute();
  const { lang, t } = useLang();
  const [theme, toggleTheme] = useTheme();
  const dark = theme === "dark" || (theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    window.scrollTo(0, 0);
    document.getElementById("main")?.focus({ preventScroll: true });
  }, [route.page]);
  const link = (page: Route["page"], href: string, label: string) => (
    <a href={href} aria-current={route.page === page ? "page" : undefined}>
      {label}
    </a>
  );
  return (
    <>
      <a className="skip" href="#main" onClick={(e) => (e.preventDefault(), document.getElementById("main")?.focus())}>
        {t("Skip to content", "Aller au contenu")}
      </a>
      <header className="bar">
        <a className="wordmark" href="#/">
          verbatim
        </a>
        <nav aria-label={t("Main", "Principale")}>
          {link("home", "#/", t("Transcribe", "Transcrire"))}
          {link("history", "#/history", t("History", "Historique"))}
          {link("ai", "#/ai", t("AI", "IA"))}
          {link("privacy", "#/privacy", t("Privacy", "Confidentialité"))}
        </nav>
        <div className="bar-tools">
          <div className="segmented" role="group" aria-label={t("Language", "Langue")}>
            <button type="button" aria-pressed={lang === "en"} onClick={() => setLang("en")}>
              EN
            </button>
            <button type="button" aria-pressed={lang === "fr"} onClick={() => setLang("fr")}>
              FR
            </button>
          </div>
          <button type="button" className="icon-button" onClick={toggleTheme} aria-label={dark ? t("Light theme", "Thème clair") : t("Dark theme", "Thème sombre")}>
            <Icon name={dark ? "sun" : "moon"} />
          </button>
        </div>
      </header>
      <main id="main" tabIndex={-1}>
        {route.page === "home" ? <Home /> : null}
        {route.page === "doc" ? <Editor id={route.id} key={route.id} /> : null}
        {route.page === "history" ? <History /> : null}
        {route.page === "ai" ? (
          <Suspense fallback={null}>
            <AiPage />
          </Suspense>
        ) : null}
        {route.page === "privacy" ? <Privacy /> : null}
      </main>
    </>
  );
}
