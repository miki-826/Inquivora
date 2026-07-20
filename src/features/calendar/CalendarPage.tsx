import type { DateSelectArg, EventApi, EventClickArg } from "@fullcalendar/core";
import jaLocale from "@fullcalendar/core/locales/ja";
import dayGridPlugin from "@fullcalendar/daygrid";
import interactionPlugin, { Draggable, type EventReceiveArg } from "@fullcalendar/interaction";
import FullCalendar from "@fullcalendar/react";
import timeGridPlugin from "@fullcalendar/timegrid";
import { formatInTimeZone, fromZonedTime } from "date-fns-tz";
import { useEffect, useRef, useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import { getEvent, type EventPatch } from "../../services/events";
import { useCalendarStore } from "../../stores/useCalendarStore";
import { TASK_COLOR_VALUES, TOKYO_TZ, type Task } from "../tasks/taskModel";
import { buildCalendarInputs, shiftDateString } from "./calendarModel";
import { EventPanel, type CalendarSelection, type EventDraft } from "./EventPanel";
import "./calendarEnhancements.css";
import "../tasks/taskColors.css";

function calStrToUtc(value: string, allDay: boolean): string {
  if (allDay) {
    return fromZonedTime(`${value}T00:00:00`, TOKYO_TZ).toISOString();
  }
  return new Date(value).toISOString();
}

function draftFromSelect(info: DateSelectArg): EventDraft {
  if (info.allDay) {
    return {
      title: "",
      allDay: true,
      startDate: info.startStr,
      startTime: "",
      endDate: shiftDateString(info.endStr, -1),
      endTime: "",
      location: "",
      description: "",
    };
  }
  return {
    title: "",
    allDay: false,
    startDate: formatInTimeZone(info.start, TOKYO_TZ, "yyyy-MM-dd"),
    startTime: formatInTimeZone(info.start, TOKYO_TZ, "HH:mm"),
    endDate: formatInTimeZone(info.end, TOKYO_TZ, "yyyy-MM-dd"),
    endTime: formatInTimeZone(info.end, TOKYO_TZ, "HH:mm"),
    location: "",
    description: "",
  };
}

type EventChangeArg = { event: EventApi; revert: () => void };

function UnscheduledTasks({ tasks }: { tasks: Task[] }) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const unscheduled = tasks.filter(
    (task) => !task.dueAtUtc && task.status !== "completed" && task.status !== "cancelled",
  );

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const draggable = new Draggable(element, {
      itemSelector: ".calendar-unscheduled__item",
      eventData: (item) => {
        const id = item.dataset.taskId ?? "";
        const color = item.dataset.taskColor as Task["color"];
        return {
          id: `task:${id}`,
          title: item.dataset.taskTitle ?? "タスク",
          backgroundColor: TASK_COLOR_VALUES[color] ?? TASK_COLOR_VALUES.blue,
          borderColor: TASK_COLOR_VALUES[color] ?? TASK_COLOR_VALUES.blue,
          textColor: "#ffffff",
          extendedProps: { kind: "task", sourceId: id },
        };
      },
    });
    return () => draggable.destroy();
  }, []);

  return (
    <div className="calendar-unscheduled" ref={containerRef}>
      <h2 className="pane-title">未予定タスク</h2>
      <p className="calendar-unscheduled__hint">カレンダーへドラッグして期日を設定</p>
      {unscheduled.length === 0 ? (
        <p className="calendar-unscheduled__empty">未予定のタスクはありません</p>
      ) : (
        <div className="calendar-unscheduled__list">
          {unscheduled.map((task) => (
            <div
              key={task.id}
              className="calendar-unscheduled__item"
              data-task-id={task.id}
              data-task-title={task.title}
              data-task-color={task.color}
              style={{ borderLeftColor: TASK_COLOR_VALUES[task.color] }}
              tabIndex={0}
            >
              {task.title}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function CalendarPage() {
  const events = useCalendarStore((s) => s.events);
  const tasks = useCalendarStore((s) => s.tasks);
  const error = useCalendarStore((s) => s.error);
  const loadRange = useCalendarStore((s) => s.loadRange);
  const updateEvent = useCalendarStore((s) => s.updateEvent);
  const scheduleTask = useCalendarStore((s) => s.scheduleTask);
  const focusEventId = useCalendarStore((s) => s.focusEventId);
  const setFocusEventId = useCalendarStore((s) => s.setFocusEventId);
  const [selection, setSelection] = useState<CalendarSelection>(null);
  const calendarRef = useRef<FullCalendar | null>(null);

  // 通知クリック（§14.3）: 対象予定の日付へ移動して右パネルで選択する
  useEffect(() => {
    if (!focusEventId) return;
    let active = true;
    void getEvent(focusEventId)
      .then((event) => {
        if (!active) return;
        calendarRef.current?.getApi().gotoDate(event.startAtUtc);
        setSelection({ type: "event", id: event.id });
      })
      .catch(() => undefined)
      .finally(() => {
        if (active) setFocusEventId(null);
      });
    return () => {
      active = false;
    };
  }, [focusEventId, setFocusEventId]);

  const applyEventChange = async (info: EventChangeArg) => {
    const { kind, sourceId } = info.event.extendedProps as {
      kind: "event" | "task";
      sourceId: string;
    };
    const allDay = info.event.allDay;
    if (!info.event.startStr) {
      info.revert();
      return;
    }
    const startAtUtc = calStrToUtc(info.event.startStr, allDay);
    if (kind === "task") {
      const ok = await scheduleTask(sourceId, startAtUtc);
      if (!ok) info.revert();
      return;
    }
    const patch: EventPatch = { startAtUtc, allDay };
    if (info.event.endStr) {
      patch.endAtUtc = calStrToUtc(info.event.endStr, allDay);
    }
    const ok = await updateEvent(sourceId, patch);
    if (!ok) info.revert();
  };

  const handleEventClick = (info: EventClickArg) => {
    const { kind, sourceId } = info.event.extendedProps as {
      kind: "event" | "task";
      sourceId: string;
    };
    setSelection({ type: kind, id: sourceId });
  };

  const handleEventReceive = async (info: EventReceiveArg) => {
    const { kind, sourceId } = info.event.extendedProps as {
      kind?: "event" | "task";
      sourceId?: string;
    };
    if (kind !== "task" || !sourceId || !info.event.startStr) {
      info.revert();
      return;
    }
    const ok = await scheduleTask(
      sourceId,
      calStrToUtc(info.event.startStr, info.event.allDay),
    );
    if (!ok) info.revert();
  };

  return (
    <ThreePaneLayout
      left={<UnscheduledTasks tasks={tasks} />}
      right={<EventPanel selection={selection} onClose={() => setSelection(null)} />}
      leftLabel="タスク設定"
      rightLabel="予定詳細"
    >
      <div className="calendar-page">
        {error && (
          <p className="calendar-page__error" role="alert">
            {error}
          </p>
        )}
        <FullCalendar
          ref={calendarRef}
          plugins={[dayGridPlugin, timeGridPlugin, interactionPlugin]}
          initialView="dayGridMonth"
          locale={jaLocale}
          headerToolbar={{
            left: "prev,next today",
            center: "title",
            right: "dayGridMonth,timeGridWeek,timeGridDay",
          }}
          height="100%"
          selectable
          editable
          droppable
          dayMaxEvents
          nowIndicator
          events={buildCalendarInputs(events, tasks, true)}
          datesSet={(info) => {
            void loadRange(info.start.toISOString(), info.end.toISOString());
          }}
          select={(info) => setSelection({ type: "draft", draft: draftFromSelect(info) })}
          eventClick={handleEventClick}
          eventDrop={(info) => void applyEventChange(info)}
          eventResize={(info) => void applyEventChange(info)}
          eventReceive={(info) => void handleEventReceive(info)}
        />
      </div>
    </ThreePaneLayout>
  );
}
