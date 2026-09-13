const test = require("node:test");
const assert = require("node:assert/strict");

const {
  buildOverviewAnalytics,
  calculateMetrics,
  localDateKey,
} = require("./overview-analytics.js");

function session(key, completedAt, units, audioMs, kind = "Dictation") {
  return {
    session_key: key,
    completed_at_epoch_ms: completedAt,
    session_kind: kind,
    inserted_text_units: units,
    audio_duration_ms: audioMs,
  };
}

test("groups multiple sessions on the same local date", () => {
  const now = Date.UTC(2026, 6, 16, 12);
  const result = buildOverviewAnalytics(
    {
      typing_baseline_wpm: 40,
      analytics_sessions: [
        session("a", Date.UTC(2026, 6, 16, 1), 10, 2_000),
        session("b", Date.UTC(2026, 6, 16, 2), 5, 1_000, "SelectedTextEdit"),
      ],
    },
    { nowEpochMs: now, timezoneOffsetMinutes: 0 },
  );

  assert.equal(result.daily.at(-1).insertedTextUnits, 15);
  assert.equal(result.daily.at(-1).audioDurationMs, 3_000);
});

test("uses local calendar dates across UTC boundaries", () => {
  const lateWednesdayUtc = Date.UTC(2026, 6, 15, 18, 30);
  assert.equal(localDateKey(lateWednesdayUtc, 480), "2026-07-16");
  assert.equal(localDateKey(Date.UTC(2026, 6, 16, 2), -420), "2026-07-15");
});

test("separates sessions across local midnight", () => {
  const result = buildOverviewAnalytics(
    {
      analytics_sessions: [
        session("before", Date.UTC(2026, 6, 15, 15, 59), 3, 1_000),
        session("after", Date.UTC(2026, 6, 15, 16, 1), 7, 2_000),
      ],
    },
    {
      nowEpochMs: Date.UTC(2026, 6, 16, 4),
      timezoneOffsetMinutes: 480,
    },
  );

  assert.equal(result.daily.at(-2).insertedTextUnits, 3);
  assert.equal(result.daily.at(-1).insertedTextUnits, 7);
});

test("emits zero values for missing days in chronological order", () => {
  const result = buildOverviewAnalytics(
    { analytics_sessions: [] },
    { nowEpochMs: Date.UTC(2026, 6, 16, 12), timezoneOffsetMinutes: 0 },
  );

  assert.deepEqual(
    result.daily.map((day) => day.dateKey),
    [
      "2026-07-10",
      "2026-07-11",
      "2026-07-12",
      "2026-07-13",
      "2026-07-14",
      "2026-07-15",
      "2026-07-16",
    ],
  );
  assert.ok(result.daily.every((day) => day.insertedTextUnits === 0));
});

test("deduplicates stable session keys", () => {
  const repeated = session("same", Date.UTC(2026, 6, 16, 1), 10, 2_000);
  const result = buildOverviewAnalytics(
    { analytics_sessions: [repeated, repeated] },
    { nowEpochMs: Date.UTC(2026, 6, 16, 12), timezoneOffsetMinutes: 0 },
  );

  assert.equal(result.completedSessions, 1);
  assert.equal(result.totals.insertedTextUnits, 10);
});

test("calculates time saved from recording duration", () => {
  const metrics = calculateMetrics(4, 3_000, 40);
  assert.equal(metrics.estimatedManualTypingMs, 6_000);
  assert.equal(metrics.estimatedTimeSavedMs, 3_000);
  assert.equal(metrics.averageUnitsPerMinute, 80);

  assert.equal(calculateMetrics(1, 3_000, 40).estimatedTimeSavedMs, 0);
  assert.equal(calculateMetrics(1, 0, 40).averageUnitsPerMinute, null);
});

test("selected text edit contributes once to totals and its local-day bucket", () => {
  const completedAt = Date.UTC(2026, 6, 16, 2);
  const edit = session(
    "edit-session",
    completedAt,
    4,
    3_000,
    "SelectedTextEdit",
  );
  const result = buildOverviewAnalytics(
    {
      typing_baseline_wpm: 40,
      analytics_sessions: [edit, edit],
    },
    { nowEpochMs: Date.UTC(2026, 6, 16, 12), timezoneOffsetMinutes: 0 },
  );

  assert.equal(result.completedSessions, 1);
  assert.equal(result.totals.insertedTextUnits, 4);
  assert.equal(result.totals.audioDurationMs, 3_000);
  assert.equal(result.totals.estimatedManualTypingMs, 6_000);
  assert.equal(result.totals.estimatedTimeSavedMs, 3_000);
  assert.equal(result.totals.averageUnitsPerMinute, 80);
  assert.equal(result.daily.at(-1).insertedTextUnits, 4);
  assert.equal(result.daily.at(-1).audioDurationMs, 3_000);
});
