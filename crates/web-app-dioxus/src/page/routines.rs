use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService, SessionService};

use crate::{
    DOMAIN_SERVICE, NOTIFICATIONS, Route,
    cache::{Cache, CacheState},
    eh, ensure_session,
    routing::NavigatorScrollExt,
    ui::{
        element::{
            DeleteConfirmationDialog, Dialog, ErrorMessage, FloatingActionButton, Icon,
            LoadingPage, MenuOption, NoConnection, OptionsMenu, SearchBox, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
};

#[component]
pub fn Routines(add: bool, search: String) -> Element {
    ensure_session!();

    let cache = consume_context::<Cache>();
    let mut dialog = use_signal(|| RoutineDialog::None);

    let search_term = search.clone();
    let show_add_dialog = move || {
        let search_term = search_term.clone();
        async move {
            let validated_name = DOMAIN_SERVICE()
                .validate_routine_name(&search_term, domain::RoutineID::nil())
                .await
                .map_err(|err| err.to_string());
            *dialog.write() = RoutineDialog::Add {
                name: FieldValue {
                    input: search_term.clone(),
                    validated: validated_name,
                    orig: search_term.clone(),
                },
            };
            navigator().replace_preserving_scroll(Route::Routines {
                add: true,
                search: search_term,
            });
        }
    };

    let show_add_dialog_for_future = show_add_dialog.clone();
    use_future(move || {
        let show_add_dialog = show_add_dialog_for_future.clone();
        async move {
            if add {
                show_add_dialog().await;
            }
        }
    });

    match (&*cache.routines.read(), &*cache.training_sessions.read()) {
        (CacheState::Ready(routines), CacheState::Ready(training_sessions)) => {
            rsx! {
                {view_search_box(&search)},
                {view_list(routines, training_sessions, &search, dialog)}
                {view_dialog(dialog, Some(Route::Routines { add: false, search: search.clone() }))}
                FloatingActionButton {
                    icon: "plus".to_string(),
                    onclick: move |_| { show_add_dialog() },
                }
            }
        }
        (CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)), _) => {
            rsx! { NoConnection {  } {} }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { ErrorMessage { message: err } }
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => rsx! { LoadingPage {} },
    }
}

fn view_search_box(search_term: &str) -> Element {
    rsx! {
        div {
            class: "px-4",
            SearchBox {
                search_term,
                oninput: move |event: FormEvent| {
                    navigator().replace_preserving_scroll(Route::Routines {
                        add: false,
                        search: event.value(),
                    });
                }
            }
        }
    }
}

fn view_list(
    routines: &[domain::Routine],
    training_sessions: &[domain::TrainingSession],
    search_term: &str,
    mut dialog: Signal<RoutineDialog>,
) -> Element {
    let current_routines =
        domain::routines_sorted_by_last_use(routines, training_sessions, |r: &domain::Routine| {
            !r.archived
                && r.name
                    .as_ref()
                    .to_lowercase()
                    .contains(&search_term.trim().to_lowercase())
        });
    let archived_routines =
        domain::routines_sorted_by_last_use(routines, training_sessions, |r: &domain::Routine| {
            r.archived
                && r.name
                    .as_ref()
                    .to_lowercase()
                    .contains(&search_term.trim().to_lowercase())
        });

    let current_routines_body = current_routines
        .into_iter()
        .map(|r| {
            vec![
                rsx! {
                    Link {
                        to: Route::Routine { id: r.id },
                        "{r.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-right",
                        a {
                            class: "mx-2",
                            "data-testid": "item-options",
                            onclick: move |_| { *dialog.write() = RoutineDialog::Options(r.clone()); },
                            Icon { name: "ellipsis-vertical"}
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    let archived_routines_body = archived_routines
        .into_iter()
        .map(|r| {
            vec![
                rsx! {
                    Link {
                        to: Route::Routine { id: r.id },
                        "{r.name}"
                    }
                },
                rsx! {
                    div {
                        class: "has-text-right",
                        a {
                            class: "mx-2",
                            "data-testid": "item-options",
                            onclick: move |_| { *dialog.write() = RoutineDialog::Options(r.clone()); },
                            Icon { name: "ellipsis-vertical"}
                        }
                    }
                },
            ]
        })
        .collect::<Vec<_>>();

    rsx! {
        Table { body: current_routines_body }
        if !archived_routines_body.is_empty() {
            Title { "Archive" }
            Table { body: archived_routines_body }
        }
    }
}

pub fn view_dialog(
    mut dialog: Signal<RoutineDialog>,
    closed_dialog_route: Option<Route>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {
            is_loading.set(true);
            $block;
            is_loading.set(false);
        };
    }

    let close_dialog = move || {
        dialog.set(RoutineDialog::None);
        if let Some(route) = closed_dialog_route {
            navigator().replace_preserving_scroll(route);
        }
    };

    let save = eh!(close_dialog; {
        async move {
            let mut saved = false;
            is_loading! {
                if let RoutineDialog::Add { name } | RoutineDialog::Copy { name, .. } | RoutineDialog::Rename { name, .. } = &*dialog.read() {
                    if let Ok(name) = name.validated.clone() {
                        match &*dialog.read() {
                            RoutineDialog::Add { .. } => {
                                match DOMAIN_SERVICE()
                                    .create_routine(name, vec![])
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        consume_context::<Cache>().refresh_routines();
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to add routine: {err}"));
                                    }
                                }
                            }
                            RoutineDialog::Copy { routine_id, .. } => {
                                match &*consume_context::<Cache>().routines.read() {
                                    CacheState::Ready(routines) => {
                                        let sections = routines.iter().find(|r| r.id == *routine_id).map(|routine| {
                                            routine
                                                .sections
                                                .clone()
                                        }).unwrap_or_default();
                                        match DOMAIN_SERVICE()
                                            .create_routine(name, sections)
                                            .await
                                        {
                                            Ok(_) => {
                                                saved = true;
                                                consume_context::<Cache>().refresh_routines();
                                            }
                                            Err(err) => {
                                                NOTIFICATIONS
                                                    .write()
                                                    .push(format!("Failed to copy routine: {err}"));
                                            }
                                        }
                                    }
                                    CacheState::Error(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to copy routine: {err}"));
                                    }
                                    CacheState::Loading => {
                                        NOTIFICATIONS
                                            .write()
                                            .push("Failed to copy routine: Cache is loading".to_string());
                                    }
                                }
                            }
                            RoutineDialog::Rename { routine_id, .. } => {
                                match DOMAIN_SERVICE()
                                    .modify_routine(*routine_id, Some(name), None, None)
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        consume_context::<Cache>().refresh_routines();
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to rename routine: {err}"));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            if saved {
                close_dialog();
            }
        }
    });
    let delete = eh!(close_dialog; {
        async move {
            let mut deleted = false;
            is_loading! {
                if let RoutineDialog::Delete(routine) = &*dialog.read() {
                    match DOMAIN_SERVICE().delete_routine(routine.id).await {
                        Ok(()) => {
                            deleted = true;
                            consume_context::<Cache>().refresh_routines();
                        },
                        Err(err) => NOTIFICATIONS.write().push(format!("Failed to delete training session: {err}"))
                    }
                }
            }
            if deleted {
                close_dialog();
            }
        }
    });

    match &*dialog.read() {
        RoutineDialog::None => rsx! {},
        RoutineDialog::Options(routine) => {
            let routine = routine.clone();
            let routine_name_copy = routine.name.clone();
            let routine_name_edit = routine.name.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: (if routine.archived { "box-open" } else { "box-archive" }).to_string(),
                                text: (if routine.archived { "Unarchive routine" } else { "Archive routine" }).to_string(),
                                "data-testid": "options-archive",
                                onclick: eh!(close_dialog; {
                                    async move {
                                        match DOMAIN_SERVICE()
                                            .modify_routine(routine.id, None, Some(!routine.archived), None)
                                            .await
                                        {
                                            Ok(_) => {
                                                close_dialog();
                                                consume_context::<Cache>().refresh_routines();
                                            }
                                            Err(err) => {
                                                NOTIFICATIONS
                                                    .write()
                                                    .push(format!("Failed to modify routine: {err}"));
                                            }
                                        }
                                    }
                                })
                            },
                            MenuOption {
                                icon: "copy".to_string(),
                                text: "Copy routine".to_string(),
                                "data-testid": "options-copy",
                                onclick: move |_| {
                                    let routine_name = routine_name_copy.clone();
                                    async move {
                                        let validated_name = DOMAIN_SERVICE().validate_routine_name(&routine_name.to_string(), domain::RoutineID::nil()).await.map_err(|err| err.to_string());
                                        *dialog.write() = RoutineDialog::Copy {
                                            name: FieldValue {
                                                input: routine_name.to_string(),
                                                validated: validated_name,
                                                orig: routine_name.to_string(),
                                            },
                                            routine_id: routine.id,
                                        };
                                    }
                                }
                            },
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Rename routine".to_string(),
                                "data-testid": "options-rename",
                                onclick: move |_| {
                                    let routine_name = routine_name_edit.clone();
                                    *dialog.write() = RoutineDialog::Rename {
                                        name: FieldValue::new(routine_name),
                                        routine_id: routine.id,
                                    };
                                }
                            },
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete routine".to_string(),
                                "data-testid": "options-delete",
                                onclick: move |_| { *dialog.write() = RoutineDialog::Delete(routine.clone()); }
                            },
                        },
                    ],
                    close_event: eh!(close_dialog; { close_dialog(); })
                }
            }
        }
        RoutineDialog::Add { name }
        | RoutineDialog::Copy { name, .. }
        | RoutineDialog::Rename { name, .. } => rsx! {
            Dialog {
                title: rsx! { match &*dialog.read() { RoutineDialog::Add { .. } => { "Add routine" }, RoutineDialog::Copy { .. } =>  { "Copy routine" }, RoutineDialog::Rename { .. } =>  { "Rename routine" }, _ => "" } },
                close_event: eh!(close_dialog; { close_dialog(); }),
                InputField {
                    label: "Name".to_string(),
                    value: name.input.clone(),
                    error: if let Err(err) = &name.validated { err.clone() },
                    has_changed: name.changed(),
                    oninput: move |event: FormEvent| {
                        let input = event.value();
                        dialog.with_mut(|dlg| {
                            match dlg {
                                RoutineDialog::Add { name, .. }
                                | RoutineDialog::Copy { name, .. }
                                | RoutineDialog::Rename { name, .. } => {
                                    name.input.clone_from(&input);
                                }
                                _ => {}
                            }
                        });
                        let routine_id = {
                            match &*dialog.read() {
                                RoutineDialog::Rename { routine_id, .. } => *routine_id,
                                _ => domain::RoutineID::nil()
                            }
                        };
                        async move {
                            // Debounce the validation to prevent unexpected input field updates
                            // caused by rapid inputs
                            gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
                            {
                                match &*dialog.read() {
                                    RoutineDialog::Add { name, .. } | RoutineDialog::Copy { name, .. } | RoutineDialog::Rename { name, .. } => {
                                        if name.input != input {
                                            return;
                                        }
                                    }
                                    _ => { }
                                }
                            }
                            let validated_name = DOMAIN_SERVICE().validate_routine_name(&input, routine_id).await.map_err(|err| err.to_string());
                            dialog.with_mut(|dialog|
                                match dialog {
                                    RoutineDialog::Add { name, .. } | RoutineDialog::Copy { name, .. } | RoutineDialog::Rename { name, .. } => {
                                        if name.input == input {
                                            name.validated = validated_name;
                                        }
                                    }
                                    _ => { }
                                }
                            );
                        }
                    }
                }
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        onclick: eh!(close_dialog; { close_dialog(); }),
                        button { class: "button is-light is-soft", "data-testid": "dialog-cancel", "Cancel" }
                    }
                    div {
                        class: "control",
                        onclick: save,
                        button {
                            class: "button is-primary",
                            class: if is_loading() { "is-loading" },
                            "data-testid": "dialog-save",
                            disabled: !name.valid(),
                            "Save"
                        }
                    }
                }
            }
        },
        RoutineDialog::Delete(routine) => rsx! {
            DeleteConfirmationDialog {
                element_type: "routine".to_string(),
                element_name: rsx! { "{routine.name}" },
                delete_event: delete.clone(),
                cancel_event: eh!(close_dialog; { close_dialog(); }),
                is_loading: is_loading(),
            }
        },
    }
}

pub enum RoutineDialog {
    None,
    Options(domain::Routine),
    Add {
        name: FieldValue<domain::Name>,
    },
    Copy {
        name: FieldValue<domain::Name>,
        routine_id: domain::RoutineID,
    },
    Rename {
        name: FieldValue<domain::Name>,
        routine_id: domain::RoutineID,
    },
    Delete(domain::Routine),
}
