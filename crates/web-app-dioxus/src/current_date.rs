use chrono::{Duration, Local, NaiveDate, NaiveTime};
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

static CURRENT_DATE: GlobalSignal<NaiveDate> = Signal::global(|| Local::now().date_naive());

/// The current local date. Reactive contexts reading it are updated when the date changes.
pub fn current_date() -> NaiveDate {
    CURRENT_DATE()
}

/// Updates [`current_date`] shortly after each midnight.
pub async fn update_at_midnight() {
    loop {
        TimeoutFuture::new(millis_until_after_midnight()).await;
        let today = Local::now().date_naive();
        // The timeout may fire before midnight when a DST transition shortens the local day
        if *CURRENT_DATE.peek() != today {
            *CURRENT_DATE.write() = today;
        }
    }
}

fn millis_until_after_midnight() -> u32 {
    let now = Local::now().naive_local();
    let midnight = (now.date() + Duration::days(1)).and_time(NaiveTime::MIN);
    u32::try_from((midnight - now).num_milliseconds())
        .unwrap_or(0)
        .saturating_add(1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_millis_until_after_midnight_within_one_day() {
        let millis = millis_until_after_midnight();
        assert!(millis > 0);
        assert!(millis <= 86_401_000);
    }
}
