import { invoke } from "@tauri-apps/api/core";
import { parseSearchResults, type SearchResult } from "../features/search/searchModel";

export async function searchGlobal(
  query: string,
  entityTypes: string[] | null,
  limit = 50,
  offset = 0,
): Promise<SearchResult[]> {
  const value = await invoke("search_global", { query, entityTypes, limit, offset });
  return parseSearchResults(value);
}

export async function searchComputerFiles(query: string, limit = 100): Promise<SearchResult[]> {
  const value = await invoke("search_computer_files", { query, limit });
  return parseSearchResults(value);
}

export async function revealComputerFile(path: string): Promise<void> {
  await invoke("search_reveal_computer_file", { path });
}

/// 索引の全再構築を要求する。実処理はバックエンドの別スレッドで行われ、
/// 進捗は search:index-started / search:index-done イベントで通知される。
export async function reindexWorkspace(): Promise<void> {
  await invoke("search_reindex_workspace");
}

export async function searchIndexPaths(paths: string[]): Promise<void> {
  if (paths.length === 0) return;
  await invoke("search_index_paths", { paths });
}
