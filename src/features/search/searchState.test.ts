import { beforeEach, describe, expect, it } from "vitest";
import { useRetainedSearchStore } from "./searchState";

describe("useRetainedSearchStore", () => {
  beforeEach(() => {
    useRetainedSearchStore.setState({
      query: "",
      selectedTypes: [],
      results: [],
      searched: false,
    });
  });

  it("検索画面がアンマウントされても条件と結果を保持できる", () => {
    const result = {
      entityType: "file" as const,
      entityId: "notes/sample.md",
      title: "sample.md",
      snippet: "検索語を含む本文",
      path: "C:/workspace/notes/sample.md",
    };
    const store = useRetainedSearchStore.getState();
    store.setQuery("検索語");
    store.setSelectedTypes(["file"]);
    store.setResults([result]);
    store.setSearched(true);

    expect(useRetainedSearchStore.getState()).toMatchObject({
      query: "検索語",
      selectedTypes: ["file"],
      results: [result],
      searched: true,
    });
  });
});
