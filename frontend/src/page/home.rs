use seed::{prelude::*, *};

// ------ ------
//     Init
// ------ ------

pub fn init(_: Url, _: &mut impl Orders<Msg>) -> Model {
    Model {}
}

// ------ ------
//     Model
// ------ ------

pub struct Model {}

// ------ ------
//    Update
// ------ ------

pub enum Msg {}

pub fn update(msg: Msg, _: &mut Model, _: &mut impl Orders<Msg>) {
    match msg {}
}

// ------ ------
//     View
// ------ ------

pub fn view(_: &Model) -> Node<Msg> {
    div![C!["container"], C!["has-text-centered"], C!["mt-6"], "Home"]
}
