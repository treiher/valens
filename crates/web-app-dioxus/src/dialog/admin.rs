use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{AuthService, Unreachable, UserService};

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE,
    diagnostics::log_failure,
    dialog::PasskeyTable,
    notification::{notify, notify_error},
    session::{Session, SessionRefresh},
    signal_changed_data,
    ui::{
        element::{
            CenteredBlock, Color, DeleteConfirmationDialog, Dialog, Error, Icon, ItemOptionsButton,
            Loading, MenuOption, Message, OptionsMenu, SaveDialog, ServerUnreachable, Table, Title,
            value_or_dash,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn AdminDialog(on_close: EventHandler<MouseEvent>) -> Element {
    rsx! {
        Dialog {
            title: rsx! { "Administration" },
            on_close,
            Users {}
        }
    }
}

#[component]
fn Users() -> Element {
    let users = use_resource(|| async {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE().get_users().await
    });
    let auth_methods = use_resource(|| async { DOMAIN_SERVICE().get_auth_methods().await });
    // The server offers no passkey login and cannot create login links without
    // `PUBLIC_URL`, so the corresponding actions are hidden and an explanation is shown
    let passkey_login_unavailable = matches!(
        &*auth_methods.read(),
        Some(Ok(methods)) if !methods.contains(&domain::AuthMethod::Passkey)
    );
    let mut dialog = use_signal(|| UserDialog::None);
    let mut is_loading = use_signal(|| false);

    macro_rules! is_loading {
        ($block:expr) => {{
            is_loading.set(true);
            $block;
            is_loading.set(false);
        }};
    }

    let mut close_dialog = move || {
        dialog.set(UserDialog::None);
    };

    let save = move |_| async move {
        let mut saved = false;
        is_loading! {
            match &*dialog.read() {
                UserDialog::Add { name, sex, height, role } => {
                    if let (Ok(name), Ok(sex), Ok(height), Ok(role)) = (name.validated.clone(), sex.validated.clone(), height.validated.clone(), role.validated.clone()) {
                        match DOMAIN_SERVICE().create_user(name, sex, height, role).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                            },
                            Err(err) => {
                                notify("add user", &err);
                            }
                        }
                    }
                },
                UserDialog::Edit { id, name, sex, height, role } => {
                    if let (Ok(name), Ok(sex), Ok(height), Ok(role)) = (name.validated.clone(), sex.validated.clone(), height.validated.clone(), role.validated.clone()) {
                        let id = *id;
                        match DOMAIN_SERVICE().replace_user(domain::User { id, name, sex, height, role }).await {
                            Ok(_) => {
                                saved = true;
                                signal_changed_data();
                                if id == consume_context::<Session>().user().id {
                                    consume_context::<SessionRefresh>().refresh();
                                }
                            },
                            Err(err) => {
                                notify("edit user", &err);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if saved {
            close_dialog();
        }
    };
    let delete = move |_| async move {
        let mut deleted = false;
        if let UserDialog::Delete(user) = &*dialog.read() {
            is_loading! {
                match DOMAIN_SERVICE().delete_user(user.id).await {
                    Ok(()) => {
                        deleted = true;
                        signal_changed_data();
                    },
                    Err(err) => notify("delete user", &err)

                }
            }
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    rsx! {
        match &*users.read() {
            Some(Ok(users)) => {
                rsx! {
                    if passkey_login_unavailable {
                        Message {
                            color: Color::Info,
                            "data-testid": "passkey-login-unavailable",
                            "Passkey login and login links are unavailable because PUBLIC_URL is not set in the server configuration."
                        }
                    }
                    Title {
                        actions: rsx! {
                            a {
                                "data-testid": "add-user",
                                onclick: move |_| {
                                    *dialog.write() = UserDialog::Add {
                                        name: FieldValue::default(),
                                        sex: FieldValue::new(domain::Sex::MALE),
                                        height: FieldValue::from_option(None),
                                        role: FieldValue::new(domain::Role::USER),
                                    };
                                },
                                Icon { name: "plus", is_small: true }
                            }
                        },
                        "Users"
                    }
                    Table {
                        head: vec![rsx! { "Name" }, rsx! { "Sex" }, rsx! { "Height (cm)" }, rsx! { "Role" }, rsx! {}],
                        body: users.iter().map(|user| {
                            let user = user.clone();
                            vec![
                                rsx! { "{user.name}" },
                                rsx! { "{user.sex}" },
                                rsx! { {value_or_dash(user.height)} },
                                rsx! { "{user.role}" },
                                rsx! {
                                    ItemOptionsButton { on_click: move |_| { *dialog.write() = UserDialog::Options(user.clone()); } }
                                }
                            ]
                        }).collect::<Vec<_>>()
                    }
                }
            }
            Some(Err(domain::ReadError::Forbidden(_))) => {
                rsx! {
                    CenteredBlock {
                        p {
                            "data-testid": "not-authorized",
                            "You are not authorized to manage users."
                        }
                    }
                }
            }
            Some(Err(err)) if err.unreachable() => {
                rsx! {
                    ServerUnreachable {}
                }
            }
            Some(Err(err)) => {
                log_failure("load users", err);
                rsx! {
                    Error { "{err}" }
                }
            }
            None => rsx! {
                Loading {}
            },
        }
        match &*dialog.read() {
            UserDialog::None => rsx! {},
            UserDialog::Options(user) => {
                let user_edit = user.clone();
                let user_passkeys = user.clone();
                let user_login_link = user.clone();
                let user_delete = user.clone();
                rsx! {
                    OptionsMenu {
                        options: vec![
                            rsx! {
                                MenuOption {
                                    icon: "user-edit".to_string(),
                                    text: "Edit user".to_string(),
                                    "data-testid": "options-edit-user",
                                    on_click: move |_| {
                                        *dialog.write() = UserDialog::Edit {
                                            id: user_edit.id,
                                            name: FieldValue {
                                                input: user_edit.name.to_string(),
                                                validated: Ok(user_edit.name.clone()),
                                                orig: user_edit.name.to_string()
                                            },
                                            sex: FieldValue {
                                                input: user_edit.sex.to_string(),
                                                validated: Ok(user_edit.sex),
                                                orig: user_edit.sex.to_string()
                                            },
                                            height: FieldValue::from_option(user_edit.height),
                                            role: FieldValue {
                                                input: user_edit.role.to_string(),
                                                validated: Ok(user_edit.role),
                                                orig: user_edit.role.to_string()
                                            },
                                        };
                                    }
                                },
                                MenuOption {
                                    icon: "key".to_string(),
                                    text: "Manage passkeys".to_string(),
                                    "data-testid": "options-manage-passkeys",
                                    on_click: move |_| { *dialog.write() = UserDialog::Passkeys(user_passkeys.clone()); }
                                },
                                if !passkey_login_unavailable {
                                    MenuOption {
                                        icon: "link".to_string(),
                                        text: "Create login link".to_string(),
                                        "data-testid": "options-create-login-link",
                                        on_click: move |_| { *dialog.write() = UserDialog::LoginLink(user_login_link.clone()); }
                                    }
                                },
                                MenuOption {
                                    icon: "user-times".to_string(),
                                    text: "Delete user".to_string(),
                                    "data-testid": "options-delete-user",
                                    on_click: move |_| { *dialog.write() = UserDialog::Delete(user_delete.clone()); }
                                },
                            },
                        ],
                        on_close: move |_| *dialog.write() = UserDialog::None,
                    }
                }
            },
            UserDialog::Passkeys(user) => rsx! {
                UserPasskeysDialog { user: user.clone(), on_close: close }
            },
            UserDialog::LoginLink(user) => rsx! {
                LoginLinkDialog { user: user.clone(), on_close: close }
            },
            UserDialog::Add { name, sex, height, role } | UserDialog::Edit { name, sex, height, role, .. } => {
                rsx! {
                SaveDialog {
                    title: rsx! { if let UserDialog::Add { .. } = &*dialog.read() { "Add user" } else { "Edit user" } },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: (!name.changed() && !sex.changed() && !height.changed() && !role.changed()) || !name.valid() || !sex.valid() || !height.valid() || !role.valid(),
                    InputField {
                        label: "Name".to_string(),
                        "data-testid": "user-name",
                        value: name.input.clone(),
                        error: if let Err(err) = &name.validated { err.clone() },
                        has_changed: name.changed(),
                        autofocus: true,
                        on_input: move |event: FormEvent| {
                            let input = event.value();
                            match &mut *dialog.write() {
                                UserDialog::Add { name, .. }
                                | UserDialog::Edit { name, .. } => {
                                    name.input.clone_from(&input);
                                }
                                _ => {}
                            }
                            let id = {
                                match &*dialog.read() {
                                    UserDialog::Edit { id, .. } => *id,
                                    _ => domain::UserID::nil()
                                }
                            };
                            async move {
                                // Debounce the validation to prevent unexpected input field updates
                                // caused by rapid inputs
                                gloo_timers::future::sleep(std::time::Duration::from_millis(10)).await;
                                {
                                    match &*dialog.read() {
                                        UserDialog::Add { name, .. } | UserDialog::Edit { name, .. }
                                            if name.input != input => {
                                                return;
                                            }
                                        _ => {}
                                    }
                                }
                                let validated_name = DOMAIN_SERVICE().validate_user_name(&input, id).await.map_err(|err| err.to_string());
                                match &mut *dialog.write() {
                                    UserDialog::Add { name, .. } | UserDialog::Edit { name, .. }
                                        if name.input == input => {
                                            name.validated = validated_name;
                                        }
                                    _ => {}
                                }
                            }
                        }
                    }
                    SelectField {
                        label: "Sex".to_string(),
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: domain::Sex::FEMALE.to_string(),
                                    value: domain::Sex::FEMALE.to_string(),
                                    selected: matches!(sex.validated, Ok(domain::Sex::FEMALE)),
                                }
                            },
                            rsx! {
                                SelectOption {
                                    text: domain::Sex::MALE.to_string(),
                                    value: domain::Sex::MALE.to_string(),
                                    selected: matches!(sex.validated, Ok(domain::Sex::MALE)),
                                }
                            },
                        ],
                        has_changed: sex.changed(),
                        on_change: move |event: FormEvent| {
                            if let UserDialog::Add { sex, .. } | UserDialog::Edit { sex, .. } = &mut *dialog.write() {
                                sex.input = event.value();
                                sex.validated = Ok(domain::Sex::from(sex.input.as_ref()));
                            }
                        }
                    }
                    InputField {
                        label: "Height".to_string(),
                        "data-testid": "user-height",
                        right_icon: rsx! { "cm" },
                        inputmode: "numeric".to_string(),
                        value: height.input.clone(),
                        error: if let Err(err) = &height.validated { err.clone() },
                        has_changed: height.changed(),
                        on_input: move |event: FormEvent| {
                            if let UserDialog::Add { height, .. } | UserDialog::Edit { height, .. } = &mut *dialog.write() {
                                height.input = event.value();
                                height.validated = DOMAIN_SERVICE()
                                    .validate_user_height(&height.input)
                                    .map_err(|err| err.to_string());
                            }
                        }
                    }
                    SelectField {
                        label: "Role".to_string(),
                        "data-testid": "user-role",
                        options: vec![
                            rsx! {
                                SelectOption {
                                    text: domain::Role::USER.to_string(),
                                    value: domain::Role::USER.to_string(),
                                    selected: matches!(role.validated, Ok(domain::Role::USER)),
                                }
                            },
                            rsx! {
                                SelectOption {
                                    text: domain::Role::ADMIN.to_string(),
                                    value: domain::Role::ADMIN.to_string(),
                                    selected: matches!(role.validated, Ok(domain::Role::ADMIN)),
                                }
                            },
                        ],
                        has_changed: role.changed(),
                        on_change: move |event: FormEvent| {
                            if let UserDialog::Add { role, .. } | UserDialog::Edit { role, .. } = &mut *dialog.write() {
                                role.input = event.value();
                                role.validated = Ok(domain::Role::from(role.input.as_ref()));
                            }
                        }
                    }
                }
            }},
            UserDialog::Delete(user) => {
                rsx! {
                    DeleteConfirmationDialog {
                        element_type: "user".to_string(),
                        element_name: rsx! { "{user.name}" },
                        on_delete: delete,
                        on_cancel: close,
                        is_loading: is_loading(),
                    }
                }
            },
        }
    }
}

#[component]
fn UserPasskeysDialog(user: domain::User, on_close: EventHandler<MouseEvent>) -> Element {
    let user_id = user.id;
    let mut passkeys =
        use_resource(move || async move { DOMAIN_SERVICE().get_passkeys(user_id).await });
    let mut passkey_to_delete = use_signal(|| Option::<domain::Passkey>::None);
    let mut is_loading = use_signal(|| false);

    let delete = move |_| async move {
        let mut deleted = false;
        if let Some(passkey) = &*passkey_to_delete.read() {
            is_loading.set(true);
            match DOMAIN_SERVICE().delete_passkey(user_id, passkey.id).await {
                Ok(()) => {
                    deleted = true;
                }
                Err(err) => notify("delete passkey", &err),
            }
            is_loading.set(false);
        }
        if deleted {
            passkey_to_delete.set(None);
            passkeys.restart();
        }
    };

    rsx! {
        Dialog {
            title: rsx! { "Passkeys of {user.name}" },
            on_close,
            match &*passkeys.read() {
                Some(Ok(passkeys)) => {
                    if passkeys.is_empty() {
                        rsx! {
                            CenteredBlock {
                                p {
                                    "data-testid": "no-passkeys",
                                    "No passkeys registered."
                                }
                            }
                        }
                    } else {
                        rsx! {
                            PasskeyTable {
                                passkeys: passkeys.clone(),
                                action: move |passkey: domain::Passkey| rsx! {
                                    button {
                                        class: "button is-small is-danger is-inverted",
                                        "data-testid": "delete-passkey",
                                        onclick: move |_| { passkey_to_delete.set(Some(passkey.clone())); },
                                        Icon { name: "trash" }
                                    }
                                },
                            }
                        }
                    }
                }
                Some(Err(err)) if err.unreachable() => rsx! {
                    ServerUnreachable {}
                },
                Some(Err(err)) => {
                    log_failure("load passkeys", err);
                    rsx! {
                        Error { "{err}" }
                    }
                }
                None => rsx! {
                    Loading {}
                },
            }
        }
        if let Some(passkey) = &*passkey_to_delete.read() {
            DeleteConfirmationDialog {
                element_type: "passkey".to_string(),
                element_name: rsx! { "{passkey.label}" },
                on_delete: delete,
                on_cancel: move |_| passkey_to_delete.set(None),
                is_loading: is_loading(),
            }
        }
    }
}

#[component]
fn LoginLinkDialog(user: domain::User, on_close: EventHandler<MouseEvent>) -> Element {
    let user_id = user.id;
    let url =
        use_resource(move || async move { DOMAIN_SERVICE().create_login_link(user_id).await });
    let mut copy_success = use_signal(|| false);

    rsx! {
        Dialog {
            title: rsx! { "Login link for {user.name}" },
            on_close,
            match &*url.read() {
                Some(Ok(url)) => {
                    let url = url.clone();
                    rsx! {
                        p {
                            class: "mb-3",
                            "The link can be used once within 24 hours to sign in without a passkey. \
                             Creating a new link invalidates all previously created links."
                        }
                        div {
                            class: "field has-addons",
                            div {
                                class: "control is-expanded",
                                input {
                                    class: "input",
                                    "data-testid": "login-link-url",
                                    readonly: true,
                                    value: "{url}",
                                }
                            }
                            div {
                                class: "control",
                                button {
                                    class: "button",
                                    "data-testid": "login-link-copy",
                                    onclick: move |_| {
                                        let url = url.clone();
                                        if let Some(window) = web_sys::window() {
                                            let promise = window.navigator().clipboard().write_text(&url);
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
                                    },
                                    if copy_success() {
                                        Icon { name: "check" }
                                    } else {
                                        Icon { name: "copy" }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(err)) if err.unreachable() => rsx! {
                    ServerUnreachable {}
                },
                Some(Err(err)) => {
                    log_failure("create login link", err);
                    rsx! {
                        Error { "{err}" }
                    }
                }
                None => rsx! {
                    Loading {}
                },
            }
        }
    }
}

enum UserDialog {
    None,
    Options(domain::User),
    Add {
        name: FieldValue<domain::Name>,
        sex: FieldValue<domain::Sex>,
        height: FieldValue<Option<u8>>,
        role: FieldValue<domain::Role>,
    },
    Edit {
        id: domain::UserID,
        name: FieldValue<domain::Name>,
        sex: FieldValue<domain::Sex>,
        height: FieldValue<Option<u8>>,
        role: FieldValue<domain::Role>,
    },
    Passkeys(domain::User),
    LoginLink(domain::User),
    Delete(domain::User),
}
