use chrono::prelude::*;
use seed::{prelude::*, *};
use valens_domain as domain;
use valens_web_app as web_app;

use crate::{common, data};

// ------ ------
//     Init
// ------ ------

pub fn init(data_model: &data::Model, navbar: &mut crate::Navbar) -> Model {
    navbar.title = String::from("Muscles");

    Model {
        interval: domain::init_interval(
            &data_model
                .training_sessions
                .values()
                .map(|t| t.date)
                .collect::<Vec<NaiveDate>>(),
            domain::DefaultInterval::_1M,
        ),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    interval: domain::Interval,
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
        let training_sessions_interval: domain::Interval =
            data_model.training_sessions_date_range().into();
        div![
            common::view_interval_buttons(
                &model.interval,
                &training_sessions_interval,
                Msg::ChangeInterval
            ),
            domain::Muscle::iter().map(|m| {
                #[allow(clippy::cast_precision_loss)]
                let total_7day_set_volume = domain::centered_moving_total(
                    &data_model
                        .training_sessions
                        .values()
                        .filter_map(|s| {
                            s.stimulus_per_muscle(&data_model.exercises)
                                .get(&m.id())
                                .map(|stimulus| (s.date, *stimulus as f32 / 100.))
                        })
                        .collect::<Vec<_>>(),
                    &model.interval,
                    3,
                );

                div![
                    common::view_title(&span![m.name()], 1),
                    div![
                        C!["block"],
                        C!["is-size-7"],
                        C!["has-text-centered"],
                        m.description()
                    ],
                    common::view_chart(
                        &[(
                            "Set volume (7 day total)",
                            web_app::chart::COLOR_SET_VOLUME,
                            web_app::chart::OPACITY_LINE
                        )],
                        web_app::chart::plot(
                            &[web_app::chart::PlotData {
                                values_high: total_7day_set_volume,
                                values_low: None,
                                plots: web_app::chart::plot_area_with_border(
                                    web_app::chart::COLOR_SET_VOLUME,
                                    web_app::chart::COLOR_SET_VOLUME,
                                ),
                                params: web_app::chart::PlotParams::primary_range(0., 10.),
                            }],
                            &model.interval,
                            data_model.theme()
                        ),
                        true,
                    )
                ]
            })
        ]
    }
}
