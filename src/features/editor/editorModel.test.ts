import { describe, expect, it } from "vitest";
import {
  addOrActivateTab,
  closeTab,
  languageForExtension,
  reorderTabs,
  viewTypeForFile,
  type EditorTab,
} from "./editorModel";

function tab(id: string, path: string): EditorTab {
  return {
    id,
    path,
    name: path.split("\\").pop() ?? path,
    language: "plaintext",
    encoding: "utf8",
    lineEnding: "LF",
    isDirty: false,
    isPinned: false,
    cursorLine: 1,
    cursorColumn: 1,
    viewType: "editor",
  };
}

describe("reorderTabs", () => {
  const tabs = [tab("a", "a.md"), tab("b", "b.md"), tab("c", "c.md")];

  it("ドラッグ元をドロップ先の位置へ移動する", () => {
    expect(reorderTabs(tabs, "a", "c").map((t) => t.id)).toEqual(["b", "c", "a"]);
    expect(reorderTabs(tabs, "c", "a").map((t) => t.id)).toEqual(["c", "a", "b"]);
  });

  it("同じタブや不明なIDは順序を変えない", () => {
    expect(reorderTabs(tabs, "a", "a").map((t) => t.id)).toEqual(["a", "b", "c"]);
    expect(reorderTabs(tabs, "x", "b").map((t) => t.id)).toEqual(["a", "b", "c"]);
  });
});

describe("languageForExtension", () => {
  it("代表的な拡張子をMonaco言語IDへ対応付ける", () => {
    expect(languageForExtension("md")).toBe("markdown");
    expect(languageForExtension("ts")).toBe("typescript");
    expect(languageForExtension("tsx")).toBe("typescript");
    expect(languageForExtension("js")).toBe("javascript");
    expect(languageForExtension("rs")).toBe("rust");
    expect(languageForExtension("py")).toBe("python");
    expect(languageForExtension("json")).toBe("json");
    expect(languageForExtension("yml")).toBe("yaml");
    expect(languageForExtension("ps1")).toBe("powershell");
    expect(languageForExtension("cs")).toBe("csharp");
  });

  it("未知の拡張子はplaintext", () => {
    expect(languageForExtension("xyz")).toBe("plaintext");
    expect(languageForExtension(null)).toBe("plaintext");
  });
});

describe("viewTypeForFile", () => {
  it("画像・PDF・音声・動画を判定する", () => {
    expect(viewTypeForFile("preview", "png")).toBe("image");
    expect(viewTypeForFile("preview", "webp")).toBe("image");
    expect(viewTypeForFile("preview", "pdf")).toBe("pdf");
    expect(viewTypeForFile("preview", "mp3")).toBe("audio");
    expect(viewTypeForFile("preview", "wav")).toBe("audio");
    expect(viewTypeForFile("preview", "mp4")).toBe("video");
  });

  it("編集対象はeditor", () => {
    expect(viewTypeForFile("edit", "md")).toBe("editor");
    expect(viewTypeForFile("unknown", null)).toBe("editor");
  });
});

describe("addOrActivateTab", () => {
  it("新しいタブを末尾へ追加してアクティブにする", () => {
    const result = addOrActivateTab([tab("1", "a.md")], "1", tab("2", "b.md"));
    expect(result.tabs.map((t) => t.id)).toEqual(["1", "2"]);
    expect(result.activeTabId).toBe("2");
  });

  it("同じパスのタブがあれば追加せずアクティブ化する", () => {
    const existing = [tab("1", "a.md"), tab("2", "b.md")];
    const result = addOrActivateTab(existing, "2", tab("3", "a.md"));
    expect(result.tabs).toHaveLength(2);
    expect(result.activeTabId).toBe("1");
  });
});

describe("closeTab", () => {
  const tabs = [tab("1", "a.md"), tab("2", "b.md"), tab("3", "c.md")];

  it("アクティブタブを閉じると右隣をアクティブにする", () => {
    const result = closeTab(tabs, "2", "2");
    expect(result.tabs.map((t) => t.id)).toEqual(["1", "3"]);
    expect(result.activeTabId).toBe("3");
  });

  it("右隣がなければ左隣をアクティブにする", () => {
    const result = closeTab(tabs, "3", "3");
    expect(result.activeTabId).toBe("2");
  });

  it("非アクティブタブを閉じてもアクティブは変わらない", () => {
    const result = closeTab(tabs, "1", "3");
    expect(result.tabs.map((t) => t.id)).toEqual(["1", "2"]);
    expect(result.activeTabId).toBe("1");
  });

  it("最後のタブを閉じるとアクティブはnull", () => {
    const result = closeTab([tab("1", "a.md")], "1", "1");
    expect(result.tabs).toHaveLength(0);
    expect(result.activeTabId).toBeNull();
  });
});
