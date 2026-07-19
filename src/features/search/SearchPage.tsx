import { RefreshCw, Search } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { reindexWorkspace, searchGlobal } from "../../services/search";
import { useEditorStore } from "../../stores/useEditorStore";
import { useMeetingStore } from "../../stores/useMeetingStore";
import {
  ENTITY_TYPES,
  entityTypeLabel,
  toEntityTypeFilter,
  type EntityType,
  type SearchResult,
} from "./searchModel";

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export function SearchPage() {
  const navigate = useNavigate();
  const openPath = useEditorStore((s) => s.openPath);
  const selectMeeting = useMeetingStore((s) => s.selectMeeting);

  const [query, setQuery] = useState("");
  const [selectedTypes, setSelectedTypes] = useState<EntityType[]>([]);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searched, setSearched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [reindexing, setReindexing] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const runSearch = useCallback(async () => {
    const trimmed = query.trim();
    if (!trimmed) {
      setResults([]);
      setSearched(false);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const found = await searchGlobal(trimmed, toEntityTypeFilter(selectedTypes), 100, 0);
      setResults(found);
      setSearched(true);
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setBusy(false);
    }
  }, [query, selectedTypes]);

  useEffect(() => {
    const timer = setTimeout(() => void runSearch(), 250);
    return () => clearTimeout(timer);
  }, [runSearch]);

  const toggleType = (type: EntityType) => {
    setSelectedTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type],
    );
  };

  const reindex = async () => {
    setReindexing(true);
    setMessage(null);
    setError(null);
    try {
      const count = await reindexWorkspace();
      setMessage(`${count}件をインデックスしました`);
      if (query.trim()) void runSearch();
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setReindexing(false);
    }
  };

  const openResult = async (result: SearchResult) => {
    try {
      if (result.entityType === "file" && result.path) {
        await openPath(result.path);
        navigate("/workspace");
      } else if (result.entityType === "meeting") {
        await selectMeeting(result.entityId);
        navigate("/meetings");
      } else if (result.entityType === "task") {
        navigate("/tasks");
      } else if (result.entityType === "event") {
        navigate("/calendar");
      }
    } catch (err) {
      setError(messageOf(err));
    }
  };

  return (
    <ThreePaneLayout>
      <div className="search-page">
        <div className="search-page__bar">
          <div className="search-page__input">
            <Search size={16} aria-hidden />
            <input
              ref={inputRef}
              type="text"
              value={query}
              placeholder="ファイル・議事録・タスク・予定を検索（Ctrl+Shift+F）"
              aria-label="全文検索"
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void runSearch();
              }}
            />
          </div>
          <button
            type="button"
            className="search-page__reindex"
            disabled={reindexing}
            onClick={() => void reindex()}
          >
            <RefreshCw size={14} aria-hidden />
            {reindexing ? "再構築中…" : "インデックス再構築"}
          </button>
        </div>

        <div className="search-page__filters">
          {ENTITY_TYPES.map((type) => (
            <button
              key={type}
              type="button"
              className={`search-chip${selectedTypes.includes(type) ? " search-chip--on" : ""}`}
              onClick={() => toggleType(type)}
            >
              {entityTypeLabel(type)}
            </button>
          ))}
        </div>

        {message && <p className="search-page__message">{message}</p>}
        {error && (
          <p className="search-page__error" role="alert">
            {error}
          </p>
        )}

        <div className="search-results">
          {busy && <p className="search-results__status">検索中…</p>}
          {!busy && searched && results.length === 0 && (
            <p className="search-results__status">一致する項目がありません</p>
          )}
          {results.map((result) => (
            <button
              key={`${result.entityType}:${result.entityId}`}
              type="button"
              className="search-result"
              onClick={() => void openResult(result)}
            >
              <div className="search-result__head">
                <span className={`search-result__type search-result__type--${result.entityType}`}>
                  {entityTypeLabel(result.entityType)}
                </span>
                <span className="search-result__title">{result.title}</span>
              </div>
              <p className="search-result__snippet">{result.snippet}</p>
              {result.path && <p className="search-result__path">{result.path}</p>}
            </button>
          ))}
        </div>
      </div>
    </ThreePaneLayout>
  );
}
