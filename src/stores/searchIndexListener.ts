import { listen } from "@tauri-apps/api/event";
import { searchIndexPaths } from "../services/search";

const DEBOUNCE_MS = 600;

let initialized = false;
let pending = new Set<string>();
let timer: ReturnType<typeof setTimeout> | null = null;

function flush(): void {
  const paths = [...pending];
  pending = new Set<string>();
  timer = null;
  void searchIndexPaths(paths).catch((err) => console.error("検索索引の増分更新に失敗", err));
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
