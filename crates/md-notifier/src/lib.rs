//! Polling decisions independent of the desktop runtime. Checkpoints commit only
//! after the complete response has been persisted successfully.
use md_moodle::PopupNotification;

pub const DEFAULT_POLL_SECONDS: u64 = 120;
pub const MAX_BACKOFF_SECONDS: u64 = 900;

pub struct Batch<'a> {
    pub checkpoint: i64,
    pub alerts: Vec<&'a PopupNotification>,
}

/// None is a fresh account: seed silently. A count increase is not required;
/// reading an old item while a new one arrives can leave the total unchanged.
pub fn batch(previous: Option<i64>, items: &[PopupNotification]) -> Batch<'_> {
    let checkpoint = items
        .iter()
        .map(|item| item.id)
        .max()
        .unwrap_or(0)
        .max(previous.unwrap_or(0));
    let mut alerts: Vec<_> = items
        .iter()
        .filter(|item| previous.is_some_and(|id| item.id > id) && !item.read)
        .collect();
    alerts.sort_by_key(|item| item.id);
    alerts.dedup_by_key(|item| item.id);
    Batch { checkpoint, alerts }
}

pub fn retry_seconds(failures: u32) -> u64 {
    DEFAULT_POLL_SECONDS
        .saturating_mul(1_u64 << failures.min(4))
        .min(MAX_BACKOFF_SECONDS)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(id: i64, read: bool) -> PopupNotification {
        PopupNotification {
            id,
            subject: "Notice".into(),
            contexturl: None,
            timecreated: 1,
            read,
        }
    }
    #[test]
    fn seed_restart_and_equal_count_arrivals() {
        let items = vec![item(8, false), item(7, false)];
        assert!(batch(None, &items).alerts.is_empty());
        assert_eq!(batch(None, &items).checkpoint, 8);
        assert!(batch(Some(8), &items).alerts.is_empty());
        let next = vec![item(9, false), item(8, true), item(9, false)];
        assert_eq!(batch(Some(8), &next).alerts.len(), 1);
        assert_eq!(batch(Some(9), &[]).checkpoint, 9);
        assert!(batch(Some(9), &[item(10, true)]).alerts.is_empty());
    }
    #[test]
    fn backoff_is_bounded_and_resets() {
        assert_eq!(retry_seconds(0), 120);
        assert_eq!(retry_seconds(1), 240);
        assert_eq!(retry_seconds(3), 900);
        assert_eq!(retry_seconds(u32::MAX), 900);
    }
}
