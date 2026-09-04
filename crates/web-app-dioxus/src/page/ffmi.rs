use chrono::NaiveDate;
use dioxus::prelude::*;

use valens_domain::{self as domain, BodyWeightService};
use valens_web_app as web_app;

use crate::{
    DOMAIN_SERVICE,
    cache::{Cache, CacheState},
    chart::{Calendar, Chart, IntervalControl},
    session::Session,
    ui::element::{ErrorPage, InfoPage, LoadingPage, NoWrap, Table},
};

#[component]
pub fn Ffmi() -> Element {
    let cache = consume_context::<Cache>();
    let session = consume_context::<Session>();
    let sex = use_memo(move || session.user().sex);
    let height = use_memo(move || session.user().height);

    let ffmi = use_memo(move || {
        if let (Some(height), CacheState::Ready(body_fat), CacheState::Ready(body_weight)) = (
            height(),
            &*cache.body_fat.read(),
            &*cache.body_weight.read(),
        ) {
            let avg_body_weight = DOMAIN_SERVICE().avg_body_weight(body_weight);
            domain::ffmi(&avg_body_weight, body_fat, sex(), height)
        } else {
            vec![]
        }
    });
    let dates = use_memo(move || {
        ffmi.read()
            .iter()
            .map(|(date, _)| *date)
            .collect::<Vec<_>>()
    });
    let current_interval =
        use_signal(|| domain::init_interval(&dates.read(), domain::DefaultInterval::_3M));
    let all = *use_memo(move || domain::Interval {
        first: dates.read().iter().min().copied().unwrap_or_default(),
        last: dates.read().iter().max().copied().unwrap_or_default(),
    })
    .read();

    if height().is_none() {
        return rsx! {
            InfoPage {
                "data-testid": "ffmi-height-missing",
                "Set your height in the profile to calculate your FFMI."
            }
        };
    }

    match (&*cache.body_fat.read(), &*cache.body_weight.read()) {
        (CacheState::Ready(_), CacheState::Ready(_)) => {
            let ffmi = ffmi.read();
            rsx! {
                IntervalControl { current_interval, all },
                {chart(&ffmi, *current_interval.read())},
                {calendar(&ffmi, *current_interval.read())},
                {table(&ffmi, *current_interval.read())},
            }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { ErrorPage { message: "{err}" } }
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => {
            rsx! { LoadingPage {} }
        }
    }
}

fn chart(ffmi: &[(NaiveDate, f32)], interval: domain::Interval) -> Element {
    let values = ffmi
        .iter()
        .filter(|(date, _)| *date >= interval.first && *date <= interval.last)
        .copied()
        .collect::<Vec<_>>();
    let data = web_app::chart::PlotData {
        values_high: values,
        values_low: None,
        plots: web_app::chart::plot_line(web_app::chart::COLOR_FFMI),
        params: web_app::chart::PlotParams::default(),
    };
    rsx! {
        Chart {
            series: vec![web_app::chart::LabeledSeries::new("Normalized FFMI (kg/m²)", data)],
            interval,
            no_data_label: true,
        }
    }
}

fn calendar(ffmi: &[(NaiveDate, f32)], interval: domain::Interval) -> Element {
    let points = ffmi
        .iter()
        .filter(|(date, _)| (interval.first..=interval.last).contains(date))
        .copied()
        .collect::<Vec<_>>();
    let values = points.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    let min = values
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);
    let max = values
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(1.);
    let entries = points
        .iter()
        .map(|(date, value)| {
            (
                *date,
                web_app::chart::COLOR_FFMI,
                if max > min {
                    f64::from((value - min) / (max - min)) * 0.8 + 0.2
                } else {
                    1.0
                },
            )
        })
        .collect();

    rsx! {
        Calendar { entries, interval }
    }
}

fn table(ffmi: &[(NaiveDate, f32)], interval: domain::Interval) -> Element {
    let head = vec![rsx! { "Date" }, rsx! { "Normalized FFMI" }];

    let body = ffmi
        .iter()
        .rev()
        .filter(|(date, _)| *date >= interval.first && *date <= interval.last)
        .map(|(date, value)| vec![rsx! { NoWrap { "{date}" } }, rsx! { "{value:.1}" }])
        .collect::<Vec<_>>();

    rsx! {
        Table { head, body }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{
        TestCache, contains, provide_session, provide_settings, render, rows_of, text_of,
    };

    use super::*;

    fn user(height: Option<u8>) -> domain::User {
        domain::User {
            id: 1.into(),
            name: domain::Name::new("Alice").unwrap(),
            sex: domain::Sex::FEMALE,
            height,
            role: domain::Role::USER,
        }
    }

    fn render_ffmi(user: domain::User, cache: impl Fn() -> TestCache + 'static) -> String {
        render(move || {
            cache().provide();
            provide_session(user.clone());
            provide_settings(web_app::Settings::default());
            rsx! { Ffmi {} }
        })
    }

    fn today() -> NaiveDate {
        chrono::Local::now().date_naive()
    }

    fn measurements() -> TestCache {
        TestCache::default()
            .with_body_weight(vec![domain::BodyWeight {
                date: today(),
                weight: 60.0,
            }])
            .with_body_fat(vec![domain::BodyFat {
                date: today(),
                chest: Some(10),
                abdominal: Some(10),
                thigh: Some(10),
                tricep: Some(10),
                subscapular: Some(10),
                suprailiac: Some(10),
                midaxillary: Some(10),
            }])
    }

    #[test]
    fn test_without_a_height_the_profile_is_pointed_at() {
        let html = render_ffmi(user(None), TestCache::default);

        assert!(contains(&html, "ffmi-height-missing"));
        assert!(!contains(&html, "chart"));
    }

    #[test]
    fn test_the_ffmi_of_every_measurement_is_tabulated() {
        let html = render_ffmi(user(Some(170)), measurements);

        assert_eq!(rows_of(&html, "table")[1][0], today().to_string());
    }

    #[test]
    fn test_the_chart_and_its_legend_are_shown() {
        let html = render_ffmi(user(Some(170)), measurements);

        assert!(contains(&html, "chart"));
        assert!(html.contains("Normalized FFMI (kg/m²)"), "{html}");
    }

    #[test]
    fn test_without_measurements_no_data_is_reported() {
        let html = render_ffmi(user(Some(170)), TestCache::default);

        assert!(!contains(&html, "chart"));
        assert_eq!(rows_of(&html, "table").len(), 1);
    }

    #[test]
    fn test_the_interval_can_be_chosen() {
        let html = render_ffmi(user(Some(170)), measurements);

        assert!(contains(&html, "interval-3M"));
        assert!(contains(&html, "interval-ALL"));
    }

    #[test]
    fn test_unread_measurements_are_shown_as_loading() {
        let html = render_ffmi(user(Some(170)), TestCache::loading);

        assert!(contains(&html, "loading-page"));
    }

    #[test]
    fn test_unreadable_measurements_are_shown_as_an_error() {
        let html = render_ffmi(user(Some(170)), TestCache::failing);

        assert_eq!(text_of(&html, "error-page"), "No connection");
    }
}
