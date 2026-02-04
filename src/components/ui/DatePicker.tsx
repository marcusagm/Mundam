import { Component, createMemo, createSignal, For, splitProps } from "solid-js";
import { ChevronLeft, ChevronRight } from "lucide-solid";
import { cn } from "../../lib/utils";
import "./date-picker.css";

export interface DatePickerProps {
  value?: Date;
  onChange?: (date: Date) => void;
  minDate?: Date;
  maxDate?: Date;
  class?: string;
}

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December"
];

const WEEKDAYS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

export const DatePicker: Component<DatePickerProps> = (props) => {
  const [local, others] = splitProps(props, ["value", "onChange", "minDate", "maxDate", "class"]);

  const now = new Date();
  // View state: "day" | "month" | "year"
  const [view, setView] = createSignal<"day" | "month" | "year">("day");
  // Default to current date or value
  const [viewDate, setViewDate] = createSignal(local.value ? new Date(local.value) : now);

  // --- Day View Logic ---
  const getDaysInMonth = (year: number, month: number) => new Date(year, month + 1, 0).getDate();
  const getFirstDayOfMonth = (year: number, month: number) => new Date(year, month, 1).getDay();

  const days = createMemo(() => {
    const year = viewDate().getFullYear();
    const month = viewDate().getMonth();
    const daysInMonth = getDaysInMonth(year, month);
    const firstDay = getFirstDayOfMonth(year, month);
    // Padding days for previous month
    const padding = Array(firstDay).fill(null);
    // Actual days
    const daysArray = Array.from({ length: daysInMonth }, (_, i) => i + 1);
    return [...padding, ...daysArray];
  });

  const isSelected = (day: number) => {
    if (!local.value) return false;
    const year = viewDate().getFullYear();
    const month = viewDate().getMonth();
    return (
      local.value.getDate() === day &&
      local.value.getMonth() === month &&
      local.value.getFullYear() === year
    );
  };

  const isToday = (day: number) => {
    const year = viewDate().getFullYear();
    const month = viewDate().getMonth();
    return (
      day === now.getDate() &&
      month === now.getMonth() &&
      year === now.getFullYear()
    );
  };

  const handleDayClick = (day: number) => {
    const year = viewDate().getFullYear();
    const month = viewDate().getMonth();
    const newDate = new Date(year, month, day);
    local.onChange?.(newDate);
  };

  // --- Month View Logic ---
  const handleMonthClick = (monthIndex: number) => {
    const d = new Date(viewDate());
    d.setMonth(monthIndex);
    setViewDate(d);
    setView("day");
  };

  // --- Year View Logic ---
  // 12 year grid centered(ish) around current view year
  const startYear = createMemo(() => {
      return Math.floor(viewDate().getFullYear() / 12) * 12;
  });
  
  const years = createMemo(() => {
      const start = startYear();
      return Array.from({ length: 12 }, (_, i) => start + i);
  });

  const handleYearClick = (year: number) => {
    const d = new Date(viewDate());
    d.setFullYear(year);
    setViewDate(d);
    setView("month");
  };

  // --- Navigation ---
  const handlePrev = () => {
      const d = new Date(viewDate());
      if (view() === "day") {
          d.setMonth(d.getMonth() - 1);
      } else if (view() === "month") {
          d.setFullYear(d.getFullYear() - 1);
      } else {
          d.setFullYear(d.getFullYear() - 12);
      }
      setViewDate(d);
  };

  const handleNext = () => {
      const d = new Date(viewDate());
      if (view() === "day") {
          d.setMonth(d.getMonth() + 1);
      } else if (view() === "month") {
          d.setFullYear(d.getFullYear() + 1);
      } else {
          d.setFullYear(d.getFullYear() + 12);
      }
      setViewDate(d);
  };

  const handleTitleClick = () => {
      if (view() === "day") setView("month");
      else if (view() === "month") setView("year");
  };

  const getTitle = () => {
      if (view() === "day") return `${MONTH_NAMES[viewDate().getMonth()]} ${viewDate().getFullYear()}`;
      if (view() === "month") return `${viewDate().getFullYear()}`;
      return `${startYear()} - ${startYear() + 11}`;
  };

  return (
    <div 
        class={cn("ui-date-picker", local.class)} 
        onClick={(e) => e.stopPropagation()}
        {...others}
    >
      <div class="ui-date-picker-header">
        <button type="button" class="ui-date-picker-nav" onClick={handlePrev}>
          <ChevronLeft size={16} />
        </button>
        <button type="button" class="ui-date-picker-title-btn" onClick={handleTitleClick}>
          {getTitle()}
        </button>
        <button type="button" class="ui-date-picker-nav" onClick={handleNext}>
          <ChevronRight size={16} />
        </button>
      </div>
      
      {/* Day View */}
      {view() === "day" && (
        <div class="ui-date-picker-grid ui-date-picker-day-grid">
            <For each={WEEKDAYS}>
            {(day) => <div class="ui-date-picker-weekday">{day}</div>}
            </For>
            <For each={days()}>
            {(day) => (
                <button
                type="button"
                class={cn(
                    "ui-date-picker-cell",
                    !day && "ui-date-picker-empty",
                    day && isSelected(day) && "ui-date-picker-selected",
                    day && isToday(day) && !isSelected(day) && "ui-date-picker-today"
                )}
                disabled={!day}
                onClick={() => day && handleDayClick(day)}
                >
                {day}
                </button>
            )}
            </For>
        </div>
      )}

      {/* Month View */}
      {view() === "month" && (
          <div class="ui-date-picker-grid ui-date-picker-month-grid">
              <For each={MONTH_NAMES}>
                  {(month, i) => (
                      <button
                        type="button"
                        class={cn(
                            "ui-date-picker-cell ui-date-picker-month-cell",
                            viewDate().getMonth() === i() && "ui-date-picker-selected"
                        )}
                        onClick={() => handleMonthClick(i())}
                      >
                          {month.substring(0, 3)}
                      </button>
                  )}
              </For>
          </div>
      )}

      {/* Year View */}
      {view() === "year" && (
          <div class="ui-date-picker-grid ui-date-picker-year-grid">
              <For each={years()}>
                  {(year) => (
                      <button
                        type="button"
                        class={cn(
                            "ui-date-picker-cell ui-date-picker-year-cell",
                            viewDate().getFullYear() === year && "ui-date-picker-selected"
                        )}
                        onClick={() => handleYearClick(year)}
                      >
                          {year}
                      </button>
                  )}
              </For>
          </div>
      )}
    </div>
  );
};
