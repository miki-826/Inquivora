import { describe, expect, it } from "vitest";
import {
  addOrActivateTab,
  assignTabToPane,
  canActivateEditorTab,
  clampSplitRatio,
  closeTab,
  isPreviewableLanguage,
  languageForExtension,
  reorderTabs,
  shouldActivateFileFromTree,
  startSplitWithTab,
  SPLIT_RATIO_DEFAULT,
  SPLIT_RATIO_MAX,
  SPLIT_RATIO_MIN,
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

describe("clampSplitRatio", () => {
  it("下限・上限でクランプする", () => {
    expect(clampSplitRatio(0.05)).toBe(SPLIT_RATIO_MIN);
    expect(clampSplitRatio(0.95)).toBe(SPLIT_RATIO_MAX);
  });

  it("範囲内の値はそのまま返す", () => {
    expect(clampSplitRatio(0.5)).toBe(0.5);
    expect(clampSplitRatio(0.3)).toBe(0.3);
  });

  it("不正な値は既定値へ戻す", () => {
    expect(clampSplitRatio(Number.NaN)).toBe(SPLIT_RATIO_DEFAULT);
    expect(clampSplitRatio(Number.POSITIVE_INFINITY)).toBe(SPLIT_RATIO_DEFAULT);
  });
});

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

  it("htmlとhtmをhtmlへ対応付ける", () => {
    expect(languageForExtension("html")).toBe("html");
    expect(languageForExtension("htm")).toBe("html");
    expect(languageForExtension("HTML")).toBe("html");
  });

  it("未知の拡張子はplaintext", () => {
    expect(languageForExtension("xyz")).toBe("plaintext");
    expect(languageForExtension(null)).toBe("plaintext");
  });
});

describe("isPreviewableLanguage", () => {
  it("markdownとhtmlはプレビューできる", () => {
    expect(isPreviewableLanguage("markdown")).toBe(true);
    expect(isPreviewableLanguage("html")).toBe(true);
  });

  it("その他の言語はプレビューできない", () => {
    expect(isPreviewableLanguage("typescript")).toBe(false);
    expect(isPreviewableLanguage("plaintext")).toBe(false);
    expect(isPreviewableLanguage("css")).toBe(false);
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

describe("shouldActivateFileFromTree", () => {
  it("ピン留め中はファイルツリーで開いたタブへ表示を切り替えない", () => {
    const pinned = { ...tab("1", "pinned.md"), isPinned: true };
    expect(shouldActivateFileFromTree([pinned], pinned.id)).toBe(false);
  });

  it("通常タブまたは未選択なら開いたタブへ表示を切り替える", () => {
    const normal = tab("1", "normal.md");
    expect(shouldActivateFileFromTree([normal], normal.id)).toBe(true);
    expect(shouldActivateFileFromTree([normal], null)).toBe(true);
  });
});

describe("canActivateEditorTab", () => {
  it("ピン留め中は別の上部タブにも切り替えない", () => {
    const pinned = { ...tab("1", "pinned.md"), isPinned: true };
    const other = tab("2", "other.md");
    expect(canActivateEditorTab([pinned, other], pinned.id, other.id)).toBe(false);
    expect(canActivateEditorTab([pinned, other], pinned.id, pinned.id)).toBe(true);
  });

  it("ピン留めしていなければ別タブへ切り替えられる", () => {
    const current = tab("1", "current.md");
    const other = tab("2", "other.md");
    expect(canActivateEditorTab([current, other], current.id, other.id)).toBe(true);
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

describe("assignTabToPane", () => {
  const panes = { activeTabId: "1", secondaryTabId: "2" };

  it("左ペインへドロップすると左に表示する", () => {
    expect(assignTabToPane(panes, "3", "primary")).toEqual({
      activeTabId: "3",
      secondaryTabId: "2",
    });
  });

  it("右ペインへドロップすると右に表示する", () => {
    expect(assignTabToPane(panes, "3", "secondary")).toEqual({
      activeTabId: "1",
      secondaryTabId: "3",
    });
  });

  it("右のタブを左へドロップすると左右が入れ替わる", () => {
    expect(assignTabToPane(panes, "2", "primary")).toEqual({
      activeTabId: "2",
      secondaryTabId: "1",
    });
  });

  it("左のタブを右へドロップすると左右が入れ替わる", () => {
    expect(assignTabToPane(panes, "1", "secondary")).toEqual({
      activeTabId: "2",
      secondaryTabId: "1",
    });
  });

  it("同じペインへのドロップは何も変えない", () => {
    expect(assignTabToPane(panes, "1", "primary")).toEqual(panes);
    expect(assignTabToPane(panes, "2", "secondary")).toEqual(panes);
  });

  it("分割していないときに右へドロップすると分割を開始する", () => {
    expect(assignTabToPane({ activeTabId: "1", secondaryTabId: null }, "2", "secondary")).toEqual({
      activeTabId: "1",
      secondaryTabId: "2",
    });
  });

  it("分割していないときに左のタブを右へドロップしても分割しない", () => {
    const single = { activeTabId: "1", secondaryTabId: null };
    expect(assignTabToPane(single, "1", "secondary")).toEqual(single);
  });
});

describe("startSplitWithTab", () => {
  const tabs = [tab("1", "a.md"), tab("2", "b.md"), tab("3", "c.md")];

  it("新しいタブを左へ置き、元の表示を右へ残す", () => {
    expect(startSplitWithTab(tabs, "1", "3", "primary")).toEqual({
      activeTabId: "3",
      secondaryTabId: "1",
    });
  });

  it("新しいタブを右へ置き、元の表示を左へ残す", () => {
    expect(startSplitWithTab(tabs, "1", "3", "secondary")).toEqual({
      activeTabId: "1",
      secondaryTabId: "3",
    });
  });

  it("現在のタブ自身でも別タブを反対側へ選んで分割する", () => {
    expect(startSplitWithTab(tabs, "1", "1", "secondary")).toEqual({
      activeTabId: "2",
      secondaryTabId: "1",
    });
  });
});
