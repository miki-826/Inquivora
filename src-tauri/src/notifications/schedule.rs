use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
pub fn parse_settings(_value: Option<serde_json::Value>) -> NotificationSettings {
    unimplemented!()
}

/// タスク期日の既定通知時刻（UTC RFC3339）。
/// 日付のみ期日はその日の既定通知時刻（Tokyo）、時刻付きはリード分前。
pub fn task_default_notify_at(
    _due_at_utc: &str,
    _settings: &NotificationSettings,
) -> Option<String> {
    unimplemented!()
}

/// 予定開始の既定通知時刻（UTC RFC3339）。
/// 終日予定は初日の既定通知時刻（Tokyo）、時刻付きはリード分前。
pub fn event_default_notify_at(
    _start_at_utc: &str,
    _all_day: bool,
    _settings: &NotificationSettings,
) -> Option<String> {
    unimplemented!()
}

/// 期日・開始日時の変更差分（秒）。どちらかが不正ならNone。
pub fn anchor_delta_seconds(_old_utc: &str, _new_utc: &str) -> Option<i64> {
    unimplemented!()
}

/// tick間隔からの大幅な遅延をスリープ復帰とみなす（§14.5）。
pub fn is_resume_gap(_elapsed_secs: u64) -> bool {
    unimplemented!()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPayload {
    pub notification_id: String,
    pub title: String,
    pub body: String,
    pub launch_uri: String,
}

/// タスク期限通知（§14.3・§14.4）。
pub fn task_notification(
    _reminder_id: &str,
    _task_id: &str,
    _task_title: &str,
    _due_at_utc: Option<&str>,
) -> NotificationPayload {
    unimplemented!()
}

/// 予定開始通知（§14.3・§14.4）。
pub fn event_notification(
    _reminder_id: &str,
    _event_id: &str,
    _event_title: &str,
    _start_at_utc: &str,
    _all_day: bool,
    _now: DateTime<Utc>,
) -> NotificationPayload {
    unimplemented!()
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
