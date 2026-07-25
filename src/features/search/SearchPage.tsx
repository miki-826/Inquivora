import { listen } from "@tauri-apps/api/event";
import { RefreshCw, Search } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { reindexWorkspace, searchGlobal } from "../../services/search";
import { useEditorStore } from "../../stores/useEditorStore";
import { useMeetingStore } from "../../stores/useMeetingStore";
import { useTaskStore } from "../../stores/useTaskStore";
import { useCalendarStore } from "../../stores/useCalendarStore";
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
  const selectTask = useTaskStore((s) => s.select);
  const focusEvent = useCalendarStore((s) => s.setFocusEventId);

  const [query, setQuery] = useState("");
  const [selectedTypes, setSelectedTypes] = useState<EntityType[]>([]);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searched, setSearched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const requestIdRef = useRef(0);

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

  useEffect(() => {
    const started = listen("search:index-started", () => setIndexing(true));
    const done = listen("search:index-done", () => setIndexing(false));
    return () => {
      void started.then((un) => un());
      void done.then((un) => un());
    };
  }, []);

  const runSearch = useCallback(async () => {
    const trimmed = query.trim();
    if (!trimmed) {
      requestIdRef.current += 1;
      setResults([]);
      setSearched(false);
      setBusy(false);
      return;
    }
    const requestId = ++requestIdRef.current;
    setBusy(true);
    setError(null);
    try {
      const found = await searchGlobal(trimmed, toEntityTypeFilter(selectedTypes), 100, 0);
      if (requestId !== requestIdRef.current) return;
      setResults(found);
      setSearched(true);
    } catch (err) {
      if (requestId !== requestIdRef.current) return;
      setError(messageOf(err));
    } finally {
      if (requestId === requestIdRef.current) setBusy(false);
    }
  }, [query, selectedTypes]);

  const rebuildIndex = async () => {
    setIndexing(true);
    setError(null);
    try {
      await reindexWorkspace();
    } catch (err) {
      setIndexing(false);
      setError(messageOf(err));
    }
  };

  useEffect(() => {
    const timer = setTimeout(() => void runSearch(), 250);
    return () => clearTimeout(timer);
  }, [runSearch]);

  const toggleType = (type: EntityType) => {
    setSelectedTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type],
    );
  };

  const openResult = async (result: SearchResult) => {
    try {
      if (result.entityType === "file" && result.path) {
        // 検索したファイルは自動的にメモ（エディタ）で開く
        navigate("/workspace");
        await openPath(result.path);
      } else if (result.entityType === "meeting") {
        await selectMeeting(result.entityId);
        navigate("/meetings");
      } else if (result.entityType === "task") {
        selectTask(result.entityId);
        navigate("/tasks");
      } else if (result.entityType === "event") {
        focusEvent(result.entityId);
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
              aria-label="ワークスペース内を検索"
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void runSearch();
              }}
            />
          </div>
          {indexing && <span className="search-page__indexing">インデックス作成中…</span>}
          <button
            type="button"
            className="search-page__reindex"
            disabled={indexing}
            title="ワークスペースの検索索引を更新"
            onClick={() => void rebuildIndex()}
          >
            <RefreshCw size={15} aria-hidden />
            索引を更新
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
              {result.snippet && <p className="search-result__snippet">{result.snippet}</p>}
              {result.path && <p className="search-result__path">{result.path}</p>}
            </button>
          ))}
        </div>
      </div>
    </ThreePaneLayout>
  );
}
