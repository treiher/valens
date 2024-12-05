use chrono::prelude::*;
use seed::{prelude::*, *};

use crate::{
    domain,
    ui::{self, common, data},
};

// ------ ------
//     Init
// ------ ------

pub fn init(data_model: &data::Model, navbar: &mut ui::Navbar) -> Model {
    navbar.title = String::from("Muscles");

    Model {
        interval: common::init_interval(
            &data_model
                .training_sessions
                .values()
                .map(|t| t.date)
                .collect::<Vec<NaiveDate>>(),
            common::DefaultInterval::_1M,
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
                #[allow(clippy::cast_precision_loss)]
                let total_7day_set_volume = common::centered_moving_total(
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
                        &[("Set volume (7 day total)", common::COLOR_SET_VOLUME)],
                        common::plot_chart(
                            &[common::PlotData {
                                values: total_7day_set_volume,
                                plots: common::plot_line(common::COLOR_SET_VOLUME),
                                params: common::PlotParams::primary_range(0., 10.),
                            }],
                            model.interval.first,
                            model.interval.last,
                            data_model.theme()
                        ),
                        true,
                    )
                ]
            })
        ]
    }
}
