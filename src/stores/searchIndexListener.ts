import { listen } from "@tauri-apps/api/event";
import { searchIndexPaths } from "../services/search";

const DEBOUNCE_MS = 1_200;
const MAX_PATHS_PER_BATCH = 256;

let initialized = false;
const pending = new Set<string>();
let timer: ReturnType<typeof setTimeout> | null = null;

function flush(): void {
  const paths = [...pending].slice(0, MAX_PATHS_PER_BATCH);
  for (const path of paths) pending.delete(path);
  timer = null;
  if (paths.length === 0) return;
  void searchIndexPaths(paths)
    .catch((err) => console.error("検索索引の増分更新に失敗", err))
    .finally(() => {
      if (pending.size > 0 && !timer) {
        timer = setTimeout(flush, DEBOUNCE_MS);
      }
    });
}

/// 外部ファイル変更（file:external-changed）を購読し、変更パスをまとめて
/// 索引へ増分反映する。VS Codeのように手動再構築なしで常に最新へ保つ。
export function initSearchIndexListener(): void {
  if (initialized) {
    return;
  }
  initialized = true;
  void listen<{ paths: string[] }>("file:external-changed", (event) => {
    for (const path of event.payload.paths) {
      pending.add(path);
    }
    if (timer) {
      clearTimeout(timer);
    }
    timer = setTimeout(flush, DEBOUNCE_MS);
  });
}
