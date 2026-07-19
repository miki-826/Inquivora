import { z } from "zod";

export const searchResultSchema = z.object({
  entityType: z.string(),
  entityId: z.string(),
  title: z.string(),
  snippet: z.string(),
  path: z.string().nullish(),
});

export type SearchResult = z.infer<typeof searchResultSchema>;

export const ENTITY_TYPES = ["file", "meeting", "task", "event"] as const;
export type EntityType = (typeof ENTITY_TYPES)[number];

const ENTITY_LABELS: Record<string, string> = {
  file: "ファイル",
  meeting: "議事録",
  task: "タスク",
  event: "予定",
};

export function entityTypeLabel(type: string): string {
  return ENTITY_LABELS[type] ?? type;
}

export function parseSearchResults(value: unknown): SearchResult[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((entry) => {
    const parsed = searchResultSchema.safeParse(entry);
    return parsed.success ? [parsed.data] : [];
  });
}

/// 選択中の種別フィルタをコマンド引数へ変換する。全選択・未選択はnull（全種別）。
export function toEntityTypeFilter(selected: EntityType[]): string[] | null {
  if (selected.length === 0 || selected.length === ENTITY_TYPES.length) {
    return null;
  }
  return [...selected];
}
