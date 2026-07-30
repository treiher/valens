use dioxus::prelude::*;

/// Sets a loading flag and resets it on drop.
///
/// Resetting on drop also covers cancellation: a task that is dropped before it completes still
/// releases the flag.
pub struct LoadingFlag(&'static GlobalSignal<bool>);

impl LoadingFlag {
    pub fn set(flag: &'static GlobalSignal<bool>) -> Self {
        flag.with_mut(|is_loading| *is_loading = true);
        Self(flag)
    }
}

impl Drop for LoadingFlag {
    fn drop(&mut self) {
        self.0.with_mut(|is_loading| *is_loading = false);
    }
}

#[cfg(test)]
mod tests {
    use std::future::pending;

    use dioxus::core::{NoOpMutations, Task};

    use super::*;

    static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);
    static SHOW: GlobalSignal<bool> = Signal::global(|| true);
    static TASK: GlobalSignal<Option<Task>> = Signal::global(|| None);

    #[component]
    fn Page() -> Element {
        use_hook(|| {
            let task = spawn(async {
                let _loading = LoadingFlag::set(&IS_LOADING);
                pending::<()>().await;
            });
            TASK.with_mut(|t| *t = Some(task));
        });
        rsx! { div {} }
    }

    #[component]
    fn App() -> Element {
        rsx! {
            if SHOW() {
                Page {}
            }
        }
    }

    #[test]
    fn test_flag_is_reset_when_task_is_cancelled() {
        let mut dom = VirtualDom::new(App);
        dom.rebuild_in_place();

        dom.in_runtime(|| {
            // The flag is set on the first poll, and the task remains pending until it is
            // dropped with the unmounted component.
            assert!(TASK().unwrap().poll_now().is_pending());
            assert!(IS_LOADING());
            SHOW.with_mut(|show| *show = false);
        });
        dom.render_immediate(&mut NoOpMutations);

        dom.in_runtime(|| assert!(!IS_LOADING()));
    }
}
