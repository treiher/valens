use chrono::{prelude::*, Duration};
use seed::{prelude::*, *};

use crate::common;

// ------ ------
//     Init
// ------ ------

pub fn init(mut url: Url, _orders: &mut impl Orders<Msg>) -> Model {
    let base_url = url.to_hash_base_url();
    let exercise_id = if let Some(id) = url.next_hash_path_part() {
        id.parse::<u32>().ok()
    } else {
        None
    };
    let today = Local::now().date().naive_local();

    Model {
        base_url,
        interval: common::Interval {
            first: today - Duration::days(30),
            last: today,
        },
        exercise_id,
    }
}

// ------ ------
//     Model
// ------ ------

pub struct Model {
    base_url: Url,
    interval: common::Interval,
    exercise_id: Option<u32>,
}

// ------ ------
//    Update
// ------ ------

pub enum Msg {
    ChangeInterval(NaiveDate, NaiveDate),
}

pub fn update(msg: Msg, model: &mut Model, _orders: &mut impl Orders<Msg>) {
    match msg {
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
    if let Some(exercise_id) = model.exercise_id {
        div![
            common::view_interval_buttons(&model.interval, Msg::ChangeInterval),
            common::view_diagram(
                &model.base_url,
                &format!("exercise/{}", exercise_id),
                &model.interval,
                &0
            )
        ]
    } else {
        empty![]
    }
}
