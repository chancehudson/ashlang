use std::time::SystemTime;

use chrono::DateTime;
use chrono::Utc;

/// Return an ISO8601 string representing the current time.
pub fn now() -> String {
    let datetime: DateTime<Utc> = SystemTime::now().into();
    datetime.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
