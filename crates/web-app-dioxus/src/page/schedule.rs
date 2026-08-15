use std::rc::Rc;

use dioxus::prelude::*;

use valens_domain::{self as domain, ScheduleService};

use crate::{
    DOMAIN_SERVICE, Route,
    cache::{Cache, CacheState},
    eh,
    loading::LoadingFlag,
    notification::{notify, notify_error},
    ui::{
        drag_and_drop,
        element::{
            Dialog, ErrorPage, Icon, ItemOptionsButton, LoadingDialog, LoadingPage, MenuOption,
            NoConnection, OptionsMenu, SaveDialog, Table, Title,
        },
        form::{FieldValue, FieldValueState, InputField},
    },
};

static IS_LOADING: GlobalSignal<bool> = Signal::global(|| false);

#[component]
pub fn Schedule() -> Element {
    let cache = consume_context::<Cache>();
    let dialog = use_signal(|| ScheduleDialog::None);
    let drag = use_signal(|| None::<Drag>);

    match (&*cache.schedule.read(), &*cache.routines.read()) {
        (CacheState::Ready(schedule), CacheState::Ready(routines)) => {
            rsx! {
                {view_week(schedule, routines, dialog, drag)}
                {view_rotations(schedule, routines, dialog, drag)}
                {drag_and_drop::view_drag_overlay(drag, &DropTarget::Remove)}
                {view_dialog(dialog, schedule, routines)}
                if IS_LOADING() && matches!(*dialog.read(), ScheduleDialog::None) {
                    LoadingDialog {}
                }
            }
        }
        (CacheState::Error(domain::ReadError::Storage(domain::StorageError::NoConnection)), _) => {
            rsx! { NoConnection {} }
        }
        (CacheState::Error(err), _) | (_, CacheState::Error(err)) => {
            rsx! { ErrorPage { "{err}" } }
        }
        (CacheState::Loading, _) | (_, CacheState::Loading) => rsx! { LoadingPage {} },
    }
}

fn view_week(
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
    mut dialog: Signal<ScheduleDialog>,
    drag: Signal<Option<Drag>>,
) -> Element {
    rsx! {
        div {
            class: "columns schedule-columns",
            for weekday in domain::Weekday::iter() {
                div {
                    class: "column",
                    "data-testid": "schedule-day-{u8::from(*weekday)}",
                    "data-drop": "day-{u8::from(*weekday)}",
                    Title {
                        class: "is-size-6 is-nowrap".to_string(),
                        actions: rsx! {
                            a {
                                class: "schedule-action schedule-title-action",
                                "data-testid": "add-slot",
                                onclick: {
                                    let weekday = *weekday;
                                    move |_| {
                                        *dialog.write() = ScheduleDialog::AddSlot { weekday };
                                    }
                                },
                                Icon { name: "plus", is_small: true }
                            }
                        },
                        "{weekday}"
                    }
                    {view_slots(schedule, routines, *weekday, drag)}
                }
            }
        }
    }
}

fn view_slots(
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
    weekday: domain::Weekday,
    drag: Signal<Option<Drag>>,
) -> Element {
    let items = schedule
        .entries()
        .get(&weekday)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|slot| match slot {
            domain::ScheduleSlot::Routine(routine_id) => rsx! {
                Link {
                    to: Route::Routine { id: routine_id },
                    "data-testid": "slot-name",
                    {routine_name(routines, routine_id)}
                }
            },
            domain::ScheduleSlot::Rotation(rotation_id) => rsx! {
                span {
                    "data-testid": "slot-name",
                    {rotation_name(schedule, rotation_id)}
                }
            },
        })
        .collect();
    view_drag_list(
        schedule,
        drag,
        items,
        move |index| {
            (
                DragSource::Slot(weekday, index),
                DropTarget::Slot(weekday, index),
                format!("slot-{}-{index}", u8::from(weekday)),
            )
        },
        DropTarget::Day(weekday),
        DragListLabels {
            item_testid: "schedule-slot",
            handle_testid: "slot-handle",
            empty_testid: "rest-day",
            empty_label: "Rest day",
        },
    )
}

fn view_rotations(
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
    mut dialog: Signal<ScheduleDialog>,
    drag: Signal<Option<Drag>>,
) -> Element {
    rsx! {
        div {
            "data-testid": "schedule-rotations",
            div {
                class: "schedule-heading",
                Title {
                    actions: rsx! {
                        a {
                            class: "schedule-action schedule-title-action",
                            "data-testid": "add-rotation",
                            onclick: move |_| {
                                *dialog.write() = ScheduleDialog::EditRotationName {
                                    rotation_id: None,
                                    name: FieldValue::default(),
                                };
                            },
                            Icon { name: "plus", is_small: true }
                        }
                    },
                    "Rotations"
                }
            }
            div {
                class: "columns is-multiline is-centered schedule-columns",
                for (rotation_id, rotation) in sorted_rotations(schedule) {
                    div {
                        class: "column is-one-fifth",
                        "data-testid": "schedule-rotation",
                        "data-drop": "rotation-{rotation_id.as_u128()}",
                        div {
                            class: "is-flex is-justify-content-center is-align-items-center mb-3 is-relative",
                            span {
                                class: "has-text-weight-bold",
                                "data-testid": "rotation-name",
                                "{rotation.name}"
                            }
                            div {
                                class: "schedule-action",
                                ItemOptionsButton {
                                    on_click: move |_| {
                                        *dialog.write() =
                                            ScheduleDialog::RotationOptions { rotation_id };
                                    }
                                }
                            }
                        }
                        {view_rotation_routines(schedule, routines, rotation_id, &rotation, drag)}
                    }
                }
            }
        }
    }
}

fn view_rotation_routines(
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
    rotation_id: domain::RotationID,
    rotation: &domain::Rotation,
    drag: Signal<Option<Drag>>,
) -> Element {
    let items = rotation
        .routines()
        .iter()
        .map(|routine_id| {
            rsx! {
                Link {
                    to: Route::Routine { id: *routine_id },
                    "data-testid": "rotation-routine-name",
                    {routine_name(routines, *routine_id)}
                }
            }
        })
        .collect();
    view_drag_list(
        schedule,
        drag,
        items,
        move |index| {
            (
                DragSource::RotationRoutine(rotation_id, index),
                DropTarget::RotationRoutine(rotation_id, index),
                format!("rotation-routine-{}-{index}", rotation_id.as_u128()),
            )
        },
        DropTarget::Rotation(rotation_id),
        DragListLabels {
            item_testid: "rotation-routine",
            handle_testid: "rotation-routine-handle",
            empty_testid: "no-routines",
            empty_label: "No routines",
        },
    )
}

#[derive(Clone, Copy)]
struct DragListLabels {
    item_testid: &'static str,
    handle_testid: &'static str,
    empty_testid: &'static str,
    empty_label: &'static str,
}

/// Render `items` as a drag & drop list.
///
/// `targets` maps an item index to its drag source, the drop target for insertion before the
/// item, and its `data-drop` identifier; the index one past the last item yields the drop target
/// for insertion at the end. An empty list renders a drop zone for `empty_target`.
fn view_drag_list(
    schedule: &domain::Schedule,
    drag: Signal<Option<Drag>>,
    items: Vec<Element>,
    targets: impl Fn(usize) -> (DragSource, DropTarget, String),
    empty_target: DropTarget,
    labels: DragListLabels,
) -> Element {
    let num_items = items.len();
    let target = drag_and_drop::hovered_target(drag);
    let droppable = drag()
        .is_some_and(|drag| drag.active && is_valid_target(schedule, drag.source, empty_target));
    let validator = drag_validator(schedule);
    let handler = drop_handler(schedule);
    let (_, end_target, _) = targets(num_items);
    let rows = items
        .into_iter()
        .enumerate()
        .map(|(index, content)| {
            let (source, insert_target, drop_id) = targets(index);
            rsx! {
                div {
                    class: "box schedule-tile is-flex is-justify-content-space-between is-align-items-center px-4 py-3",
                    "data-drag-state": drag_and_drop::drag_state(drag, &source),
                    "data-drop-state": drag_and_drop::insertion_state(
                        target == Some(insert_target),
                        index + 1 == num_items && target == Some(end_target),
                    ),
                    "data-testid": "{labels.item_testid}",
                    "data-drop": "{drop_id}",
                    {content}
                    {drag_and_drop::view_drag_handle(
                        drag,
                        source,
                        labels.handle_testid,
                        validator.clone(),
                        handler.clone(),
                    )}
                }
            }
        })
        .collect::<Vec<_>>();
    rsx! {
        {rows.into_iter()}
        if num_items == 0 {
            div {
                class: "is-drop-zone schedule-tile is-size-7 has-text-centered px-4 py-3",
                class: if target == Some(empty_target) { "has-text-text-bold" } else { "has-text-grey-light" },
                class: if droppable { "is-active" },
                "data-drop-state": drag_and_drop::drop_state(target == Some(empty_target)),
                "data-testid": "{labels.empty_testid}",
                {labels.empty_label}
            }
        }
    }
}

fn view_dialog(
    mut dialog: Signal<ScheduleDialog>,
    schedule: &domain::Schedule,
    routines: &[domain::Routine],
) -> Element {
    if matches!(&*dialog.read(), ScheduleDialog::None) {
        return rsx! {};
    }

    let close_dialog = move || {
        let mut dialog = dialog;
        dialog.set(ScheduleDialog::None);
    };

    let schedule_for_save = schedule.clone();
    let save = eh!(close_dialog; {
        let schedule = schedule_for_save.clone();
        async move {
            let modified_schedule = match &*dialog.read() {
                ScheduleDialog::EditRotationName { rotation_id, name } => name
                    .validated
                    .clone()
                    .ok()
                    .and_then(|name| match rotation_id {
                        Some(rotation_id) => rename_rotation(&schedule, *rotation_id, name),
                        None => add_rotation(&schedule, name),
                    }),
                ScheduleDialog::None
                | ScheduleDialog::AddSlot { .. }
                | ScheduleDialog::RotationOptions { .. }
                | ScheduleDialog::AddRotationRoutine { .. } => None,
            };
            let mut saved = false;
            if let Some(modified_schedule) = modified_schedule {
                saved = save_schedule(modified_schedule).await;
            }
            if saved {
                close_dialog();
            }
        }
    });

    match &*dialog.read() {
        ScheduleDialog::None => rsx! {},
        ScheduleDialog::AddSlot { weekday } => {
            let weekday = *weekday;
            let mut options = sorted_rotations(schedule)
                .into_iter()
                .map(|(rotation_id, rotation)| {
                    (
                        domain::ScheduleSlot::Rotation(rotation_id),
                        rsx! {
                            Icon { name: "rotate", is_small: true, class: "mr-2".to_string() }
                            "{rotation.name}"
                        },
                    )
                })
                .collect::<Vec<_>>();
            options.extend(sorted_by_name(routines, &[]).into_iter().map(|routine| {
                (
                    domain::ScheduleSlot::Routine(routine.id),
                    rsx! { "{routine.name}" },
                )
            }));
            let options = options
                .into_iter()
                .map(|(slot, label)| {
                    let schedule = schedule.clone();
                    (
                        label,
                        EventHandler::new(eh!(close_dialog; {
                            if let Some(modified) = add_slot(&schedule, weekday, slot) {
                                spawn_save(modified);
                            }
                            close_dialog();
                        })),
                    )
                })
                .collect();
            view_selection_dialog(
                rsx! { "{weekday}" },
                "slot-option",
                options,
                EventHandler::new(eh!(close_dialog; { close_dialog(); })),
            )
        }
        ScheduleDialog::EditRotationName { rotation_id, name } => {
            let rotation_id = *rotation_id;
            let title = if rotation_id.is_none() {
                rsx! { "Add rotation" }
            } else {
                rsx! { "Rename rotation" }
            };
            rsx! {
                SaveDialog {
                    title,
                    on_close: eh!(close_dialog; { close_dialog(); }),
                    on_save: save,
                    is_loading: IS_LOADING(),
                    disabled: !name.valid(),
                    InputField {
                        label: "Name".to_string(),
                        "data-testid": "dialog-name",
                        value: name.input.clone(),
                        error: if let Err(err) = &name.validated { err.clone() },
                        has_changed: name.changed(),
                        autofocus: true,
                        on_input: {
                            let schedule = schedule.clone();
                            move |event: FormEvent| {
                                if let ScheduleDialog::EditRotationName { name, .. } = &mut *dialog.write() {
                                    name.input = event.value();
                                    name.validated = schedule
                                        .validate_rotation_name(&name.input, rotation_id)
                                        .map_err(|err| err.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        ScheduleDialog::AddRotationRoutine { rotation_id } => {
            let rotation_id = *rotation_id;
            let Some(rotation) = schedule.rotations().get(&rotation_id) else {
                return rsx! {};
            };
            let options = sorted_by_name(routines, rotation.routines())
                .into_iter()
                .map(|routine| {
                    let routine_id = routine.id;
                    let schedule = schedule.clone();
                    let mut routines = rotation.routines().to_vec();
                    routines.push(routine_id);
                    (
                        rsx! { "{routine.name}" },
                        EventHandler::new(eh!(close_dialog; {
                            if let Some(modified) =
                                rotation_with_routines(&schedule, rotation_id, routines.clone())
                            {
                                spawn_save(modified);
                            }
                            close_dialog();
                        })),
                    )
                })
                .collect();
            view_selection_dialog(
                rsx! { "Add routine to rotation" },
                "routine-option",
                options,
                EventHandler::new(eh!(close_dialog; { close_dialog(); })),
            )
        }
        ScheduleDialog::RotationOptions { rotation_id } => {
            let rotation_id = *rotation_id;
            let Some(rotation) = schedule.rotations().get(&rotation_id) else {
                return rsx! {};
            };
            let name = rotation.name.clone();
            let schedule = schedule.clone();
            rsx! {
                OptionsMenu {
                    options: vec![
                        rsx! {
                            MenuOption {
                                icon: "plus".to_string(),
                                text: "Add routine".to_string(),
                                "data-testid": "options-add-routine",
                                on_click: move |_| {
                                    *dialog.write() =
                                        ScheduleDialog::AddRotationRoutine { rotation_id };
                                },
                            }
                        },
                        rsx! {
                            MenuOption {
                                icon: "edit".to_string(),
                                text: "Rename rotation".to_string(),
                                "data-testid": "options-rename",
                                on_click: move |_| {
                                    *dialog.write() = ScheduleDialog::EditRotationName {
                                        rotation_id: Some(rotation_id),
                                        name: FieldValue::new(name.clone()),
                                    };
                                },
                            }
                        },
                        rsx! {
                            MenuOption {
                                icon: "times".to_string(),
                                text: "Delete rotation".to_string(),
                                "data-testid": "options-delete",
                                on_click: move |_| {
                                    delete_rotation(&schedule, rotation_id);
                                    dialog.set(ScheduleDialog::None);
                                },
                            }
                        },
                    ],
                    on_close: eh!(close_dialog; { close_dialog(); }),
                }
            }
        }
    }
}

/// Render a dialog with a list of clickable options.
fn view_selection_dialog(
    title: Element,
    option_testid: &'static str,
    options: Vec<(Element, EventHandler<MouseEvent>)>,
    on_close: EventHandler<MouseEvent>,
) -> Element {
    let body = options
        .into_iter()
        .map(|(label, on_select)| {
            vec![rsx! {
                span {
                    class: "has-text-link",
                    "data-testid": "{option_testid}",
                    onclick: on_select,
                    {label}
                }
            }]
        })
        .collect::<Vec<_>>();
    rsx! {
        Dialog {
            title,
            on_close,
            no_horizontal_padding: true,
            Table { body }
        }
    }
}

fn delete_rotation(schedule: &domain::Schedule, rotation_id: domain::RotationID) {
    if let Some(schedule) = remove_rotation(schedule, rotation_id) {
        spawn_save(schedule);
    }
}

fn remove_rotation(
    schedule: &domain::Schedule,
    rotation_id: domain::RotationID,
) -> Option<domain::Schedule> {
    let mut schedule = schedule.clone();
    match schedule.remove_rotation(rotation_id) {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to delete rotation: {err}"));
            None
        }
    }
}

fn sorted_by_name<'a>(
    routines: &'a [domain::Routine],
    excluded: &[domain::RoutineID],
) -> Vec<&'a domain::Routine> {
    let mut routines = routines
        .iter()
        .filter(|routine| !routine.archived && !excluded.contains(&routine.id))
        .collect::<Vec<_>>();
    routines.sort_by(|a, b| a.name.cmp(&b.name));
    routines
}

fn sorted_rotations(schedule: &domain::Schedule) -> Vec<(domain::RotationID, domain::Rotation)> {
    let mut rotations = schedule
        .rotations()
        .iter()
        .map(|(rotation_id, rotation)| (*rotation_id, rotation.clone()))
        .collect::<Vec<_>>();
    rotations.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    rotations
}

fn routine_name(routines: &[domain::Routine], id: domain::RoutineID) -> String {
    routines
        .iter()
        .find(|routine| routine.id == id)
        .map_or_else(|| "Unknown routine".to_string(), |r| r.name.to_string())
}

fn rotation_name(schedule: &domain::Schedule, id: domain::RotationID) -> String {
    schedule
        .rotations()
        .get(&id)
        .map_or_else(|| "Unknown rotation".to_string(), |r| r.name.to_string())
}

fn add_slot(
    schedule: &domain::Schedule,
    weekday: domain::Weekday,
    slot: domain::ScheduleSlot,
) -> Option<domain::Schedule> {
    let mut schedule = schedule.clone();
    match schedule.add_slot(weekday, slot) {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to change schedule: {err}"));
            None
        }
    }
}

fn remove_slot(
    schedule: &domain::Schedule,
    weekday: domain::Weekday,
    index: usize,
) -> Option<domain::Schedule> {
    let mut schedule = schedule.clone();
    schedule.remove_slot(weekday, index)?;
    Some(schedule)
}

fn drag_validator(
    schedule: &domain::Schedule,
) -> impl Fn(DragSource, DropTarget) -> bool + Clone + 'static {
    let schedule = Rc::new(schedule.clone());
    move |source, target| is_valid_target(&schedule, source, target)
}

fn drop_handler(schedule: &domain::Schedule) -> impl Fn(DragSource, DropTarget) + Clone + 'static {
    let schedule = Rc::new(schedule.clone());
    move |source, target| {
        if let Some(modified) = apply_drop(&schedule, source, target)
            && modified != *schedule
        {
            spawn_save(modified);
        }
    }
}

/// Convert the element index of a hovered slot into an insertion index.
///
/// Pointing at the upper half of an element inserts before it, pointing at the lower half inserts
/// after it. Targets without an index are returned unchanged.
fn insertion_target(target: DropTarget, lower_half: bool) -> DropTarget {
    if !lower_half {
        return target;
    }
    match target {
        DropTarget::Slot(weekday, index) => DropTarget::Slot(weekday, index + 1),
        DropTarget::RotationRoutine(rotation_id, index) => {
            DropTarget::RotationRoutine(rotation_id, index + 1)
        }
        DropTarget::Day(_) | DropTarget::Rotation(_) | DropTarget::Remove => target,
    }
}

fn parse_weekday(value: &str) -> Option<domain::Weekday> {
    domain::Weekday::try_from(value.parse::<u8>().ok()?).ok()
}

fn parse_rotation_id(value: &str) -> Option<domain::RotationID> {
    value.parse::<u128>().ok().map(domain::RotationID::from)
}

/// Whether dropping `source` on `target` is allowed.
///
/// Slots can only be dropped on days and rotation routines only on rotations. A rotation that
/// already contains the dragged routine is not a valid target.
fn is_valid_target(schedule: &domain::Schedule, source: DragSource, target: DropTarget) -> bool {
    match (source, target) {
        (DragSource::Slot(..), DropTarget::Slot(..) | DropTarget::Day(_) | DropTarget::Remove)
        | (DragSource::RotationRoutine(..), DropTarget::Remove) => true,
        (
            DragSource::RotationRoutine(source_rotation_id, index),
            DropTarget::RotationRoutine(target_rotation_id, _)
            | DropTarget::Rotation(target_rotation_id),
        ) => {
            if source_rotation_id == target_rotation_id {
                return true;
            }
            let Some(routine_id) = schedule
                .rotations()
                .get(&source_rotation_id)
                .and_then(|rotation| rotation.routines().get(index))
            else {
                return false;
            };
            schedule
                .rotations()
                .get(&target_rotation_id)
                .is_some_and(|rotation| !rotation.routines().contains(routine_id))
        }
        (DragSource::Slot(..), DropTarget::RotationRoutine(..) | DropTarget::Rotation(..))
        | (DragSource::RotationRoutine(..), DropTarget::Slot(..) | DropTarget::Day(..)) => false,
    }
}

/// Apply the effect of dropping `source` on `target`, or `None` if the combination is invalid.
fn apply_drop(
    schedule: &domain::Schedule,
    source: DragSource,
    target: DropTarget,
) -> Option<domain::Schedule> {
    match (source, target) {
        (DragSource::Slot(weekday, index), DropTarget::Slot(target_weekday, target_index)) => {
            drop_slot(
                schedule,
                (weekday, index),
                target_weekday,
                Some(target_index),
            )
        }
        (DragSource::Slot(weekday, index), DropTarget::Day(target_weekday)) => {
            drop_slot(schedule, (weekday, index), target_weekday, None)
        }
        (DragSource::Slot(weekday, index), DropTarget::Remove) => {
            remove_slot(schedule, weekday, index)
        }
        (
            DragSource::RotationRoutine(rotation_id, index),
            DropTarget::RotationRoutine(target_rotation_id, target_index),
        ) => drop_rotation_routine(
            schedule,
            (rotation_id, index),
            target_rotation_id,
            Some(target_index),
        ),
        (
            DragSource::RotationRoutine(rotation_id, index),
            DropTarget::Rotation(target_rotation_id),
        ) => drop_rotation_routine(schedule, (rotation_id, index), target_rotation_id, None),
        (DragSource::RotationRoutine(rotation_id, index), DropTarget::Remove) => {
            remove_rotation_routine(schedule, rotation_id, index)
        }
        (DragSource::Slot(..), DropTarget::RotationRoutine(..) | DropTarget::Rotation(..))
        | (DragSource::RotationRoutine(..), DropTarget::Slot(..) | DropTarget::Day(..)) => None,
    }
}

/// Move the slot at `source` to the insertion position `target_index` of `target_weekday` or to
/// the end of `target_weekday` if `target_index` is `None`.
fn drop_slot(
    schedule: &domain::Schedule,
    source: (domain::Weekday, usize),
    target_weekday: domain::Weekday,
    target_index: Option<usize>,
) -> Option<domain::Schedule> {
    let (source_weekday, source_index) = source;
    let mut schedule = schedule.clone();
    let slot = schedule.remove_slot(source_weekday, source_index)?;
    let index = match target_index {
        // `target_index` refers to the list before the removal of the source slot
        Some(target_index) if source_weekday == target_weekday && source_index < target_index => {
            target_index - 1
        }
        Some(target_index) => target_index,
        None => schedule.entries().get(&target_weekday).map_or(0, Vec::len),
    };
    match schedule.insert_slot(target_weekday, index, slot) {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to change schedule: {err}"));
            None
        }
    }
}

/// Move the routine at `source` to the insertion position `target_index` of `target_rotation_id`
/// or to the end of `target_rotation_id` if `target_index` is `None`.
fn drop_rotation_routine(
    schedule: &domain::Schedule,
    source: (domain::RotationID, usize),
    target_rotation_id: domain::RotationID,
    target_index: Option<usize>,
) -> Option<domain::Schedule> {
    let (source_rotation_id, source_index) = source;
    let mut routines = schedule
        .rotations()
        .get(&source_rotation_id)?
        .routines()
        .to_vec();
    if source_index >= routines.len() {
        return None;
    }
    let routine_id = routines.remove(source_index);
    if source_rotation_id == target_rotation_id {
        let index = match target_index {
            // `target_index` refers to the list before the removal of the source routine
            Some(target_index) if source_index < target_index => target_index - 1,
            Some(target_index) => target_index.min(routines.len()),
            None => routines.len(),
        };
        routines.insert(index, routine_id);
        rotation_with_routines(schedule, source_rotation_id, routines)
    } else {
        let mut target_routines = schedule
            .rotations()
            .get(&target_rotation_id)?
            .routines()
            .to_vec();
        let index = target_index
            .unwrap_or(target_routines.len())
            .min(target_routines.len());
        target_routines.insert(index, routine_id);
        let schedule = rotation_with_routines(schedule, source_rotation_id, routines)?;
        rotation_with_routines(&schedule, target_rotation_id, target_routines)
    }
}

fn remove_rotation_routine(
    schedule: &domain::Schedule,
    rotation_id: domain::RotationID,
    index: usize,
) -> Option<domain::Schedule> {
    let mut routines = schedule.rotations().get(&rotation_id)?.routines().to_vec();
    if index >= routines.len() {
        return None;
    }
    routines.remove(index);
    rotation_with_routines(schedule, rotation_id, routines)
}

fn add_rotation(schedule: &domain::Schedule, name: domain::Name) -> Option<domain::Schedule> {
    let rotation_id = domain::RotationID::from(
        schedule
            .rotations()
            .keys()
            .map(|id| id.as_u128())
            .max()
            .unwrap_or(0)
            + 1,
    );
    let mut schedule = schedule.clone();
    let result = domain::Rotation::new(name, vec![])
        .map_err(|err| err.to_string())
        .and_then(|rotation| {
            schedule
                .insert_rotation(rotation_id, rotation)
                .map_err(|err| err.to_string())
        });
    match result {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to add rotation: {err}"));
            None
        }
    }
}

fn rename_rotation(
    schedule: &domain::Schedule,
    rotation_id: domain::RotationID,
    name: domain::Name,
) -> Option<domain::Schedule> {
    let mut schedule = schedule.clone();
    match schedule.rename_rotation(rotation_id, name) {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to rename rotation: {err}"));
            None
        }
    }
}

fn rotation_with_routines(
    schedule: &domain::Schedule,
    rotation_id: domain::RotationID,
    routines: Vec<domain::RoutineID>,
) -> Option<domain::Schedule> {
    let mut schedule = schedule.clone();
    let rotation = schedule.rotations().get(&rotation_id)?;
    let result = domain::Rotation::new(rotation.name.clone(), routines)
        .map_err(|err| err.to_string())
        .and_then(|rotation| {
            schedule
                .insert_rotation(rotation_id, rotation)
                .map_err(|err| err.to_string())
        });
    match result {
        Ok(()) => Some(schedule),
        Err(err) => {
            notify_error(format!("Failed to change rotation: {err}"));
            None
        }
    }
}

fn spawn_save(schedule: domain::Schedule) {
    spawn(async move {
        save_schedule(schedule).await;
    });
}

async fn save_schedule(schedule: domain::Schedule) -> bool {
    let _loading = LoadingFlag::set(&IS_LOADING);
    match DOMAIN_SERVICE().modify_schedule(schedule).await {
        Ok(_) => {
            consume_context::<Cache>().refresh_schedule();
            true
        }
        Err(err) => {
            notify("Failed to change schedule", &err);
            false
        }
    }
}

type Drag = drag_and_drop::Drag<DragSource, DropTarget>;

#[derive(Clone, Copy, Debug, PartialEq)]
enum DragSource {
    Slot(domain::Weekday, usize),
    RotationRoutine(domain::RotationID, usize),
}

/// The index of `Slot` and `RotationRoutine` is an insertion position between 0 and the number of
/// elements, referring to the gap before the element with the same index.
#[derive(Clone, Copy, Debug, PartialEq)]
enum DropTarget {
    Slot(domain::Weekday, usize),
    Day(domain::Weekday),
    RotationRoutine(domain::RotationID, usize),
    Rotation(domain::RotationID),
    Remove,
}

impl drag_and_drop::DropTarget for DropTarget {
    fn parse(value: &str) -> Option<Self> {
        if value == "remove" {
            return Some(Self::Remove);
        }
        if let Some(weekday) = value.strip_prefix("day-") {
            return Some(Self::Day(parse_weekday(weekday)?));
        }
        if let Some(slot) = value.strip_prefix("slot-") {
            let (weekday, index) = slot.split_once('-')?;
            return Some(Self::Slot(parse_weekday(weekday)?, index.parse().ok()?));
        }
        if let Some(rotation_routine) = value.strip_prefix("rotation-routine-") {
            let (rotation_id, index) = rotation_routine.split_once('-')?;
            return Some(Self::RotationRoutine(
                parse_rotation_id(rotation_id)?,
                index.parse().ok()?,
            ));
        }
        if let Some(rotation_id) = value.strip_prefix("rotation-") {
            return Some(Self::Rotation(parse_rotation_id(rotation_id)?));
        }
        None
    }

    fn resolve(self, element: &web_sys::Element, y: f64) -> Self {
        match self {
            Self::Slot(..) | Self::RotationRoutine(..) => {
                insertion_target(self, drag_and_drop::in_lower_half(element, y))
            }
            Self::Day(weekday) => {
                drag_and_drop::insertion_index(element, "[data-drop^='slot-']", y)
                    .map_or(self, |index| Self::Slot(weekday, index))
            }
            Self::Rotation(rotation_id) => {
                drag_and_drop::insertion_index(element, "[data-drop^='rotation-routine-']", y)
                    .map_or(self, |index| Self::RotationRoutine(rotation_id, index))
            }
            Self::Remove => self,
        }
    }

    fn suspends_auto_scroll(&self) -> bool {
        *self == Self::Remove
    }
}

enum ScheduleDialog {
    None,
    AddSlot {
        weekday: domain::Weekday,
    },
    EditRotationName {
        rotation_id: Option<domain::RotationID>,
        name: FieldValue<domain::Name>,
    },
    AddRotationRoutine {
        rotation_id: domain::RotationID,
    },
    RotationOptions {
        rotation_id: domain::RotationID,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::ui::drag_and_drop::DropTarget as _;

    use super::*;

    #[test]
    fn test_add_and_remove_slot() {
        let schedule = schedule();
        let modified = add_slot(
            &schedule,
            domain::Weekday::Tuesday,
            domain::ScheduleSlot::Routine(2.into()),
        )
        .unwrap();
        assert_eq!(
            modified.entries()[&domain::Weekday::Tuesday],
            vec![domain::ScheduleSlot::Routine(2.into())]
        );
        assert_eq!(
            remove_slot(&modified, domain::Weekday::Tuesday, 0),
            Some(schedule)
        );
    }

    #[test]
    fn test_remove_slot_keeps_remaining_slots_of_day() {
        let schedule = schedule();
        let modified = remove_slot(&schedule, domain::Weekday::Monday, 0).unwrap();
        assert_eq!(
            modified.entries()[&domain::Weekday::Monday],
            vec![domain::ScheduleSlot::Routine(1.into())]
        );
    }

    #[test]
    fn test_drop_slot_within_day() {
        let schedule = schedule();
        let modified = drop_slot(
            &schedule,
            (domain::Weekday::Monday, 0),
            domain::Weekday::Monday,
            None,
        )
        .unwrap();
        assert_eq!(
            modified.entries()[&domain::Weekday::Monday],
            vec![
                domain::ScheduleSlot::Routine(1.into()),
                domain::ScheduleSlot::Rotation(1.into()),
            ]
        );
        assert_eq!(
            drop_slot(
                &modified,
                (domain::Weekday::Monday, 1),
                domain::Weekday::Monday,
                Some(0)
            ),
            Some(schedule)
        );
    }

    #[test]
    fn test_drop_slot_onto_itself_keeps_schedule_unchanged() {
        let schedule = schedule();
        assert_eq!(
            drop_slot(
                &schedule,
                (domain::Weekday::Monday, 0),
                domain::Weekday::Monday,
                Some(0)
            ),
            Some(schedule.clone())
        );
        assert_eq!(
            drop_slot(
                &schedule,
                (domain::Weekday::Monday, 0),
                domain::Weekday::Monday,
                Some(1)
            ),
            Some(schedule)
        );
    }

    #[test]
    fn test_drop_slot_between_days() {
        let schedule = schedule();
        let modified = drop_slot(
            &schedule,
            (domain::Weekday::Monday, 0),
            domain::Weekday::Friday,
            None,
        )
        .unwrap();
        assert_eq!(
            modified.entries()[&domain::Weekday::Monday],
            vec![domain::ScheduleSlot::Routine(1.into())]
        );
        assert_eq!(
            modified.entries()[&domain::Weekday::Friday],
            vec![domain::ScheduleSlot::Rotation(1.into())]
        );
        assert_eq!(
            drop_slot(
                &modified,
                (domain::Weekday::Friday, 0),
                domain::Weekday::Monday,
                Some(0)
            ),
            Some(schedule)
        );
    }

    #[test]
    fn test_drop_slot_with_invalid_source() {
        let schedule = schedule();
        assert_eq!(
            drop_slot(
                &schedule,
                (domain::Weekday::Tuesday, 0),
                domain::Weekday::Monday,
                None
            ),
            None
        );
        assert_eq!(
            drop_slot(
                &schedule,
                (domain::Weekday::Monday, 2),
                domain::Weekday::Friday,
                None
            ),
            None
        );
    }

    #[test]
    fn test_parse_drop_target() {
        assert_eq!(DropTarget::parse("remove"), Some(DropTarget::Remove));
        assert_eq!(
            DropTarget::parse("day-3"),
            Some(DropTarget::Day(domain::Weekday::Wednesday))
        );
        assert_eq!(
            DropTarget::parse("slot-1-2"),
            Some(DropTarget::Slot(domain::Weekday::Monday, 2))
        );
        assert_eq!(
            DropTarget::parse("rotation-1"),
            Some(DropTarget::Rotation(1.into()))
        );
        assert_eq!(
            DropTarget::parse("rotation-routine-1-2"),
            Some(DropTarget::RotationRoutine(1.into(), 2))
        );
        assert_eq!(DropTarget::parse("day-8"), None);
        assert_eq!(DropTarget::parse("slot-1"), None);
        assert_eq!(DropTarget::parse("slot-x-0"), None);
        assert_eq!(DropTarget::parse("rotation-x"), None);
        assert_eq!(DropTarget::parse("rotation-routine-1"), None);
        assert_eq!(DropTarget::parse(""), None);
    }

    #[test]
    fn test_insertion_target() {
        let slot = DropTarget::Slot(domain::Weekday::Monday, 1);
        assert_eq!(insertion_target(slot, false), slot);
        assert_eq!(
            insertion_target(slot, true),
            DropTarget::Slot(domain::Weekday::Monday, 2)
        );
        assert_eq!(
            insertion_target(DropTarget::RotationRoutine(1.into(), 1), true),
            DropTarget::RotationRoutine(1.into(), 2)
        );
        assert_eq!(
            insertion_target(DropTarget::Day(domain::Weekday::Monday), true),
            DropTarget::Day(domain::Weekday::Monday)
        );
    }

    #[test]
    fn test_apply_drop() {
        let schedule = schedule();
        let removed = apply_drop(
            &schedule,
            DragSource::Slot(domain::Weekday::Monday, 0),
            DropTarget::Remove,
        )
        .unwrap();
        assert_eq!(
            removed.entries()[&domain::Weekday::Monday],
            vec![domain::ScheduleSlot::Routine(1.into())]
        );
        let moved_to_day = apply_drop(
            &schedule,
            DragSource::Slot(domain::Weekday::Monday, 0),
            DropTarget::Day(domain::Weekday::Friday),
        )
        .unwrap();
        assert_eq!(
            moved_to_day.entries()[&domain::Weekday::Friday],
            vec![domain::ScheduleSlot::Rotation(1.into())]
        );
        let reordered = apply_drop(
            &schedule,
            DragSource::Slot(domain::Weekday::Monday, 1),
            DropTarget::Slot(domain::Weekday::Monday, 0),
        )
        .unwrap();
        assert_eq!(
            reordered.entries()[&domain::Weekday::Monday],
            vec![
                domain::ScheduleSlot::Routine(1.into()),
                domain::ScheduleSlot::Rotation(1.into()),
            ]
        );
        let rotation_removed = apply_drop(
            &schedule,
            DragSource::RotationRoutine(1.into(), 0),
            DropTarget::Remove,
        )
        .unwrap();
        assert_eq!(
            rotation_removed.rotations()[&domain::RotationID::from(1)].routines(),
            [2.into()]
        );
    }

    #[test]
    fn test_apply_drop_with_mismatched_source_and_target() {
        let schedule = schedule();
        assert_eq!(
            apply_drop(
                &schedule,
                DragSource::Slot(domain::Weekday::Monday, 0),
                DropTarget::Rotation(1.into()),
            ),
            None
        );
        assert_eq!(
            apply_drop(
                &schedule,
                DragSource::RotationRoutine(1.into(), 0),
                DropTarget::Day(domain::Weekday::Monday),
            ),
            None
        );
    }

    #[test]
    fn test_is_valid_target_rejects_rotation_already_containing_routine() {
        // Rotation 1 contains routines 1 and 2, rotation 2 contains routines 2 and 3
        let mut schedule = schedule();
        schedule
            .insert_rotation(
                domain::RotationID::from(2),
                domain::Rotation::new(domain::Name::new("B").unwrap(), vec![2.into(), 3.into()])
                    .unwrap(),
            )
            .unwrap();
        let source_routine_1 = DragSource::RotationRoutine(1.into(), 0);
        let source_routine_2 = DragSource::RotationRoutine(1.into(), 1);
        assert!(is_valid_target(
            &schedule,
            source_routine_2,
            DropTarget::RotationRoutine(1.into(), 0)
        ));
        assert!(is_valid_target(
            &schedule,
            source_routine_1,
            DropTarget::Rotation(2.into())
        ));
        assert!(!is_valid_target(
            &schedule,
            source_routine_2,
            DropTarget::Rotation(2.into())
        ));
        assert!(!is_valid_target(
            &schedule,
            source_routine_2,
            DropTarget::RotationRoutine(2.into(), 0)
        ));
        assert!(is_valid_target(
            &schedule,
            source_routine_2,
            DropTarget::Remove
        ));
        assert!(!is_valid_target(
            &schedule,
            DragSource::RotationRoutine(1.into(), 2),
            DropTarget::Rotation(2.into())
        ));
        assert!(!is_valid_target(
            &schedule,
            source_routine_1,
            DropTarget::Rotation(3.into())
        ));
    }

    #[test]
    fn test_drop_rotation_routine_within_rotation() {
        let schedule = schedule();
        let modified = drop_rotation_routine(&schedule, (1.into(), 0), 1.into(), None).unwrap();
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(1)].routines(),
            [2.into(), 1.into()]
        );
        assert_eq!(
            drop_rotation_routine(&modified, (1.into(), 1), 1.into(), Some(0)),
            Some(schedule)
        );
    }

    #[test]
    fn test_drop_rotation_routine_onto_itself_keeps_schedule_unchanged() {
        let schedule = schedule();
        assert_eq!(
            drop_rotation_routine(&schedule, (1.into(), 0), 1.into(), Some(0)),
            Some(schedule.clone())
        );
        assert_eq!(
            drop_rotation_routine(&schedule, (1.into(), 0), 1.into(), Some(1)),
            Some(schedule)
        );
    }

    #[test]
    fn test_drop_rotation_routine_between_rotations() {
        let schedule = schedule_with_two_rotations();
        let modified = drop_rotation_routine(&schedule, (1.into(), 0), 2.into(), Some(0)).unwrap();
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(1)].routines(),
            [2.into()]
        );
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(2)].routines(),
            [1.into(), 3.into()]
        );
    }

    #[test]
    fn test_drop_rotation_routine_with_invalid_source_or_target() {
        let schedule = schedule();
        assert_eq!(
            drop_rotation_routine(&schedule, (2.into(), 0), 1.into(), None),
            None
        );
        assert_eq!(
            drop_rotation_routine(&schedule, (1.into(), 2), 1.into(), None),
            None
        );
        assert_eq!(
            drop_rotation_routine(&schedule, (1.into(), 0), 2.into(), None),
            None
        );
    }

    #[test]
    fn test_remove_rotation_routine() {
        let schedule = schedule();
        let modified = remove_rotation_routine(&schedule, 1.into(), 0).unwrap();
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(1)].routines(),
            [2.into()]
        );
        assert_eq!(remove_rotation_routine(&schedule, 1.into(), 2), None);
        assert_eq!(remove_rotation_routine(&schedule, 2.into(), 0), None);
    }

    #[test]
    fn test_remove_last_rotation_routine_keeps_empty_rotation() {
        let schedule = schedule_with_two_rotations();
        let modified = remove_rotation_routine(&schedule, 2.into(), 0).unwrap();
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(2)].routines(),
            []
        );
    }

    #[test]
    fn test_add_rotation_assigns_unused_id() {
        let schedule = schedule();
        let modified = add_rotation(&schedule, domain::Name::new("B").unwrap()).unwrap();
        assert_eq!(modified.rotations().len(), 2);
        let (id, rotation) = modified
            .rotations()
            .iter()
            .find(|(_, r)| r.name == domain::Name::new("B").unwrap())
            .unwrap();
        assert!(!schedule.rotations().contains_key(id));
        assert_eq!(rotation.routines(), []);
    }

    #[test]
    fn test_rename_rotation() {
        let schedule = schedule();
        let name = domain::Name::new("B").unwrap();
        let modified = rename_rotation(&schedule, 1.into(), name.clone()).unwrap();
        assert_eq!(
            modified.rotations()[&domain::RotationID::from(1)].name,
            name
        );
    }

    #[test]
    fn test_rotation_with_routines_preserves_name() {
        let schedule = schedule();
        let modified =
            rotation_with_routines(&schedule, 1.into(), vec![2.into(), 1.into()]).unwrap();
        let rotation = &modified.rotations()[&domain::RotationID::from(1)];
        assert_eq!(rotation.name, domain::Name::new("A").unwrap());
        assert_eq!(rotation.routines(), [2.into(), 1.into()]);
    }

    #[test]
    fn test_sorted_rotations_orders_by_name() {
        let mut schedule = schedule();
        schedule
            .insert_rotation(
                domain::RotationID::from(0),
                domain::Rotation::new(domain::Name::new("B").unwrap(), vec![1.into()]).unwrap(),
            )
            .unwrap();
        schedule
            .insert_rotation(
                domain::RotationID::from(2),
                domain::Rotation::new(domain::Name::new("0").unwrap(), vec![2.into()]).unwrap(),
            )
            .unwrap();
        assert_eq!(
            sorted_rotations(&schedule)
                .into_iter()
                .map(|(rotation_id, _)| rotation_id)
                .collect::<Vec<_>>(),
            [2.into(), 1.into(), 0.into()]
        );
    }

    fn schedule_with_two_rotations() -> domain::Schedule {
        let mut schedule = schedule();
        schedule
            .insert_rotation(
                domain::RotationID::from(2),
                domain::Rotation::new(domain::Name::new("B").unwrap(), vec![3.into()]).unwrap(),
            )
            .unwrap();
        schedule
    }

    fn schedule() -> domain::Schedule {
        domain::Schedule::new(
            BTreeMap::from([(
                domain::RotationID::from(1),
                domain::Rotation::new(domain::Name::new("A").unwrap(), vec![1.into(), 2.into()])
                    .unwrap(),
            )]),
            BTreeMap::from([(
                domain::Weekday::Monday,
                vec![
                    domain::ScheduleSlot::Rotation(1.into()),
                    domain::ScheduleSlot::Routine(1.into()),
                ],
            )]),
        )
        .unwrap()
    }
}
