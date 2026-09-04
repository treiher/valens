//! Rendering of page components to HTML, and extraction of what they rendered.
//!
//! A page is rendered inside a router, which [`Link`] requires, and inside a root component,
//! which is where the contexts a page consumes can be constructed: they hold signals, which need
//! a running runtime and an owning scope.

use std::rc::Rc;

use dioxus::prelude::*;

/// The maximum number of rounds [`render_settled`] waits for the pending work to settle.
const MAX_SETTLE_ROUNDS: usize = 100;

/// Render `target` and return the resulting HTML.
///
/// # Panics
///
/// Panics if the target contributed no element. Dioxus renders nothing for a component that
/// panics, so a page reaching the browser turns into empty output rather than into a failure.
pub fn render(target: impl Fn() -> Element + 'static) -> String {
    let mut dom = virtual_dom(target);
    dom.rebuild_in_place();
    rendered(&dom)
}

/// Render `target` after its pending work has settled, so that a [`Resource`] reaches its ready
/// branch.
///
/// # Panics
///
/// Panics if the target contributed no element, like [`render`], or if the pending work has not
/// settled after [`MAX_SETTLE_ROUNDS`] rounds.
pub fn render_settled(target: impl Fn() -> Element + 'static) -> String {
    let mut dom = virtual_dom(target);
    dom.rebuild_in_place();
    for _ in 0..MAX_SETTLE_ROUNDS {
        // Awaiting `wait_for_work` blocks forever once no work remains, so it is polled once
        // per round and the loop stops when a round yields nothing.
        if futures_util::FutureExt::now_or_never(dom.wait_for_work()).is_none() {
            return rendered(&dom);
        }
        dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
    }
    panic!("pending work did not settle");
}

fn virtual_dom(target: impl Fn() -> Element + 'static) -> VirtualDom {
    // The target is passed through a context rather than through a static, so that tests
    // rendering different targets can run in parallel in the same binary.
    let target: Rc<dyn Fn() -> Element> = Rc::new(target);
    VirtualDom::new_with_props(
        Root,
        RootProps {
            target: PartialEqRc(target),
        },
    )
}

fn rendered(dom: &VirtualDom) -> String {
    let html = dioxus_ssr::render(dom);
    assert!(!html.is_empty(), "the target rendered no element");
    html
}

#[derive(Clone)]
struct PartialEqRc(Rc<dyn Fn() -> Element>);

impl PartialEq for PartialEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, PartialEq, Props)]
struct RootProps {
    target: PartialEqRc,
}

#[component]
fn Root(props: RootProps) -> Element {
    use_context_provider(|| props.target.clone());
    rsx! { Router::<TestRoute> {} }
}

/// Router of a single route, which is all [`Link`] needs to resolve the hrefs of the real
/// [`Route`](crate::Route) by string.
#[derive(Clone, PartialEq, Routable)]
enum TestRoute {
    #[route("/")]
    Target {},
}

#[component]
fn Target() -> Element {
    (use_context::<PartialEqRc>().0)()
}

/// The text of the element carrying `test_id`, with runs of whitespace collapsed.
///
/// # Panics
///
/// Panics if no element carries `test_id`, so that an assertion against an element that never
/// rendered cannot pass.
pub fn text_of(html: &str, test_id: &str) -> String {
    text(&element(&scraper::Html::parse_fragment(html), test_id))
}

/// The rows and cells of the element carrying `test_id`.
///
/// # Panics
///
/// Panics if no element carries `test_id`.
pub fn rows_of(html: &str, test_id: &str) -> Vec<Vec<String>> {
    let document = scraper::Html::parse_fragment(html);
    let row_selector = scraper::Selector::parse("tr").unwrap();
    let cell_selector = scraper::Selector::parse("th, td").unwrap();
    element(&document, test_id)
        .select(&row_selector)
        .map(|row| row.select(&cell_selector).map(|cell| text(&cell)).collect())
        .collect()
}

fn element<'a>(document: &'a scraper::Html, test_id: &str) -> scraper::ElementRef<'a> {
    let selector = scraper::Selector::parse(&format!("[data-testid=\"{test_id}\"]")).unwrap();
    document
        .select(&selector)
        .next()
        .unwrap_or_else(|| panic!("no element with test id `{test_id}`"))
}

fn text(element: &scraper::ElementRef) -> String {
    element
        .text()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_render_returns_the_rendered_target() {
        let html = render(|| rsx! { p { "data-testid": "greeting", "hello" } });

        assert_eq!(text_of(&html, "greeting"), "hello");
    }

    #[test]
    #[should_panic(expected = "the target rendered no element")]
    fn test_render_of_an_empty_target_panics() {
        render(|| rsx! {});
    }

    #[component]
    fn Resolving() -> Element {
        let value = use_resource(|| async { "ready" });
        rsx! {
            p { "data-testid": "value", {value.read().unwrap_or("loading")} }
        }
    }

    #[test]
    fn test_render_shows_the_loading_branch_of_a_resource() {
        let html = render(|| rsx! { Resolving {} });

        assert_eq!(text_of(&html, "value"), "loading");
    }

    #[test]
    fn test_render_settled_reaches_the_ready_branch_of_a_resource() {
        let html = render_settled(|| rsx! { Resolving {} });

        assert_eq!(text_of(&html, "value"), "ready");
    }

    #[test]
    fn test_text_of_collapses_whitespace_of_the_descendants() {
        let html = render(|| {
            rsx! {
                div {
                    "data-testid": "box",
                    span { "one" }
                    span { "  two\n  three  " }
                }
            }
        });

        assert_eq!(text_of(&html, "box"), "one two three");
    }

    #[test]
    #[should_panic(expected = "no element with test id `missing`")]
    fn test_text_of_an_absent_test_id_panics() {
        text_of(&render(|| rsx! { p { "text" } }), "missing");
    }

    #[test]
    fn test_rows_of_returns_the_cells_of_every_row() {
        let html = render(|| {
            rsx! {
                table {
                    "data-testid": "table",
                    tr { th { "name" } th { "value" } }
                    tr { td { "a" } td { "1" } }
                }
            }
        });

        assert_eq!(
            rows_of(&html, "table"),
            vec![vec!["name", "value"], vec!["a", "1"]]
        );
    }
}
