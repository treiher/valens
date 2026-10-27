use dioxus::prelude::*;

use valens_domain as domain;

use crate::{
    cache::{Cache, CacheState},
    eh,
    notification::notify_error,
    ui::element::{Block, Dialog, Error, Icon, LoadingDialog},
};

/// Shows the text written by `to_text` from the cached exercises, once they are loaded.
pub fn view_exercise_text_dialog(
    to_text: impl Fn(&[domain::Exercise]) -> String,
    on_close: impl FnOnce() + Clone + 'static,
) -> Element {
    match &*consume_context::<Cache>().exercises.read() {
        CacheState::Ready(exercises) => rsx! {
            TextDialog { text: to_text(exercises), on_close: move |_| on_close.clone()() }
        },
        CacheState::Error(err) => rsx! {
            Dialog {
                on_close: move |_| on_close.clone()(),
                Error { message: "{err}" }
            }
        },
        CacheState::Loading => rsx! { LoadingDialog {} },
    }
}

/// Shows a text with a button copying it to the clipboard.
#[component]
pub fn TextDialog(text: String, on_close: EventHandler<MouseEvent>) -> Element {
    let mut copy_success = use_signal(|| false);

    rsx! {
        Dialog {
            on_close,
            div {
                class: "is-relative",
                Block {
                    pre {
                        class: "has-text-text px-0 py-1",
                        "data-testid": "show-text-content",
                        "{text}"
                    }
                }
                button {
                    class: "button is-overlay-top-right p-0",
                    "data-testid": "show-text-copy",
                    onclick: eh!(text; {
                        if let Some(window) = web_sys::window() {
                            let promise = window.navigator().clipboard().write_text(&text);
                            spawn(async move {
                                match wasm_bindgen_futures::JsFuture::from(promise).await {
                                    Ok(_) => {
                                        *copy_success.write() = true;
                                        gloo_timers::future::TimeoutFuture::new(2_000).await;
                                        *copy_success.write() = false;
                                    }
                                    Err(e) => {
                                        log::error!("failed to copy to clipboard: {e:?}");
                                        notify_error(
                                            "copy to clipboard",
                                            "clipboard is not available",
                                        );
                                    }
                                }
                            });
                        }
                    }),
                    if copy_success() {
                        Icon { name: "check".to_string() }
                    } else {
                        Icon { name: "copy".to_string() }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::test_render::{TestCache, contains, render, text_of};

    use super::*;

    #[test]
    fn test_the_text_is_written_from_the_cached_exercises() {
        let html = render_with(|| TestCache::default().with_exercises(vec![exercise("Squat")]));

        assert_eq!(text_of(&html, "show-text-content"), "Squat");
    }

    #[rstest::rstest]
    #[case::unread(|| TestCache::default().with_unread_exercises())]
    #[case::unreadable(|| TestCache::default().with_unreadable_exercises())]
    fn test_no_text_is_shown_without_the_exercises(#[case] cache: fn() -> TestCache) {
        assert!(!contains(&render_with(cache), "show-text-content"));
    }

    fn render_with(cache: fn() -> TestCache) -> String {
        render(move || {
            cache().provide();
            view_exercise_text_dialog(
                |exercises| {
                    exercises
                        .iter()
                        .map(|e| e.name.to_string())
                        .collect::<String>()
                },
                || {},
            )
        })
    }

    fn exercise(name: &str) -> domain::Exercise {
        domain::Exercise {
            id: 1.into(),
            name: domain::Name::new(name).unwrap(),
            notes: String::new(),
            muscles: vec![],
            force: None,
            mechanic: None,
            laterality: None,
            assistance: None,
            equipment: vec![],
            category: None,
        }
    }
}
