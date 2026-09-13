(function initializeOverviewAnalytics(globalScope) {
  const DEFAULT_TYPING_BASELINE_UNITS_PER_MINUTE = 40;
  const DAY_MS = 86_400_000;

  function datePartsForEpoch(epochMs, timezoneOffsetMinutes) {
    const date = new Date(
      timezoneOffsetMinutes === undefined
        ? epochMs
        : epochMs + timezoneOffsetMinutes * 60_000,
    );
    return timezoneOffsetMinutes === undefined
      ? [date.getFullYear(), date.getMonth() + 1, date.getDate()]
      : [date.getUTCFullYear(), date.getUTCMonth() + 1, date.getUTCDate()];
  }

  function dateKeyFromParts(year, month, day) {
    return `${year.toString().padStart(4, "0")}-${month
      .toString()
      .padStart(2, "0")}-${day.toString().padStart(2, "0")}`;
  }

  function localDateKey(epochMs, timezoneOffsetMinutes) {
    return dateKeyFromParts(...datePartsForEpoch(epochMs, timezoneOffsetMinutes));
  }

  function calculateMetrics(insertedTextUnits, audioDurationMs, baseline) {
    const estimatedManualTypingMs =
      baseline > 0 ? (insertedTextUnits * 60_000) / baseline : 0;
    return {
      insertedTextUnits,
      audioDurationMs,
      estimatedManualTypingMs,
      estimatedTimeSavedMs: Math.max(estimatedManualTypingMs - audioDurationMs, 0),
      averageUnitsPerMinute:
        audioDurationMs > 0 ? (insertedTextUnits * 60_000) / audioDurationMs : null,
    };
  }

  function buildOverviewAnalytics(usage, options = {}) {
    const baseline =
      Number(usage?.typing_baseline_wpm) ||
      DEFAULT_TYPING_BASELINE_UNITS_PER_MINUTE;
    const nowEpochMs = options.nowEpochMs ?? Date.now();
    const timezoneOffsetMinutes = options.timezoneOffsetMinutes;
    const sessions = Array.isArray(usage?.analytics_sessions)
      ? usage.analytics_sessions
      : [];
    const seenSessionKeys = new Set();
    const includedSessions = sessions.filter((session) => {
      if (
        !session?.session_key ||
        seenSessionKeys.has(session.session_key) ||
        !Number.isFinite(Number(session.completed_at_epoch_ms))
      ) {
        return false;
      }
      seenSessionKeys.add(session.session_key);
      return true;
    });

    const todayParts = datePartsForEpoch(nowEpochMs, timezoneOffsetMinutes);
    const todayOrdinal = Date.UTC(
      todayParts[0],
      todayParts[1] - 1,
      todayParts[2],
    );
    const daily = Array.from({ length: 7 }, (_, index) => {
      const ordinal = todayOrdinal - (6 - index) * DAY_MS;
      const date = new Date(ordinal);
      return {
        dateKey: dateKeyFromParts(
          date.getUTCFullYear(),
          date.getUTCMonth() + 1,
          date.getUTCDate(),
        ),
        weekdayEpochMs: ordinal,
        insertedTextUnits: 0,
        audioDurationMs: 0,
      };
    });
    const dailyByKey = new Map(daily.map((day) => [day.dateKey, day]));

    let totalInsertedTextUnits = 0;
    let totalAudioDurationMs = 0;
    includedSessions.forEach((session) => {
      const insertedTextUnits = Math.max(0, Number(session.inserted_text_units) || 0);
      const audioDurationMs = Math.max(0, Number(session.audio_duration_ms) || 0);
      totalInsertedTextUnits += insertedTextUnits;
      totalAudioDurationMs += audioDurationMs;

      const day = dailyByKey.get(
        localDateKey(Number(session.completed_at_epoch_ms), timezoneOffsetMinutes),
      );
      if (day) {
        day.insertedTextUnits += insertedTextUnits;
        day.audioDurationMs += audioDurationMs;
      }
    });

    return {
      baseline,
      completedSessions: includedSessions.length,
      totals: calculateMetrics(
        totalInsertedTextUnits,
        totalAudioDurationMs,
        baseline,
      ),
      daily: daily.map((day) => ({
        ...day,
        ...calculateMetrics(day.insertedTextUnits, day.audioDurationMs, baseline),
      })),
    };
  }

  const api = {
    buildOverviewAnalytics,
    calculateMetrics,
    localDateKey,
  };
  globalScope.VoiceFlowOverviewAnalytics = api;
  if (typeof module !== "undefined" && module.exports) {
    module.exports = api;
  }
})(typeof window === "undefined" ? globalThis : window);
