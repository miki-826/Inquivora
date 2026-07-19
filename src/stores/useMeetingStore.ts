import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import {
  insertBeforeEndMarker,
  segmentSchema,
  type Meeting,
  type MeetingDecision,
  type OpenQuestion,
  type TaskCandidate,
  type TranscriptSegment,
} from "../features/meetings/meetingModel";
import * as meetings from "../services/meetings";
import type { MeetingStartInput } from "../services/meetings";
import { acceptTaskCandidate } from "../services/tasks";
import { useEditorStore } from "./useEditorStore";

type AudioLevels = {
  mic: number;
  loopback: number;
};

type MeetingAi = {
  summary: string | null;
  decisions: MeetingDecision[];
  candidates: TaskCandidate[];
  openQuestions: OpenQuestion[];
};

const EMPTY_AI: MeetingAi = { summary: null, decisions: [], candidates: [], openQuestions: [] };

type MeetingStore = {
  meetings: Meeting[];
  activeMeeting: Meeting | null;
  selectedMeetingId: string | null;
  segments: TranscriptSegment[];
  levels: AudioLevels;
  ai: MeetingAi;
  summaryAvailable: boolean;
  generatingSummary: boolean;
  error: string | null;
  busy: boolean;
  loadMeetings: () => Promise<void>;
  selectMeeting: (meetingId: string | null) => Promise<void>;
  start: (input: MeetingStartInput) => Promise<boolean>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  stop: () => Promise<void>;
  remove: (meetingId: string) => Promise<void>;
  generateSummary: (meetingId: string) => Promise<void>;
  acceptCandidate: (candidateId: string) => Promise<void>;
  clearError: () => void;
};

function messageOf(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, "/").toLowerCase();
}

export const useMeetingStore = create<MeetingStore>((set, get) => ({
  meetings: [],
  activeMeeting: null,
  selectedMeetingId: null,
  segments: [],
  levels: { mic: 0, loopback: 0 },
  ai: EMPTY_AI,
  summaryAvailable: false,
  generatingSummary: false,
  error: null,
  busy: false,

  loadMeetings: async () => {
    try {
      const list = await meetings.listMeetings();
      const active = list.find((m) => m.status === "recording" || m.status === "paused") ?? null;
      set({ meetings: list, activeMeeting: active });
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  selectMeeting: async (meetingId) => {
    set({ selectedMeetingId: meetingId, segments: [], ai: EMPTY_AI });
    if (!meetingId) {
      return;
    }
    try {
      const [segments, meeting, decisions, candidates, available] = await Promise.all([
        meetings.listSegments(meetingId),
        meetings.getMeeting(meetingId),
        meetings.listDecisions(meetingId),
        meetings.listCandidates(meetingId),
        meetings.isSummaryAvailable().catch(() => false),
      ]);
      if (get().selectedMeetingId === meetingId) {
        set({
          segments,
          summaryAvailable: available,
          ai: {
            summary: meeting?.summary ?? null,
            decisions,
            candidates,
            openQuestions: [],
          },
        });
      }
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  start: async (input) => {
    set({ busy: true, error: null });
    try {
      const meeting = await meetings.startMeeting(input);
      if (meeting) {
        set({
          activeMeeting: meeting,
          selectedMeetingId: meeting.id,
          segments: [],
          levels: { mic: 0, loopback: 0 },
        });
        await get().loadMeetings();
        set({ activeMeeting: meeting, selectedMeetingId: meeting.id });
      }
      return meeting != null;
    } catch (error) {
      set({ error: messageOf(error) });
      return false;
    } finally {
      set({ busy: false });
    }
  },

  pause: async () => {
    const meeting = get().activeMeeting;
    if (!meeting) return;
    try {
      await meetings.pauseMeeting(meeting.id);
      set({ activeMeeting: { ...meeting, status: "paused" } });
      await get().loadMeetings();
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  resume: async () => {
    const meeting = get().activeMeeting;
    if (!meeting) return;
    try {
      await meetings.resumeMeeting(meeting.id);
      set({ activeMeeting: { ...meeting, status: "recording" } });
      await get().loadMeetings();
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  stop: async () => {
    const meeting = get().activeMeeting;
    if (!meeting) return;
    set({ busy: true });
    try {
      await meetings.stopMeeting(meeting.id);
      set({ activeMeeting: null, levels: { mic: 0, loopback: 0 } });
      await get().loadMeetings();
    } catch (error) {
      set({ error: messageOf(error) });
    } finally {
      set({ busy: false });
    }
  },

  remove: async (meetingId) => {
    try {
      await meetings.deleteMeeting(meetingId);
      if (get().activeMeeting?.id === meetingId) {
        set({ activeMeeting: null });
      }
      if (get().selectedMeetingId === meetingId) {
        set({ selectedMeetingId: null, segments: [] });
      }
      await get().loadMeetings();
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  generateSummary: async (meetingId) => {
    set({ generatingSummary: true, error: null });
    try {
      const result = await meetings.generateSummary(meetingId);
      if (get().selectedMeetingId === meetingId) {
        set({
          ai: {
            summary: result.summary,
            decisions: result.decisions,
            candidates: result.taskCandidates,
            openQuestions: result.openQuestions,
          },
        });
      }
    } catch (error) {
      set({ error: messageOf(error) });
    } finally {
      set({ generatingSummary: false });
    }
  },

  acceptCandidate: async (candidateId) => {
    try {
      await acceptTaskCandidate(candidateId);
      set((state) => ({
        ai: {
          ...state.ai,
          candidates: state.ai.candidates.map((c) =>
            c.id === candidateId ? { ...c, status: "accepted" } : c,
          ),
        },
      }));
    } catch (error) {
      set({ error: messageOf(error) });
    }
  },

  clearError: () => set({ error: null }),
}));

type SegmentCommittedPayload = {
  meetingId: string;
  targetFilePath: string;
  segmentMarkdown: string;
  segment: unknown;
};

/// §9.5 確定セグメントの追記。対象ファイルが開いていればMonaco Model、
/// 閉じていればRustの安全な保存で追記する。
async function applySegmentToFile(payload: SegmentCommittedPayload): Promise<void> {
  const endMarker = `<!-- inquivora:meeting:${payload.meetingId}:end -->`;
  const editor = useEditorStore.getState();
  const tab = editor.tabs.find(
    (t) => normalizePath(t.path) === normalizePath(payload.targetFilePath),
  );
  if (tab && (tab.viewType === "editor" || tab.viewType === "markdown-preview")) {
    const content = editor.contents[tab.id];
    if (typeof content === "string") {
      const updated = insertBeforeEndMarker(content, endMarker, payload.segmentMarkdown);
      if (updated !== null) {
        editor.updateContent(tab.id, updated);
        return;
      }
    }
  }
  await meetings.appendSegmentToFile(payload.meetingId, payload.segmentMarkdown);
}

let listenersInitialized = false;

export function initMeetingListeners(): void {
  if (listenersInitialized) {
    return;
  }
  listenersInitialized = true;

  void listen<SegmentCommittedPayload>("meeting:segment-committed", (event) => {
    const payload = event.payload;
    const parsed = segmentSchema.safeParse(payload.segment);
    if (parsed.success) {
      const { selectedMeetingId, segments } = useMeetingStore.getState();
      if (selectedMeetingId === payload.meetingId) {
        useMeetingStore.setState({ segments: [...segments, parsed.data] });
      }
    }
    applySegmentToFile(payload).catch((error) => {
      console.error("セグメントの追記に失敗", error);
      useMeetingStore.setState({ error: messageOf(error) });
    });
  });

  void listen<{ meetingId: string; source: string; rms: number }>(
    "meeting:audio-level",
    (event) => {
      const { activeMeeting, levels } = useMeetingStore.getState();
      if (activeMeeting?.id !== event.payload.meetingId) {
        return;
      }
      const source = event.payload.source === "mic" ? "mic" : "loopback";
      useMeetingStore.setState({ levels: { ...levels, [source]: event.payload.rms } });
    },
  );

  void listen<{ meetingId: string; code: string; message: string }>(
    "meeting:transcription-error",
    (event) => {
      useMeetingStore.setState({ error: `文字起こしに失敗: ${event.payload.message}` });
    },
  );

  void listen<{ meetingId: string; code: string; message: string }>(
    "meeting:audio-error",
    (event) => {
      useMeetingStore.setState({ error: `録音エラー: ${event.payload.message}` });
    },
  );

  void listen<{ meetingId: string }>("meeting:audio-stopped", () => {
    void useMeetingStore.getState().loadMeetings();
  });
}
