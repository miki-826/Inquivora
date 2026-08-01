import { create } from "zustand";
import type { EntityType, SearchResult } from "./searchModel";

type RetainedSearchState = {
  query: string;
  selectedTypes: EntityType[];
  results: SearchResult[];
  searched: boolean;
  setQuery: (query: string) => void;
  setSelectedTypes: (selectedTypes: EntityType[]) => void;
  setResults: (results: SearchResult[]) => void;
  setSearched: (searched: boolean) => void;
};

// ファイルを開いて検索画面から離れても、同じアプリ実行中は検索条件と結果を保持する。
export const useRetainedSearchStore = create<RetainedSearchState>((set) => ({
  query: "",
  selectedTypes: [],
  results: [],
  searched: false,
  setQuery: (query) => set({ query }),
  setSelectedTypes: (selectedTypes) => set({ selectedTypes }),
  setResults: (results) => set({ results }),
  setSearched: (searched) => set({ searched }),
}));
