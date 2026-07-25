-- 周期通知: NULLなら単発、正の分数ならその間隔で繰り返し再スケジュールする
ALTER TABLE reminders ADD COLUMN repeat_interval_minutes INTEGER;
