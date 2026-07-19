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

export async function reindexWorkspace(): Promise<number> {
  const value = await invoke("search_reindex_workspace");
  return typeof value === "number" ? value : 0;
}
