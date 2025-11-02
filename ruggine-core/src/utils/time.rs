use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// Restituisce l'istante corrente in UTC formattato come RFC3339 (es. "2025-11-02T12:34:56Z").
pub fn now_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    now.format(&Rfc3339).expect("error formatting timestamp")
}
