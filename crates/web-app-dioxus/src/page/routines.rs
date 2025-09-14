use dioxus::prelude::*;

use valens_domain::{self as domain, RoutineService, SessionService, TrainingSessionService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE, NOTIFICATIONS, Route,
    component::{
        element::{
            DeleteConfirmationDialog, Dialog, ErrorMessage, FloatingActionButton, Icon,
            LoadingPage, MenuOption, NoConnection, OptionsMenu, SearchBox, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
    eh, ensure_session, signal_changed_data,
};

#[component]
pub fn Routines(add: bool, search: String) -> Element {
    ensure_session!();

    let routines = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_routines().await
    });
    let training_sessions = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE.read().get_training_sessions().await
    });
    let mut dialog = use_signal(|| RoutineDialog::None);

    let search_term = search.clone();
    let show_add_dialog = move || {
        let search_term = search_term.clone();
        async move {
            *dialog.write() = RoutineDialog::Add {
                name: FieldValue {
                    input: search_term.clone(),
                    validated: DOMAIN_SERVICE
                        .read()
                        .validate_routine_name(&search_term, domain::RoutineID::nil())
                        .await
                        .map_err(|err| err.to_string()),
                    orig: search_term.clone(),
                },
            };
            navigator().replace(Route::Routines {
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

    match (&*routines.read(), &*training_sessions.read()) {
        (Some(Ok(routines)), Some(Ok(training_sessions))) => {
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
        (Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))), _) => {
            rsx! { NoConnection {  } {} }
        }
        (Some(Err(err)), _) | (_, Some(Err(err))) => {
            rsx! { ErrorMessage { message: err } }
        }
        (None, _) | (_, None) => rsx! { LoadingPage {} },
    }
}

fn view_search_box(search_term: &str) -> Element {
    rsx! {
        div {
            class: "px-4",
            SearchBox {
                search_term,
                oninput: move |event: FormEvent| {
                    navigator().replace(Route::Routines {
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
            Title { title: "Archive" }
            Table { body: archived_routines_body }
        }
    }
}

pub fn view_dialog(
    mut dialog: Signal<RoutineDialog>,
    closed_dialog_route: Option<Route>,
) -> Element {
    let mut is_loading = use_signal(|| false);

    let close_dialog = move || {
        *dialog.write() = RoutineDialog::None;
        if let Some(route) = closed_dialog_route {
            navigator().replace(route);
        }
    };

    macro_rules! is_loading {
        ($block:expr) => {
            *is_loading.write() = true;
            $block;
            *is_loading.write() = false;
        };
    }

    let save = eh!(close_dialog; {
        async move {
            let mut saved = false;
            is_loading! {
                if let RoutineDialog::Add { name } | RoutineDialog::Copy { name, .. } | RoutineDialog::Rename { name, .. } = &*dialog.read() {
                    if let Ok(name) = name.validated.clone() {
                        match &*dialog.read() {
                            RoutineDialog::Add { .. } => {
                                match DOMAIN_SERVICE
                                    .read()
                                    .create_routine(name, vec![])
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        signal_changed_data();
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to add routine: {err}"));
                                    }
                                }
                            }
                            RoutineDialog::Copy { routine_id, .. } => {
                                match DOMAIN_SERVICE.read().get_routines().await {
                                    Ok(routines) => {
                                        let sections = routines.iter().find(|r| r.id == *routine_id).map(|routine| {
                                            routine
                                                .sections
                                                .clone()
                                        }).unwrap_or_default();
                                        match DOMAIN_SERVICE
                                            .read()
                                            .create_routine(name, sections)
                                            .await
                                        {
                                            Ok(_) => {
                                                saved = true;
                                                signal_changed_data();
                                            }
                                            Err(err) => {
                                                NOTIFICATIONS
                                                    .write()
                                                    .push(format!("Failed to copy routine: {err}"));
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        NOTIFICATIONS
                                            .write()
                                            .push(format!("Failed to copy routine: {err}"));
                                        }
                                }
                            }
                            RoutineDialog::Rename { routine_id, .. } => {
                                match DOMAIN_SERVICE
                                    .read()
                                    .modify_routine(*routine_id, Some(name), None, None)
                                    .await
                                {
                                    Ok(_) => {
                                        saved = true;
                                        signal_changed_data();
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
                    match DOMAIN_SERVICE.read().delete_routine(routine.id).await {
                        Ok(_) => {
                            deleted = true;
                            signal_changed_data();
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
                                onclick: eh!(close_dialog; {
                                    async move {
                                        match DOMAIN_SERVICE
                                            .read()
                                            .modify_routine(routine.id, None, Some(!routine.archived), None)
                                            .await
                                        {
                                            Ok(_) => {
                                                close_dialog();
                                                signal_changed_data();
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
                                onclick: move |_| {
                                    let routine_name = routine_name_copy.clone();
                                    async move {
                                        *dialog.write() = RoutineDialog::Copy {
                                            name: FieldValue {
                                                input: routine_name.to_string(),
                                                validated: DOMAIN_SERVICE.read().validate_routine_name(&routine_name.to_string(), domain::RoutineID::nil()).await.map_err(|err| err.to_string()),
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
                        async move {
                            match &mut *dialog.write() {
                                RoutineDialog::Add { name, .. } | RoutineDialog::Copy { name, .. } => {
                                    name.input = event.value();
                                    name.validated = DOMAIN_SERVICE.read().validate_routine_name(&name.input, domain::RoutineID::nil()).await.map_err(|err| err.to_string());
                                }
                                RoutineDialog::Rename { name, routine_id } => {
                                    name.input = event.value();
                                    name.validated = DOMAIN_SERVICE.read().validate_routine_name(&name.input, *routine_id).await.map_err(|err| err.to_string());
                                }
                                _ => { }
                            }
                        }
                    }
                }
                div {
                    class: "field is-grouped is-grouped-centered",
                    div {
                        class: "control",
                        onclick: eh!(close_dialog; { close_dialog(); }),
                        button { class: "button is-light is-soft", "Cancel" }
                    }
                    div {
                        class: "control",
                        onclick: save,
                        button {
                            class: "button is-primary",
                            class: if is_loading() { "is-loading" },
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
