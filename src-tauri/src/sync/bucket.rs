//! Shared wire-compatible time bucket names, independent of the brain merge engine.
use crate::flow_neuro::scoring::TimeBucket;

pub(crate) fn bucket_to_string(b: &TimeBucket) -> String {
    serde_json::to_value(b)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}
pub(crate) fn string_to_bucket(s: &str) -> Option<TimeBucket> {
    if let Ok(bucket) = serde_json::from_value(serde_json::Value::String(s.to_string())) {
        return Some(bucket);
    }
    let normalized: String = s
        .chars()
        .filter(|c| *c != '_')
        .flat_map(char::to_lowercase)
        .collect();
    match normalized.as_str() {
        "weekdaymorning" => Some(TimeBucket::WeekdayMorning),
        "weekdayafternoon" => Some(TimeBucket::WeekdayAfternoon),
        "weekdayevening" => Some(TimeBucket::WeekdayEvening),
        "weekdaynight" => Some(TimeBucket::WeekdayNight),
        "weekendmorning" => Some(TimeBucket::WeekendMorning),
        "weekendafternoon" => Some(TimeBucket::WeekendAfternoon),
        "weekendevening" => Some(TimeBucket::WeekendEvening),
        "weekendnight" => Some(TimeBucket::WeekendNight),
        _ => None,
    }
}

#[must_use]
pub fn canonical_bucket_key(key: &str) -> Option<String> {
    string_to_bucket(key).map(|bucket| bucket_to_string(&bucket))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_android_and_desktop_bucket_names() {
        let canonical = bucket_to_string(&TimeBucket::WeekdayMorning);
        assert_eq!(canonical_bucket_key("weekday_morning"), Some(canonical.clone()));
        assert_eq!(canonical_bucket_key("WeekdayMorning"), Some(canonical));
        assert_eq!(canonical_bucket_key("unknown"), None);
    }
}
