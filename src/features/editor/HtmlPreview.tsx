import { useEffect, useState } from "react";
import { readFileBase64 } from "../../services/workspace";
import { buildPreviewDocument, directoryOf } from "./htmlPreviewModel";

const REBUILD_DEBOUNCE_MS = 250;

type HtmlPreviewProps = {
  content: string;
  path: string;
  name: string;
};

/**
 * HTMLをそのままの見た目で表示する。CSS・画像などの相対参照は
 * ワークスペースから読み込んで埋め込み、iframeはsandboxで隔離する。
 */
export function HtmlPreview({ content, path, name }: HtmlPreviewProps) {
  const [document, setDocument] = useState<string | null>(null);
  const [missing, setMissing] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(() => {
      buildPreviewDocument(content, directoryOf(path), readFileBase64)
        .then((result) => {
          if (cancelled) return;
          setDocument(result.document);
          setMissing(result.missing);
          setError(null);
        })
        .catch((e: unknown) => {
          if (!cancelled) setError(e instanceof Error ? e.message : String(e));
        });
    }, REBUILD_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [content, path]);

  return (
    <div className="html-preview" data-testid="html-preview">
      {error && (
        <p className="html-preview__notice" role="alert">
          プレビューを作成できませんでした: {error}
        </p>
      )}
      {missing.length > 0 && (
        <p className="html-preview__notice">
          読み込めなかったファイル: {missing.join(", ")}
        </p>
      )}
      {document !== null ? (
        <iframe
          className="html-preview__frame"
          title={`${name} プレビュー`}
          sandbox=""
          srcDoc={document}
        />
      ) : (
        !error && <p className="html-preview__notice">読み込み中…</p>
      )}
    </div>
  );
}
