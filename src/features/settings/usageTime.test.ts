import { describe, expect, it } from "vitest";
import { formatApiUsageTime } from "./usageTime";

describe("formatApiUsageTime", () => {
  it("UTCのAPI記録を指定したPCタイムゾーンへ変換する", () => {
    expect(formatApiUsageTime("2026-08-01T02:46:00Z", "Asia/Tokyo")).toBe(
      "2026-08-01 11:46",
    );
  });

  it("不正な日時は元の値を表示する", () => {
    expect(formatApiUsageTime("unknown")).toBe("unknown");
  });
});
