use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::common;
use crate::data;
use crate::domain;

// ------ ------
//     Init
// ------ ------

pub fn init(data_model: &data::Model, navbar: &mut crate::Navbar) -> Model {
    navbar.title = String::from("Muscles");

    Model {
        interval: common::init_interval(
            &data_model
                .training_sessions
                .values()
                .map(|t| t.date)
                .collect::<Vec<NaiveDate>>(),
            false,
        ),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: common::Interval,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: &Msg, model: &mut Model) {
    match msg {
        Msg::ChangeInterval(first, last) => {
            model.interval.first = *first;
            model.interval.last = *last;
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model, data_model: &data::Model) -> Node<Msg> {
    if (data_model.exercises.is_empty() && data_model.loading_exercises)
        || (data_model.training_sessions.is_empty() && data_model.loading_training_sessions)
    {
        common::view_page_loading()
    } else {
        let training_sessions_interval: common::Interval =
            data_model.training_sessions_date_range().into();
        div![
            common::view_interval_buttons(
                &model.interval,
                &training_sessions_interval,
                Msg::ChangeInterval
            ),
            domain::Muscle::iter().map(|m| {
                let set_volume = data_model
                    .training_stats
                    .stimulus_for_each_muscle_per_week
                    .get(&domain::Muscle::id(*m))
                    .map(|stimulus_per_muscle| {
                        stimulus_per_muscle
                            .iter()
                            .filter(|(date, _)| {
                                *date >= model.interval.first
                                    && *date <= model.interval.last.week(Weekday::Mon).last_day()
                            })
                            .map(
                                #[allow(clippy::cast_precision_loss)]
                                |(date, stimulus)| (*date, *stimulus as f32 / 100.0),
                            )
                            .collect()
                    })
                    .unwrap_or_default();
                div![
                    common::view_title(&span![domain::Muscle::name(*m)], 1),
                    div![
                        C!["block"],
                        C!["is-size-7"],
                        C!["has-text-centered"],
                        domain::Muscle::description(*m)
                    ],
                    common::view_chart(
                        &[("Set volume (weekly total)", common::COLOR_SET_VOLUME)],
                        common::plot_line_chart(
                            &[(set_volume, common::COLOR_SET_VOLUME)],
                            model.interval.first,
                            model.interval.last,
                            Some(0.),
                            Some(10.),
                            data_model.theme()
                        ),
                        true,
                    )
                ]
            })
        ]
    }
}
