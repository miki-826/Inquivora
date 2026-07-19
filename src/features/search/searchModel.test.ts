import { describe, expect, it } from "vitest";
import {
  entityTypeLabel,
  parseSearchResults,
  toEntityTypeFilter,
} from "./searchModel";

describe("parseSearchResults", () => {
  it("正しい結果だけを取り出す", () => {
    const results = parseSearchResults([
      { entityType: "file", entityId: "C:/a.md", title: "メモ", snippet: "本文", path: "C:/a.md" },
      { entityType: "task", entityId: "t1", title: "タスク", snippet: "説明" },
      { broken: true },
    ]);
    expect(results).toHaveLength(2);
    expect(results[0].entityType).toBe("file");
    expect(results[1].path).toBeUndefined();
  });

  it("配列でなければ空を返す", () => {
    expect(parseSearchResults(null)).toEqual([]);
  });
});

describe("entityTypeLabel", () => {
  it("種別を日本語ラベルにする", () => {
    expect(entityTypeLabel("meeting")).toBe("議事録");
    expect(entityTypeLabel("unknown")).toBe("unknown");
  });
});

describe("toEntityTypeFilter", () => {
  it("未選択・全選択はnull（全種別）", () => {
    expect(toEntityTypeFilter([])).toBeNull();
    expect(toEntityTypeFilter(["file", "meeting", "task", "event"])).toBeNull();
  });

  it("一部選択はその配列を返す", () => {
    expect(toEntityTypeFilter(["task", "event"])).toEqual(["task", "event"]);
  });
});
