//! Rendering of page components to HTML, and extraction of what they rendered.
//!
//! A page is rendered inside a router, which [`Link`] requires, and inside a root component,
//! which is where the contexts a page consumes can be constructed: they hold signals, which need
//! a running runtime and an owning scope.

use std::rc::Rc;

use dioxus::prelude::*;

use valens_domain as domain;
use valens_web_app as web_app;

use crate::{
    cache::{Cache, CacheState},
    chart::WindowWidth,
    ongoing_training_session::{self, OngoingTrainingSession},
    session::{Session, SessionRefresh},
    settings::Settings,
};

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
    provide_context(props.target.clone());
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

/// The [`Cache`] a page reads, every collection ready and empty unless it is named.
pub struct TestCache {
    body_weight: CacheState<Vec<domain::BodyWeight>>,
    body_fat: CacheState<Vec<domain::BodyFat>>,
    period: CacheState<Vec<domain::Period>>,
    exercises: CacheState<Vec<domain::Exercise>>,
    routines: CacheState<Vec<domain::Routine>>,
    schedule: CacheState<domain::Schedule>,
    training_sessions: CacheState<Vec<domain::TrainingSession>>,
}

impl Default for TestCache {
    fn default() -> Self {
        Self {
            body_weight: CacheState::Ready(vec![]),
            body_fat: CacheState::Ready(vec![]),
            period: CacheState::Ready(vec![]),
            exercises: CacheState::Ready(vec![]),
            routines: CacheState::Ready(vec![]),
            schedule: CacheState::Ready(domain::Schedule::default()),
            training_sessions: CacheState::Ready(vec![]),
        }
    }
}

impl TestCache {
    /// A cache whose collections have not been read yet.
    #[must_use]
    pub fn loading() -> Self {
        Self {
            body_weight: CacheState::Loading,
            body_fat: CacheState::Loading,
            period: CacheState::Loading,
            exercises: CacheState::Loading,
            routines: CacheState::Loading,
            schedule: CacheState::Loading,
            training_sessions: CacheState::Loading,
        }
    }

    /// A cache whose collections could not be read.
    #[must_use]
    pub fn failing() -> Self {
        Self {
            body_weight: error(),
            body_fat: error(),
            period: error(),
            exercises: error(),
            routines: error(),
            schedule: error(),
            training_sessions: error(),
        }
    }

    #[must_use]
    pub fn with_body_weight(mut self, body_weight: Vec<domain::BodyWeight>) -> Self {
        self.body_weight = CacheState::Ready(body_weight);
        self
    }

    #[must_use]
    pub fn with_body_fat(mut self, body_fat: Vec<domain::BodyFat>) -> Self {
        self.body_fat = CacheState::Ready(body_fat);
        self
    }

    #[must_use]
    pub fn with_period(mut self, period: Vec<domain::Period>) -> Self {
        self.period = CacheState::Ready(period);
        self
    }

    #[must_use]
    pub fn with_exercises(mut self, exercises: Vec<domain::Exercise>) -> Self {
        self.exercises = CacheState::Ready(exercises);
        self
    }

    #[must_use]
    pub fn with_routines(mut self, routines: Vec<domain::Routine>) -> Self {
        self.routines = CacheState::Ready(routines);
        self
    }

    #[must_use]
    pub fn with_schedule(mut self, schedule: domain::Schedule) -> Self {
        self.schedule = CacheState::Ready(schedule);
        self
    }

    #[must_use]
    pub fn with_training_sessions(
        mut self,
        training_sessions: Vec<domain::TrainingSession>,
    ) -> Self {
        self.training_sessions = CacheState::Ready(training_sessions);
        self
    }

    /// Provide the cache. Must be called from a component, the signals it holds needing a
    /// running runtime and an owning scope.
    pub fn provide(self) {
        let cache = Cache {
            body_weight: Signal::new(self.body_weight),
            body_fat: Signal::new(self.body_fat),
            period: Signal::new(self.period),
            exercises: Signal::new(self.exercises),
            routines: Signal::new(self.routines),
            schedule: Signal::new(self.schedule),
            training_sessions: Signal::new(self.training_sessions),
        };
        provide_context(cache);
    }
}

fn error<T>() -> CacheState<T> {
    CacheState::Error(domain::ReadError::Storage(
        domain::StorageError::NoConnection,
    ))
}

/// Provide the session of `user`.
pub fn provide_session(user: domain::User) {
    provide_context(Session::new_for_test(user.clone()));
    provide_context(SessionRefresh::new_for_test(user));
}

/// Provide `settings`, alongside the window width the charts of a page are plotted for.
pub fn provide_settings(settings: web_app::Settings) {
    provide_context(Settings::new_for_test(settings));
    provide_context(WindowWidth(DEFAULT_WINDOW_WIDTH));
}

/// The window width a page renders for unless a test chooses one.
pub const DEFAULT_WINDOW_WIDTH: u32 = 420;

/// Seed the domain service of this virtual dom with `repository`.
///
/// Global signals live per runtime, so the seeded service is visible to the rendered page alone.
pub fn seed_domain_service(repository: domain::tests::FakeRepository) {
    *crate::DOMAIN_SERVICE.write() = domain::Service::new(repository);
}

/// Seed the web app service of this virtual dom with `repository`.
pub fn seed_web_app_service(repository: web_app::tests::FakeRepository) {
    *crate::WEB_APP_SERVICE.write() = web_app::Service::new(repository);
}

/// Provide the in-progress training session, if any.
pub fn provide_ongoing_training_session(state: ongoing_training_session::State) {
    provide_context(OngoingTrainingSession::new_for_test(state));
}

/// The text of the element carrying `test_id`, with runs of whitespace collapsed.
///
/// # Panics
///
/// Panics if no element carries `test_id`, so that an assertion against an element that never
/// rendered cannot pass.
pub fn text_of(html: &str, test_id: &str) -> String {
    text_of_nth(html, test_id, 0)
}

/// The text of the `index`th element carrying `test_id`, counted in document order.
///
/// # Panics
///
/// Panics if fewer elements carry `test_id`.
pub fn text_of_nth(html: &str, test_id: &str, index: usize) -> String {
    text(&element(
        &scraper::Html::parse_fragment(html),
        test_id,
        index,
    ))
}

/// The rows and cells of the element carrying `test_id`.
///
/// # Panics
///
/// Panics if no element carries `test_id`.
pub fn rows_of(html: &str, test_id: &str) -> Vec<Vec<String>> {
    rows_of_nth(html, test_id, 0)
}

/// The rows and cells of the `index`th element carrying `test_id`, counted in document order.
///
/// # Panics
///
/// Panics if fewer elements carry `test_id`.
pub fn rows_of_nth(html: &str, test_id: &str, index: usize) -> Vec<Vec<String>> {
    let document = scraper::Html::parse_fragment(html);
    let row_selector = scraper::Selector::parse("tr").unwrap();
    let cell_selector = scraper::Selector::parse("th, td").unwrap();
    element(&document, test_id, index)
        .select(&row_selector)
        .map(|row| row.select(&cell_selector).map(|cell| text(&cell)).collect())
        .collect()
}

fn element<'a>(
    document: &'a scraper::Html,
    test_id: &str,
    index: usize,
) -> scraper::ElementRef<'a> {
    let selector = scraper::Selector::parse(&format!("[data-testid=\"{test_id}\"]")).unwrap();
    document
        .select(&selector)
        .nth(index)
        .unwrap_or_else(|| panic!("no element {index} with test id `{test_id}`"))
}

/// The text of every element carrying `test_id`, in document order.
#[must_use]
pub fn all_text_of(html: &str, test_id: &str) -> Vec<String> {
    let document = scraper::Html::parse_fragment(html);
    let selector = scraper::Selector::parse(&format!("[data-testid=\"{test_id}\"]")).unwrap();
    document.select(&selector).map(|e| text(&e)).collect()
}

/// The value of `attribute` on the element carrying `test_id`.
///
/// # Panics
///
/// Panics if no element carries `test_id`, or if that element does not carry `attribute`.
#[must_use]
pub fn attribute_of(html: &str, test_id: &str, attribute: &str) -> String {
    let document = scraper::Html::parse_fragment(html);
    element(&document, test_id, 0)
        .value()
        .attr(attribute)
        .unwrap_or_else(|| {
            panic!("element with test id `{test_id}` has no attribute `{attribute}`")
        })
        .to_string()
}

/// Whether any element carries `test_id`.
#[must_use]
pub fn contains(html: &str, test_id: &str) -> bool {
    let document = scraper::Html::parse_fragment(html);
    let selector = scraper::Selector::parse(&format!("[data-testid=\"{test_id}\"]")).unwrap();
    document.select(&selector).next().is_some()
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
    #[should_panic(expected = "no element 0 with test id `missing`")]
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
