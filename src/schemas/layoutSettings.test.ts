import { describe, expect, it } from "vitest";
import { DEFAULT_LAYOUT_SETTINGS, parseLayoutSettings } from "./layoutSettings";

describe("parseLayoutSettings", () => {
  it("正しい値はそのまま採用する", () => {
    const value = {
      leftSidebarWidth: 280,
      rightSidebarWidth: 400,
      lastScreen: "/tasks",
      navigationPosition: "top" as const,
      taskListFontSize: "large" as const,
      uiDensity: "compact" as const,
      showStatusBar: false,
      reduceMotion: true,
      editorFontSize: "large" as const,
      editorWordWrap: false,
      editorSaveMode: "manual" as const,
    };
    expect(parseLayoutSettings(value)).toEqual(value);
  });

  it("古い設定には左側ナビゲーションを補完する", () => {
    expect(
      parseLayoutSettings({
        leftSidebarWidth: 280,
        rightSidebarWidth: 400,
        lastScreen: "/tasks",
      }),
    ).toEqual({
      leftSidebarWidth: 280,
      rightSidebarWidth: 400,
      lastScreen: "/tasks",
      navigationPosition: "side",
      taskListFontSize: "small",
      uiDensity: "comfortable",
      showStatusBar: true,
      reduceMotion: false,
      editorFontSize: "medium",
      editorWordWrap: true,
      editorSaveMode: "auto",
    });
  });

  it("右側・下部のナビゲーション配置を保存できる", () => {
    expect(parseLayoutSettings({ navigationPosition: "right" }).navigationPosition).toBe("right");
    expect(parseLayoutSettings({ navigationPosition: "bottom" }).navigationPosition).toBe(
      "bottom",
    );
  });

  it("メモ帳の保存方法を保存でき、古い設定は自動保存になる", () => {
    expect(parseLayoutSettings({ editorSaveMode: "manual" }).editorSaveMode).toBe("manual");
    expect(parseLayoutSettings({}).editorSaveMode).toBe("auto");
  });

  it("nullや不正値は既定値を返す", () => {
    expect(parseLayoutSettings(null)).toEqual(DEFAULT_LAYOUT_SETTINGS);
    expect(parseLayoutSettings("broken")).toEqual(DEFAULT_LAYOUT_SETTINGS);
    expect(parseLayoutSettings({ leftSidebarWidth: "wide" })).toEqual(DEFAULT_LAYOUT_SETTINGS);
  });

  it("範囲外の幅はクランプする", () => {
    const parsed = parseLayoutSettings({
      leftSidebarWidth: 10,
      rightSidebarWidth: 9999,
      lastScreen: "/meetings",
    });
    expect(parsed.leftSidebarWidth).toBe(200);
    expect(parsed.rightSidebarWidth).toBe(600);
    expect(parsed.lastScreen).toBe("/meetings");
  });

  it("未知の画面パスは既定画面へ戻す", () => {
    const parsed = parseLayoutSettings({
      leftSidebarWidth: 320,
      rightSidebarWidth: 360,
      lastScreen: "https://evil.example/",
    });
    expect(parsed.lastScreen).toBe("/workspace");
  });
});
