// Cutting: say it ("supprime de 15:39 à 15:43 et les 5:42 dernières minutes"),
// select words, or remove hesitations and pauses. Plain times are read here by
// the same Rust engine as the CLI; anything else goes to the user's model,
// whose plan is shown before it applies. The result renders in the page, and
// the same cuts leave as a cut list or a `verbatim cut` command.

import { useRef, useState } from "react";
import { cliArgs, cutList, fillers, findText, keeps, type Plan, parseInstruction, type Range, remap, render as renderText, resolve, silences } from "./edl/index.ts";
import type { EditorApi } from "./editor.tsx";
import { baseName, download, exportDoc } from "./engine/export.ts";
import { clock } from "./engine/time.ts";
import { useLang } from "./i18n.ts";
import { Icon } from "./icons.tsx";

/** Ranges a plan stands for, on this transcript. */
function planRanges(plan: Plan, api: EditorApi): Range[] {
  const words = api.doc.transcript.words;
  const out: Range[] = [...resolve(plan.edits, api.duration).delete];
  if (plan.fillers) out.push(...fillers(words));
  if (plan.silences) out.push(...silences(words, api.duration, plan.silences));
  for (const p of plan.phrases) out.push(...findText(words, p));
  return cutList(out, api.duration).delete;
}

const shellQuote = (s: string) => (/^[\w./-]+$/.test(s) ? s : `'${s.replaceAll("'", "'\\''")}'`);

export function Montage({ api }: { api: EditorApi }) {
  const { t } = useLang();
  const { doc, duration } = api;
  const cuts = (doc.cuts ?? []) as Range[];
  const [instruction, setInstruction] = useState("");
  const [proposal, setProposal] = useState<{ ranges: Range[]; note: string | null; fromModel: boolean } | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [gap, setGap] = useState(1.5);
  const [render, setRender] = useState<number | null>(null);
  const abort = useRef<AbortController | null>(null);
  const importer = useRef<HTMLInputElement>(null);
  const removed = cuts.reduce((n, [a, b]) => n + b - a, 0);
  const list = cutList(cuts, duration);
  const stem = baseName(doc.name);
  const command = `verbatim cut ${shellQuote(doc.name)} ${cliArgs({ ...list, source: doc.name })} -o ${shellQuote(`${stem}.edit.mp4`)}`;

  async function read() {
    const text = instruction.trim();
    if (!text) return;
    setStatus("");
    setProposal(null);
    const parsed = parseInstruction(text);
    if (parsed.edits.length && !parsed.unparsed.length) {
      setProposal({ ranges: resolve(parsed.edits, duration).delete, note: null, fromModel: false });
      return;
    }
    const { savedSettings, loadLlm, planEdit } = await import("./ai/byok.ts");
    await loadLlm();
    if (!savedSettings()) {
      setStatus(
        t(
          `Not understood without a model: “${parsed.unparsed.join(", ")}”. Use times (“from 15:39 to 15:43”, “the last 5:42”), or set up a model on the AI page.`,
          `Pas compris sans modèle : « ${parsed.unparsed.join(", ")} ». Donnez des temps (« de 15:39 à 15:43 », « les 5:42 dernières minutes »), ou configurez un modèle dans la page IA.`,
        ),
      );
      return;
    }
    setBusy(true);
    abort.current = new AbortController();
    try {
      const plan = await planEdit(text, duration, doc.transcript, abort.current.signal);
      const ranges = planRanges(plan, api);
      setProposal({ ranges, note: plan.note, fromModel: true });
      if (!ranges.length) setStatus(plan.note ?? t("The model found nothing to cut.", "Le modèle n'a rien trouvé à couper."));
    } catch (e) {
      setStatus((e as Error).message);
    } finally {
      setBusy(false);
    }
  }

  async function renderEdit() {
    if (!api.file) return;
    abort.current = new AbortController();
    setRender(0);
    setStatus("");
    try {
      const { renderCut } = await import("./engine/render.ts");
      const out = await renderCut(api.file, keeps(list), setRender, abort.current.signal);
      download(`${stem}.edit.${out.ext}`, out.blob, out.blob.type);
      setStatus(t("The edited file is in your downloads.", "Le fichier monté est dans vos téléchargements."));
    } catch (e) {
      if ((e as Error).name !== "AbortError") setStatus((e as Error).message);
    } finally {
      setRender(null);
    }
  }

  const fillerCount = fillers(doc.transcript.words).length;

  return (
    <div className="montage">
      <form
        className="say"
        onSubmit={(e) => {
          e.preventDefault();
          void read();
        }}
      >
        <label htmlFor="instruction">{t("Say what to cut", "Dites ce qu'il faut couper")}</label>
        <div className="say-row">
          <input
            id="instruction"
            type="text"
            value={instruction}
            autoComplete="off"
            placeholder={t("delete 15:39–15:43 and the last 5:42", "supprime de 15:39 à 15:43 et les 5:42 dernières minutes")}
            onChange={(e) => setInstruction(e.currentTarget.value)}
          />
          <button type="submit" className="secondary" disabled={busy || !instruction.trim()}>
            {busy ? t("Reading…", "Lecture…") : t("Plan", "Préparer")}
          </button>
        </div>
      </form>

      {proposal ? (
        <div className="proposal" aria-live="polite">
          {proposal.fromModel ? <p className="note">{proposal.note ?? t("Plan from your model:", "Plan de votre modèle :")}</p> : null}
          <ul className="cuts">
            {proposal.ranges.map(([a, b]) => (
              <li key={`${a}-${b}`}>
                <button type="button" className="time num" onClick={() => api.seek(a)}>
                  {clock(a)} → {clock(b)}
                </button>
                <span className="num note">{(b - a).toFixed(1)} s</span>
              </li>
            ))}
          </ul>
          <div className="row">
            <button
              type="button"
              className="primary"
              disabled={!proposal.ranges.length}
              onClick={() => {
                api.addCuts(proposal.ranges);
                setProposal(null);
                setInstruction("");
              }}
            >
              <Icon name="cut" size={16} /> {t(`Apply ${proposal.ranges.length} cut(s)`, `Appliquer ${proposal.ranges.length} coupe(s)`)}
            </button>
            <button type="button" className="link" onClick={() => setProposal(null)}>
              {t("Discard", "Abandonner")}
            </button>
          </div>
        </div>
      ) : null}

      <div className="quick">
        <button type="button" className="secondary small" disabled={!fillerCount} onClick={() => api.addCuts(fillers(doc.transcript.words))}>
          {t(`Remove hesitations (${fillerCount})`, `Retirer les hésitations (${fillerCount})`)}
        </button>
        <span className="quick-gap">
          <button type="button" className="secondary small" disabled={!doc.transcript.words.length} onClick={() => api.addCuts(silences(doc.transcript.words, duration, gap))}>
            {t("Remove pauses over", "Retirer les pauses de plus de")}
          </button>
          <input type="number" min={0.3} max={30} step={0.1} value={gap} aria-label={t("Seconds", "Secondes")} onChange={(e) => setGap(Number(e.currentTarget.value) || 1.5)} />
          <span className="note">s</span>
        </span>
      </div>

      <div className="cut-list">
        <div className="cut-list-head">
          <h2>{t("Cuts", "Coupes")}</h2>
          <span className="num note">
            {cuts.length
              ? t(`${cuts.length} · ${clock(removed)} removed · ${clock(duration - removed)} left`, `${cuts.length} · ${clock(removed)} retirées · ${clock(duration - removed)} restantes`)
              : t("none yet", "aucune pour l'instant")}
          </span>
          {api.undo ? (
            <button type="button" className="icon-button" onClick={api.undo} aria-label={t("Undo", "Annuler")}>
              <Icon name="undo" size={16} />
            </button>
          ) : null}
        </div>
        {cuts.length ? (
          <ul className="cuts">
            {cuts.map(([a, b], i) => (
              <li key={`${a}-${b}`}>
                <button type="button" className="time num" onClick={() => api.seek(Math.max(0, a - 2))}>
                  {clock(a)} → {clock(b)}
                </button>
                <span className="num note">{(b - a).toFixed(1)} s</span>
                <button type="button" className="icon-button" onClick={() => api.setCuts(cuts.filter((_, j) => j !== i))} aria-label={t(`Keep ${clock(a)} to ${clock(b)}`, `Garder ${clock(a)} à ${clock(b)}`)}>
                  <Icon name="close" size={14} />
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="note">{t("Select words in the transcript and press Delete, use a segment's scissors, or say it above.", "Sélectionnez des mots dans la transcription puis Suppr, utilisez les ciseaux d'un segment, ou dites-le ci-dessus.")}</p>
        )}
        {cuts.length ? (
          <>
            <label className="check">
              <input type="checkbox" checked={api.preview} onChange={(e) => api.setPreview(e.currentTarget.checked)} />
              {t("Play without the cuts", "Lire sans les coupes")}
            </label>
            <div className="row">
              {render === null ? (
                <button type="button" className="primary" disabled={!api.file} onClick={() => void renderEdit()}>
                  <Icon name="download" size={16} /> {t("Export the edited file", "Exporter le fichier monté")}
                </button>
              ) : (
                <>
                  <div className="meter grow" role="progressbar" aria-label={t("Rendering", "Rendu")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(render * 100)}>
                    <span style={{ inlineSize: `${render * 100}%` }} />
                  </div>
                  <span className="num note">{Math.round(render * 100)} %</span>
                  <button type="button" className="link" onClick={() => abort.current?.abort()}>
                    {t("Stop", "Arrêter")}
                  </button>
                </>
              )}
              <button type="button" className="secondary" onClick={() => download(`${stem}.edit.srt`, renderText(exportDoc({ ...doc, transcript: remap(doc.transcript, list) }), "srt"), "application/x-subrip")}>
                {t("Subtitles of the edit", "Sous-titres du montage")}
              </button>
            </div>
            {!api.file ? <p className="note">{t("Open the file again above to render it.", "Rouvrez le fichier ci-dessus pour l'exporter.")}</p> : null}
          </>
        ) : null}
      </div>

      <details className="handoff">
        <summary>{t("The same cuts in a script", "Les mêmes coupes dans un script")}</summary>
        <p className="note">{t("Render with ffmpeg on your machine (brew install maxgfr/tap/verbatim):", "Rendu par ffmpeg sur votre machine (brew install maxgfr/tap/verbatim) :")}</p>
        <div className="command">
          <code>{command}</code>
          <button type="button" className="icon-button" onClick={() => void navigator.clipboard.writeText(command).then(() => setStatus(t("Command copied.", "Commande copiée.")))} aria-label={t("Copy the command", "Copier la commande")}>
            <Icon name="copy" size={16} />
          </button>
        </div>
        <div className="row">
          <button type="button" className="secondary small" disabled={!cuts.length} onClick={() => download(`${stem}.cuts.json`, `${JSON.stringify({ ...list, source: doc.name }, null, 2)}\n`, "application/json")}>
            {t("Download the cut list", "Télécharger la liste de coupes")}
          </button>
          <button type="button" className="secondary small" onClick={() => importer.current?.click()}>
            {t("Import a cut list", "Importer une liste de coupes")}
          </button>
          <input
            ref={importer}
            type="file"
            accept="application/json,.json"
            hidden
            onChange={async (e) => {
              const f = e.currentTarget.files?.[0];
              e.currentTarget.value = "";
              if (!f) return;
              try {
                const data = JSON.parse(await f.text()) as { delete?: Range[] };
                if (!Array.isArray(data.delete)) throw new Error();
                api.addCuts(data.delete);
              } catch {
                setStatus(t("This is not a cut list.", "Ce n'est pas une liste de coupes."));
              }
            }}
          />
        </div>
      </details>
      <p className="note" role="status">
        {status}
      </p>
    </div>
  );
}
