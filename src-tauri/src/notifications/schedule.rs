use chrono::{DateTime, Datelike, NaiveTime, SecondsFormat, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use serde::{Deserialize, Serialize};

use crate::database::tasks::is_date_only_due;

/// §19.5 通知設定。app_settingsのキー "notifications" にJSONで保存する。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationSettings {
    pub enabled: bool,
    pub sound: bool,
    /// 日付のみ期日・終日予定の既定通知時刻（Asia/Tokyo、"HH:MM"）
    pub default_notify_time: String,
    /// 時刻付きタスク期日の既定リマインド（分）。Noneなら自動作成しない
    pub task_lead_minutes: Option<i64>,
    /// 予定開始の既定リマインド（分）。Noneなら自動作成しない
    pub event_lead_minutes: Option<i64>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            sound: true,
            default_notify_time: "09:00".to_string(),
            task_lead_minutes: Some(30),
            event_lead_minutes: Some(10),
        }
    }
}

/// 欠落・不正なJSONは既定値で補う。
pub fn parse_settings(value: Option<serde_json::Value>) -> NotificationSettings {
    value
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

fn parse_utc(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn to_rfc3339_z(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn default_time(settings: &NotificationSettings) -> NaiveTime {
    NaiveTime::parse_from_str(&settings.default_notify_time, "%H:%M")
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(9, 0, 0).expect("固定値"))
}

/// 対象日時のTokyo日付における既定通知時刻をUTCで返す。
fn notify_at_default_time(anchor: DateTime<Utc>, settings: &NotificationSettings) -> Option<String> {
    let tokyo = anchor.with_timezone(&Tokyo);
    let date = tokyo.date_naive();
    Tokyo
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .map(|midnight| {
            let at = midnight + chrono::Duration::seconds(
                default_time(settings).signed_duration_since(NaiveTime::MIN).num_seconds(),
            );
            to_rfc3339_z(at.with_timezone(&Utc))
        })
}

/// タスク期日の既定通知時刻（UTC RFC3339）。
/// 日付のみ期日はその日の既定通知時刻（Tokyo）、時刻付きはリード分前。
pub fn task_default_notify_at(
    due_at_utc: &str,
    settings: &NotificationSettings,
) -> Option<String> {
    let lead = settings.task_lead_minutes?;
    let due = parse_utc(due_at_utc)?;
    if is_date_only_due(due_at_utc) {
        notify_at_default_time(due, settings)
    } else {
        Some(to_rfc3339_z(due - chrono::Duration::minutes(lead)))
    }
}

/// 予定開始の既定通知時刻（UTC RFC3339）。
/// 終日予定は初日の既定通知時刻（Tokyo）、時刻付きはリード分前。
pub fn event_default_notify_at(
    start_at_utc: &str,
    all_day: bool,
    settings: &NotificationSettings,
) -> Option<String> {
    let lead = settings.event_lead_minutes?;
    let start = parse_utc(start_at_utc)?;
    if all_day {
        notify_at_default_time(start, settings)
    } else {
        Some(to_rfc3339_z(start - chrono::Duration::minutes(lead)))
    }
}

/// 期日・開始日時の変更差分（秒）。どちらかが不正ならNone。
pub fn anchor_delta_seconds(old_utc: &str, new_utc: &str) -> Option<i64> {
    let old = parse_utc(old_utc)?;
    let new = parse_utc(new_utc)?;
    Some((new - old).num_seconds())
}

/// tick間隔からの大幅な遅延をスリープ復帰とみなす（§14.5）。
pub fn is_resume_gap(elapsed_secs: u64) -> bool {
    elapsed_secs > 90
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    pub notification_id: String,
    pub title: String,
    pub body: String,
    pub launch_uri: String,
}

fn tokyo_date_label(dt: DateTime<chrono_tz::Tz>) -> String {
    format!("{}月{}日", dt.month(), dt.day())
}

fn tokyo_time_label(dt: DateTime<chrono_tz::Tz>) -> String {
    dt.format("%-H:%M").to_string()
}

/// タスク期限通知（§14.3・§14.4）。
pub fn task_notification(
    reminder_id: &str,
    task_id: &str,
    task_title: &str,
    due_at_utc: Option<&str>,
) -> NotificationPayload {
    let body = match due_at_utc.and_then(parse_utc) {
        Some(due) => {
            let tokyo = due.with_timezone(&Tokyo);
            if due_at_utc.is_some_and(is_date_only_due) {
                format!("「{task_title}」の期日は{}です。", tokyo_date_label(tokyo))
            } else {
                format!(
                    "「{task_title}」の期日は{} {}です。",
                    tokyo_date_label(tokyo),
                    tokyo_time_label(tokyo)
                )
            }
        }
        None => format!("「{task_title}」のリマインダーです。"),
    };
    NotificationPayload {
        notification_id: reminder_id.to_string(),
        title: "タスク期限".to_string(),
        body,
        launch_uri: format!("inquivora://open?type=task&id={task_id}"),
    }
}

/// 予定開始通知（§14.3・§14.4）。
pub fn event_notification(
    reminder_id: &str,
    event_id: &str,
    event_title: &str,
    start_at_utc: &str,
    all_day: bool,
    now: DateTime<Utc>,
) -> NotificationPayload {
    let body = match parse_utc(start_at_utc) {
        Some(start) => {
            let tokyo = start.with_timezone(&Tokyo);
            if all_day {
                format!("「{event_title}」は{}の終日予定です。", tokyo_date_label(tokyo))
            } else if tokyo.date_naive() == now.with_timezone(&Tokyo).date_naive() {
                format!("「{event_title}」が{}から始まります。", tokyo_time_label(tokyo))
            } else {
                format!(
                    "「{event_title}」が{} {}から始まります。",
                    tokyo_date_label(tokyo),
                    tokyo_time_label(tokyo)
                )
            }
        }
        None => format!("「{event_title}」のリマインダーです。"),
    };
    NotificationPayload {
        notification_id: reminder_id.to_string(),
        title: "予定開始".to_string(),
        body,
        launch_uri: format!("inquivora://open?type=event&id={event_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 17, 3, 0, 0).unwrap()
    }

    #[test]
    fn 設定が欠落や不正なら既定値になる() {
        assert_eq!(parse_settings(None), NotificationSettings::default());
        assert_eq!(
            parse_settings(Some(serde_json::json!("壊れた値"))),
            NotificationSettings::default()
        );
    }

    #[test]
    fn 設定jsonを解釈できる() {
        let parsed = parse_settings(Some(serde_json::json!({
            "enabled": false,
            "sound": false,
            "defaultNotifyTime": "08:30",
            "taskLeadMinutes": null,
            "eventLeadMinutes": 15
        })));
        assert!(!parsed.enabled);
        assert!(!parsed.sound);
        assert_eq!(parsed.default_notify_time, "08:30");
        assert_eq!(parsed.task_lead_minutes, None);
        assert_eq!(parsed.event_lead_minutes, Some(15));
    }

    #[test]
    fn 時刻付き期日の既定通知はリード分前() {
        let settings = NotificationSettings::default();
        assert_eq!(
            task_default_notify_at("2026-07-18T01:00:00Z", &settings).as_deref(),
            Some("2026-07-18T00:30:00Z")
        );
    }

    #[test]
    fn 日付のみ期日は既定通知時刻になる() {
        // 2026-07-17T15:00:00Z = Tokyo 7/18 00:00（日付のみ期日）→ 7/18 09:00 Tokyo = 00:00 UTC
        let settings = NotificationSettings::default();
        assert_eq!(
            task_default_notify_at("2026-07-17T15:00:00Z", &settings).as_deref(),
            Some("2026-07-18T00:00:00Z")
        );
    }

    #[test]
    fn タスクリードが無効なら通知を作らない() {
        let settings = NotificationSettings {
            task_lead_minutes: None,
            ..NotificationSettings::default()
        };
        assert_eq!(task_default_notify_at("2026-07-18T01:00:00Z", &settings), None);
    }

    #[test]
    fn 予定の既定通知はリード分前() {
        let settings = NotificationSettings::default();
        assert_eq!(
            event_default_notify_at("2026-07-18T01:00:00Z", false, &settings).as_deref(),
            Some("2026-07-18T00:50:00Z")
        );
    }

    #[test]
    fn 終日予定は初日の既定通知時刻になる() {
        let settings = NotificationSettings::default();
        assert_eq!(
            event_default_notify_at("2026-07-17T15:00:00Z", true, &settings).as_deref(),
            Some("2026-07-18T00:00:00Z")
        );
    }

    #[test]
    fn anchor_delta_secondsは差分秒を返す() {
        assert_eq!(
            anchor_delta_seconds("2026-07-18T01:00:00Z", "2026-07-19T02:30:00Z"),
            Some(24 * 3600 + 90 * 60)
        );
        assert_eq!(anchor_delta_seconds("不正", "2026-07-19T02:30:00Z"), None);
    }

    #[test]
    fn 復帰判定は90秒超の遅延だけ真になる() {
        assert!(!is_resume_gap(20));
        assert!(!is_resume_gap(90));
        assert!(is_resume_gap(91));
        assert!(is_resume_gap(3600));
    }

    #[test]
    fn タスク通知のペイロードを組み立てる() {
        let payload = task_notification("r1", "t1", "資料作成", Some("2026-07-18T01:00:00Z"));
        assert_eq!(payload.notification_id, "r1");
        assert_eq!(payload.title, "タスク期限");
        assert_eq!(payload.body, "「資料作成」の期日は7月18日 10:00です。");
        assert_eq!(payload.launch_uri, "inquivora://open?type=task&id=t1");
    }

    #[test]
    fn 日付のみ期日のタスク通知は時刻を出さない() {
        let payload = task_notification("r1", "t1", "資料作成", Some("2026-07-17T15:00:00Z"));
        assert_eq!(payload.body, "「資料作成」の期日は7月18日です。");
    }

    #[test]
    fn 期日なしタスクの通知本文() {
        let payload = task_notification("r1", "t1", "資料作成", None);
        assert_eq!(payload.body, "「資料作成」のリマインダーです。");
    }

    #[test]
    fn 当日の予定通知は時刻だけを出す() {
        // Tokyo 7/17 19:00 開始、現在は Tokyo 7/17 12:00
        let payload = event_notification(
            "r2",
            "e1",
            "DX導入定例会",
            "2026-07-17T10:00:00Z",
            false,
            fixed_now(),
        );
        assert_eq!(payload.title, "予定開始");
        assert_eq!(payload.body, "「DX導入定例会」が19:00から始まります。");
        assert_eq!(payload.launch_uri, "inquivora://open?type=event&id=e1");
    }

    #[test]
    fn 別日の予定通知は日付も出す() {
        let payload = event_notification(
            "r2",
            "e1",
            "DX導入定例会",
            "2026-07-18T01:00:00Z",
            false,
            fixed_now(),
        );
        assert_eq!(payload.body, "「DX導入定例会」が7月18日 10:00から始まります。");
    }

    #[test]
    fn 終日予定の通知本文() {
        let payload = event_notification(
            "r2",
            "e1",
            "全社休暇",
            "2026-07-17T15:00:00Z",
            true,
            fixed_now(),
        );
        assert_eq!(payload.body, "「全社休暇」は7月18日の終日予定です。");
    }
}
