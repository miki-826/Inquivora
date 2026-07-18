import { describe, expect, it } from "vitest";
import {
  defaultMeetingFileName,
  insertBeforeEndMarker,
  meetingStatusLabel,
  parseMeeting,
} from "./meetingModel";

describe("parseMeeting", () => {
  it("バックエンド応答を解析できる", () => {
    const meeting = parseMeeting({
      id: "m1",
      workspaceId: null,
      title: "定例会議",
      startedAtUtc: "2026-07-18T01:00:00Z",
      endedAtUtc: null,
      timezone: "Asia/Tokyo",
      targetFilePath: "C:/notes/meeting.md",
      startMarker: "<!-- inquivora:meeting:m1:start -->",
      endMarker: "<!-- inquivora:meeting:m1:end -->",
      summary: null,
      status: "recording",
      createdAt: "2026-07-18T01:00:00Z",
      updatedAt: "2026-07-18T01:00:00Z",
    });
    expect(meeting?.id).toBe("m1");
    expect(meeting?.status).toBe("recording");
  });

  it("不正な応答はnullになる", () => {
    expect(parseMeeting({ id: 1 })).toBeNull();
  });
});

describe("meetingStatusLabel", () => {
  it("状態を日本語で表示する", () => {
    expect(meetingStatusLabel("recording")).toBe("録音中");
    expect(meetingStatusLabel("paused")).toBe("一時停止");
    expect(meetingStatusLabel("completed")).toBe("終了");
  });
});

describe("insertBeforeEndMarker", () => {
  const endMarker = "<!-- inquivora:meeting:m1:end -->";

  it("終了マーカー直前へ挿入する", () => {
    const content = `## 文字起こし\n\n### 10:02 自分\n\n最初\n\n${endMarker}\n`;
    const result = insertBeforeEndMarker(content, endMarker, "### 10:03 PC音声\n\n次\n");
    expect(result).not.toBeNull();
    const markerPos = result!.indexOf(endMarker);
    const insertedPos = result!.indexOf("次");
    expect(insertedPos).toBeGreaterThan(-1);
    expect(insertedPos).toBeLessThan(markerPos);
    expect(result).toContain(`次\n\n${endMarker}`);
  });

  it("マーカーがなければnullを返す", () => {
    expect(insertBeforeEndMarker("本文だけ", endMarker, "### x\n")).toBeNull();
  });
});

describe("defaultMeetingFileName", () => {
  it("日付とタイトルからファイル名を作る", () => {
    const name = defaultMeetingFileName("定例会議", new Date("2026-07-18T10:00:00+09:00"));
    expect(name).toBe("2026-07-18_定例会議.md");
  });

  it("ファイル名に使えない文字は置換される", () => {
    const name = defaultMeetingFileName('a/b\\c:d*e?f"g<h>i|j', new Date("2026-07-18T10:00:00+09:00"));
    expect(name).toBe("2026-07-18_a-b-c-d-e-f-g-h-i-j.md");
  });

  it("空タイトルは会議になる", () => {
    expect(defaultMeetingFileName("  ", new Date("2026-07-18T10:00:00+09:00"))).toBe(
      "2026-07-18_会議.md",
    );
  });
});
