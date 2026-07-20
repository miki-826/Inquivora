import { ExternalLink } from "lucide-react";
import { useEffect, useState } from "react";
import type { EditorTab } from "./editorModel";
import { openExternal, readFileBase64 } from "../../services/workspace";

const MIME_BY_EXTENSION: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  svg: "image/svg+xml",
  pdf: "application/pdf",
  wav: "audio/wav",
  mp3: "audio/mpeg",
  m4a: "audio/mp4",
  mp4: "video/mp4",
  webm: "video/webm",
};

type MediaViewProps = {
  tab: EditorTab;
};

export function MediaView({ tab }: MediaViewProps) {
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const extension = tab.name.split(".").pop()?.toLowerCase() ?? "";
  const mime = MIME_BY_EXTENSION[extension];
  const loadable = mime !== undefined;

  useEffect(() => {
    let cancelled = false;
    if (!loadable) return;
    readFileBase64(tab.path)
      .then((base64) => {
        if (!cancelled) setDataUrl(`data:${mime};base64,${base64}`);
      })
      .catch((e) => {
        if (!cancelled) {
          setError(e && typeof e === "object" && "message" in e ? String(e.message) : String(e));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [tab.path, mime, loadable]);

  return (
    <div className="media-view">
      {tab.viewType === "image" && dataUrl && (
        <img src={dataUrl} alt={tab.name} className="media-view__image" />
      )}
      {tab.viewType === "audio" && dataUrl && <audio src={dataUrl} controls />}
      {tab.viewType === "video" && dataUrl && <video src={dataUrl} controls className="media-view__video" />}
      {tab.viewType === "pdf" && dataUrl && (
        <iframe
          src={dataUrl}
          title={`${tab.name} PDFプレビュー`}
          className="media-view__pdf"
          onError={() => setError("PDFをアプリ内で表示できませんでした")}
        />
      )}
      {loadable && !dataUrl && !error && <p className="media-view__hint">読み込み中…</p>}
      {(!loadable || error) && (
        <div className="media-view__fallback">
          {error && <p className="media-view__hint">{error}</p>}
          <p className="media-view__hint">このファイルはアプリ内でプレビューできません。</p>
          <button type="button" onClick={() => openExternal(tab.path)}>
            <ExternalLink size={14} aria-hidden />
            既定のアプリで開く
          </button>
        </div>
      )}
    </div>
  );
}
