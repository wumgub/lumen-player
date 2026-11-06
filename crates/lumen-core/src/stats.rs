//! The Stats view's numbers, worked out from the raw play log the same way
//! StatsView.tsx does: a most-played leaderboard and the last 12 weeks.

use std::collections::HashMap;

use chrono::{Datelike, Duration, Local, LocalResult, NaiveDate, TimeZone};

use crate::models::PlayEvent;

pub const WEEKS: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopRow {
    pub track_id: i64,
    pub plays: i64,
    pub seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Week {
    /// Monday 00:00 local time, epoch ms.
    pub start: i64,
    pub seconds: i64,
    pub plays: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Summary {
    /// Up to 30 tracks, most plays first (then most time).
    pub top: Vec<TopRow>,
    /// Oldest first; the last one is this week.
    pub weeks: Vec<Week>,
    pub total_seconds: i64,
    pub total_plays: i64,
}

fn local_midnight(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).expect("midnight exists");
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(t) => t.timestamp_millis(),
        LocalResult::Ambiguous(a, _) => a.timestamp_millis(),
        // Midnight skipped by a DST jump: the day starts at the first valid time.
        LocalResult::None => Local
            .from_local_datetime(&(naive + Duration::hours(1)))
            .earliest()
            .map(|t| t.timestamp_millis())
            .unwrap_or(0),
    }
}

/// The Monday of the week containing `ms`, as a local date.
fn monday_of(ms: i64) -> NaiveDate {
    let d = Local.timestamp_millis_opt(ms).earliest().map(|t| t.date_naive()).unwrap_or_default();
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// The local (month, day) of an epoch-ms time, for chart labels.
pub fn local_month_day(ms: i64) -> (u32, u32) {
    let d = Local.timestamp_millis_opt(ms).earliest().map(|t| t.date_naive()).unwrap_or_default();
    (d.month(), d.day())
}

/// Monday 00:00 (local) of the week containing `ms`.
pub fn week_start(ms: i64) -> i64 {
    local_midnight(monday_of(ms))
}

pub fn summarise(history: &[PlayEvent], known: impl Fn(i64) -> bool, now_ms: i64) -> Summary {
    let mut per_track: HashMap<i64, (i64, i64)> = HashMap::new();
    let (mut total_seconds, mut total_plays) = (0, 0);
    for e in history {
        total_seconds += e.seconds;
        total_plays += e.counted as i64;
        let a = per_track.entry(e.track_id).or_default();
        a.0 += e.counted as i64;
        a.1 += e.seconds;
    }
    let mut top: Vec<TopRow> = per_track
        .into_iter()
        .filter(|(id, (plays, _))| *plays > 0 && known(*id))
        .map(|(track_id, (plays, seconds))| TopRow { track_id, plays, seconds })
        .collect();
    top.sort_by(|x, y| y.plays.cmp(&x.plays).then(y.seconds.cmp(&x.seconds)).then(x.track_id.cmp(&y.track_id)));
    top.truncate(30);

    // Step by calendar weeks, not fixed milliseconds, so a DST change can't
    // push a bucket onto the wrong Monday.
    let this_monday = monday_of(now_ms);
    let mut weeks: Vec<Week> = (0..WEEKS)
        .rev()
        .map(|i| Week {
            start: local_midnight(this_monday - Duration::weeks(i as i64)),
            seconds: 0,
            plays: 0,
        })
        .collect();
    let index: HashMap<i64, usize> = weeks.iter().enumerate().map(|(i, w)| (w.start, i)).collect();
    for e in history {
        if let Some(&i) = index.get(&week_start(e.played_at)) {
            weeks[i].seconds += e.seconds;
            weeks[i].plays += e.counted as i64;
        }
    }
    Summary { top, weeks, total_seconds, total_plays }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(track_id: i64, played_at: i64, seconds: i64, counted: bool) -> PlayEvent {
        PlayEvent { track_id, played_at, seconds, counted }
    }

    #[test]
    fn leaderboard_and_weeks() {
        let now = Local::now().timestamp_millis();
        let week = 7 * 24 * 3600 * 1000;
        let h = vec![
            ev(1, now - 1000, 200, true),
            ev(1, now - 2000, 30, false),
            ev(2, now - 3000, 240, true),
            ev(2, now - 4000, 240, true),
            ev(3, now - 20 * week, 100, true), // too old for the weekly chart
            ev(9, now - 5000, 300, true),      // a track that's gone from the library
        ];
        let s = summarise(&h, |id| id != 9, now);
        assert_eq!(s.top.iter().map(|r| r.track_id).collect::<Vec<_>>(), vec![2, 1, 3]);
        assert_eq!(s.top[1], TopRow { track_id: 1, plays: 1, seconds: 230 });
        assert_eq!(s.total_plays, 5);
        assert_eq!(s.weeks.len(), WEEKS);
        let this_week = s.weeks.last().unwrap();
        assert_eq!(this_week.plays, 4);
        assert_eq!(this_week.seconds, 200 + 30 + 240 + 240 + 300);
        assert_eq!(this_week.start, week_start(now));
        assert!(s.weeks.windows(2).all(|w| w[0].start < w[1].start));
    }
}
