use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();

    if let Some(id) = url.next_hash_path_part() {
        let id = id.parse::<u32>().unwrap_or(0);
        orders.send_msg(Msg::FetchExercise(id));
    }

    let local = Local::now().date().naive_local();

    Model {
        base_url,
        interval: common::Interval {
            first: local - Duration::days(30),
            last: local,
        },
        exercise: crate::page::exercises::Exercise {
            id: 0,
            name: String::new(),
        },
        errors: Vec::new(),
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    interval: common::Interval,
    exercise: crate::page::exercises::Exercise,
    errors: Vec<String>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    CloseErrorDialog,

    FetchExercise(u32),
    ExerciseFetched(Result<crate::page::exercises::Exercise, String>),

    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::CloseErrorDialog => {
            model.errors.remove(0);
        }

        Msg::FetchExercise(id) => {
            orders.skip().perform_cmd(async move {
                common::fetch(format!("api/exercises/{}", id), Msg::ExerciseFetched).await
            });
        }
        Msg::ExerciseFetched(Ok(exercise)) => {
            model.exercise = exercise;
        }
        Msg::ExerciseFetched(Err(message)) => {
            model
                .errors
                .push("Failed to fetch exercise: ".to_owned() + &message);
        }

        Msg::ChangeInterval(first, last) => {
            model.interval.first = first;
            model.interval.last = last;
        }
    }
}

// ------ ------
//     View
// ------ ------

pub fn view(model: &Model) -> Node<Msg> {
    div![
        common::view_error_dialog(&model.errors, &ev(Ev::Click, |_| Msg::CloseErrorDialog)),
        common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
        common::view_diagram(
            &model.base_url,
            &format!("exercise/{}", model.exercise.id),
            &model.interval,
            &0
        )
    ]
}
