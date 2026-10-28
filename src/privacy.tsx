import { useLang } from "./i18n.ts";

export function Privacy() {
  const { t } = useLang();
  return (
    <article className="page prose">
      <h1>{t("What stays here", "Ce qui reste ici")}</h1>
      <p>{t("verbatim runs in this tab. Your video or audio is decoded and transcribed by a speech model running on your device (WebGPU or WebAssembly). The file is never uploaded, and never stored: only the transcript, its corrections and its cuts are kept, in this browser's storage, until you delete them.", "verbatim tourne dans cet onglet. Votre vidéo ou votre audio est décodé et transcrit par un modèle de parole qui tourne sur votre appareil (WebGPU ou WebAssembly). Le fichier n'est jamais envoyé, ni enregistré : seuls la transcription, ses corrections et ses coupes sont gardées, dans le stockage de ce navigateur, jusqu'à ce que vous les supprimiez.")}</p>
      <h2>{t("What goes out, and when", "Ce qui sort, et quand")}</h2>
      <ul>
        <li>{t("The speech model, downloaded once from Hugging Face the first time you use it (80 MB to 670 MB), then served from the browser's cache.", "Le modèle de parole, téléchargé une fois depuis Hugging Face à la première utilisation (80 Mo à 670 Mo), puis servi par le cache du navigateur.")}</li>
        <li>{t("Only if you set up an AI provider and ask for a summary, chapters, a translation, a cleanup or an edit it must interpret: the transcript's text, sent to that provider with your key. The media stays here.", "Seulement si vous configurez un fournisseur d'IA et demandez un résumé, des chapitres, une traduction, un nettoyage ou un montage à interpréter : le texte de la transcription, envoyé à ce fournisseur avec votre clé. Le média reste ici.")}</li>
        <li>{t("Only if you pick “Your AI provider” as the engine: the file itself, sent to that provider for transcription. The page says so before it starts.", "Seulement si vous choisissez « Votre fournisseur d'IA » comme moteur : le fichier lui-même, envoyé à ce fournisseur pour la transcription. La page le dit avant de commencer.")}</li>
      </ul>
      <p>{t("Nothing else: no account, no analytics, no tracking. The page's Content Security Policy only allows these destinations, and the build fails if the code mentions any other.", "Rien d'autre : pas de compte, pas de mesure d'audience, pas de pistage. La politique de sécurité du contenu de la page n'autorise que ces destinations, et la compilation échoue si le code en mentionne une autre.")}</p>
      <p>{t("Your API key is encrypted in this browser under a key the page itself cannot read out.", "Votre clé d'API est chiffrée dans ce navigateur sous une clé que la page elle-même ne peut pas extraire.")}</p>
      <p>
        <a href="https://github.com/maxgfr/verbatim">{t("Source code", "Code source")}</a>
      </p>
    </article>
  );
}
