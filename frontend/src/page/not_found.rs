use seed::{prelude::*, *};

pub fn view<Ms>() -> Node<Ms> {
    div![
        C!["message"],
        C!["is-danger"],
        C!["mx-2"],
        div![C!["message-body"], "Page not found"],
    ]
}
