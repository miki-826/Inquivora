import type { DateSelectArg, EventApi, EventClickArg } from "@fullcalendar/core";
import jaLocale from "@fullcalendar/core/locales/ja";
import dayGridPlugin from "@fullcalendar/daygrid";
import interactionPlugin from "@fullcalendar/interaction";
import FullCalendar from "@fullcalendar/react";
import timeGridPlugin from "@fullcalendar/timegrid";
import { formatInTimeZone, fromZonedTime } from "date-fns-tz";
import { useState } from "react";
import { ThreePaneLayout } from "../../components/layout/ThreePaneLayout";
import type { EventPatch } from "../../services/events";
import { updateTask } from "../../services/tasks";
import { useCalendarStore } from "../../stores/useCalendarStore";
import { TOKYO_TZ } from "../tasks/taskModel";
import { buildCalendarInputs, shiftDateString } from "./calendarModel";
import { EventPanel, type CalendarSelection, type EventDraft } from "./EventPanel";

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

export function CalendarPage() {
  const events = useCalendarStore((s) => s.events);
  const tasks = useCalendarStore((s) => s.tasks);
  const error = useCalendarStore((s) => s.error);
  const loadRange = useCalendarStore((s) => s.loadRange);
  const reload = useCalendarStore((s) => s.reload);
  const updateEvent = useCalendarStore((s) => s.updateEvent);
  const [selection, setSelection] = useState<CalendarSelection>(null);

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
      try {
        await updateTask(sourceId, { dueAtUtc: startAtUtc });
        await reload();
      } catch {
        info.revert();
      }
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

  return (
    <ThreePaneLayout right={<EventPanel selection={selection} onClose={() => setSelection(null)} />}>
      <div className="calendar-page">
        {error && (
          <p className="calendar-page__error" role="alert">
            {error}
          </p>
        )}
        <FullCalendar
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
        />
      </div>
    </ThreePaneLayout>
  );
}
