import { listen } from "@tauri-apps/api/event";
import { CalendarDays, FileText, ListChecks, Mic2, RefreshCw, Search } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { reindexWorkspace, searchGlobal } from "../../services/search";
import { revealInExplorer } from "../../services/workspace";
import { useMeetingStore } from "../../stores/useMeetingStore";
import { useTaskStore } from "../../stores/useTaskStore";
import { useCalendarStore } from "../../stores/useCalendarStore";
import { useWorkspaceStore } from "../../stores/useWorkspaceStore";
import {
  ENTITY_TYPES,
  entityTypeLabel,
  toEntityTypeFilter,
  type EntityType,
  type SearchResult,
} from "./searchModel";
import { useRetainedSearchStore } from "./searchState";

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function SearchResultIcon({ type }: { type: string }) {
  const props = { size: 16, strokeWidth: 1.8, "aria-hidden": true } as const;
  switch (type) {
    case "meeting":
      return <Mic2 {...props} />;
    case "task":
      return <ListChecks {...props} />;
    case "event":
      return <CalendarDays {...props} />;
    default:
      return <FileText {...props} />;
  }
}

type SearchMenuState = { x: number; y: number; path: string };

async function copyPath(path: string): Promise<void> {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(path);
    return;
  }
  const textarea = document.createElement("textarea");
  textarea.value = path;
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  document.execCommand("copy");
  textarea.remove();
}

function SearchResultMenu({
  menu,
  onClose,
  onError,
}: {
  menu: SearchMenuState;
  onClose: () => void;
  onError: (message: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (event: MouseEvent) => {
      if (!ref.current?.contains(event.target as Node)) onClose();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", escape);
    };
  }, [onClose]);
  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      style={{
        left: Math.max(8, Math.min(menu.x, window.innerWidth - 220)),
        top: Math.max(8, Math.min(menu.y, window.innerHeight - 90)),
      }}
    >
      <button
        type="button"
        role="menuitem"
        className="context-menu__item"
        onClick={() => {
          onClose();
          void copyPath(menu.path).catch((error) => onError(messageOf(error)));
        }}
      >
        ファイルパスをコピー
      </button>
      <button
        type="button"
        role="menuitem"
        className="context-menu__item"
        onClick={() => {
          onClose();
          void revealInExplorer(menu.path).catch((error) => onError(messageOf(error)));
        }}
      >
        エクスプローラーで表示
      </button>
    </div>
  );
}

export function SearchPage() {
  const navigate = useNavigate();
  const revealPath = useWorkspaceStore((s) => s.revealPath);
  const selectMeeting = useMeetingStore((s) => s.selectMeeting);
  const selectTask = useTaskStore((s) => s.select);
  const focusEvent = useCalendarStore((s) => s.setFocusEventId);

  const query = useRetainedSearchStore((s) => s.query);
  const selectedTypes = useRetainedSearchStore((s) => s.selectedTypes);
  const results = useRetainedSearchStore((s) => s.results);
  const searched = useRetainedSearchStore((s) => s.searched);
  const setQuery = useRetainedSearchStore((s) => s.setQuery);
  const setSelectedTypes = useRetainedSearchStore((s) => s.setSelectedTypes);
  const setResults = useRetainedSearchStore((s) => s.setResults);
  const setSearched = useRetainedSearchStore((s) => s.setSearched);
  const [busy, setBusy] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [menu, setMenu] = useState<SearchMenuState | null>(null);
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
  }, [query, selectedTypes, setResults, setSearched]);

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
    setSelectedTypes(
      selectedTypes.includes(type)
        ? selectedTypes.filter((selected) => selected !== type)
        : [...selectedTypes, type],
    );
  };

  const openResult = async (result: SearchResult) => {
    try {
      if (result.entityType === "file" && result.path) {
        // 検索したファイルは自動的にメモ（エディタ）で開く
        await revealPath(result.path);
        navigate("/workspace", { state: { openPath: result.path } });
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
              onContextMenu={(event) => {
                if (!result.path) return;
                event.preventDefault();
                setMenu({ x: event.clientX, y: event.clientY, path: result.path });
              }}
            >
              <span
                className={`search-result__icon search-result__icon--${result.entityType}`}
                title={entityTypeLabel(result.entityType)}
              >
                <SearchResultIcon type={result.entityType} />
              </span>
              <span className="search-result__content">
                <span className="search-result__head">
                  <span className="search-result__title">{result.title}</span>
                  <span className="search-result__kind">{entityTypeLabel(result.entityType)}</span>
                </span>
                {result.path && <span className="search-result__path">{result.path}</span>}
                {result.snippet && <span className="search-result__snippet">{result.snippet}</span>}
              </span>
            </button>
          ))}
        </div>
        {menu && (
          <SearchResultMenu
            menu={menu}
            onClose={() => setMenu(null)}
            onError={setError}
          />
        )}
      </div>
    </ThreePaneLayout>
  );
}
