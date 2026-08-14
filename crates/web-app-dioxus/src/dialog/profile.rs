use dioxus::prelude::*;

use valens_domain as domain;
use valens_domain::{AuthService, UserService};
use valens_storage as storage;

use crate::{
    DATA_CHANGED, DOMAIN_SERVICE,
    diagnostics::log_failure,
    dialog::PasskeyTable,
    notification::notify,
    session::{Session, SessionRefresh},
    signal_changed_data,
    ui::{
        element::{
            DeleteConfirmationDialog, Error, Icon, ItemOptionsButton, Loading, MenuOption,
            NoConnection, NoData, OptionsMenu, SaveDialog, Title,
        },
        form::{FieldValue, FieldValueState, InputField, SelectField, SelectOption},
    },
};

#[component]
pub fn ProfileDialog(on_close: EventHandler<MouseEvent>) -> Element {
    let user = consume_context::<Session>().user();
    let mut name = use_signal(|| FieldValue {
        input: user.name.to_string(),
        validated: Ok(user.name.clone()),
        orig: user.name.to_string(),
    });
    let mut sex = use_signal(|| FieldValue {
        input: user.sex.to_string(),
        validated: Ok(user.sex),
        orig: user.sex.to_string(),
    });
    let mut height = use_signal(|| FieldValue::from_option(user.height));
    let mut is_loading = use_signal(|| false);

    let user_id = user.id;
    let save = move |event: MouseEvent| async move {
        let mut saved = false;
        is_loading.set(true);
        if let (Ok(name), Ok(sex), Ok(height)) = (
            name.read().validated.clone(),
            sex.read().validated.clone(),
            height.read().validated.clone(),
        ) {
            match DOMAIN_SERVICE()
                .update_user(user_id, name, sex, height)
                .await
            {
                Ok(_) => {
                    saved = true;
                    signal_changed_data();
                    consume_context::<SessionRefresh>().refresh();
                }
                Err(err) => {
                    notify("Failed to edit profile", &err);
                }
            }
        }
        is_loading.set(false);
        if saved {
            on_close.call(event);
        }
    };

    rsx! {
        SaveDialog {
            title: rsx! { "Profile" },
            on_close,
            on_save: save,
            is_loading: is_loading(),
            disabled: (!name.read().changed() && !sex.read().changed() && !height.read().changed())
                || !name.read().valid() || !sex.read().valid() || !height.read().valid(),
            InputField {
                label: "Name".to_string(),
                "data-testid": "profile-name",
                value: name.read().input.clone(),
                error: if let Err(err) = &name.read().validated { err.clone() },
                has_changed: name.read().changed(),
                on_input: move |event: FormEvent| {
                    let mut name = name.write();
                    name.input = event.value();
                    name.validated = domain::Name::new(&name.input)
                        .map_err(|err| err.to_string());
                }
            }
            SelectField {
                label: "Sex".to_string(),
                options: vec![
                    rsx! {
                        SelectOption {
                            text: domain::Sex::FEMALE.to_string(),
                            value: domain::Sex::FEMALE.to_string(),
                            selected: matches!(sex.read().validated, Ok(domain::Sex::FEMALE)),
                        }
                    },
                    rsx! {
                        SelectOption {
                            text: domain::Sex::MALE.to_string(),
                            value: domain::Sex::MALE.to_string(),
                            selected: matches!(sex.read().validated, Ok(domain::Sex::MALE)),
                        }
                    },
                ],
                has_changed: sex.read().changed(),
                on_change: move |event: FormEvent| {
                    let mut sex = sex.write();
                    sex.input = event.value();
                    sex.validated = Ok(domain::Sex::from(sex.input.as_ref()));
                }
            }
            InputField {
                label: "Height".to_string(),
                "data-testid": "profile-height",
                right_icon: rsx! { "cm" },
                inputmode: "numeric".to_string(),
                value: height.read().input.clone(),
                error: if let Err(err) = &height.read().validated { err.clone() },
                has_changed: height.read().changed(),
                on_input: move |event: FormEvent| {
                    let mut height = height.write();
                    height.input = event.value();
                    height.validated = DOMAIN_SERVICE()
                        .validate_user_height(&height.input)
                        .map_err(|err| err.to_string());
                }
            }
            Passkeys { user_id }
        }
    }
}

#[component]
fn Passkeys(user_id: domain::UserID) -> Element {
    let passkeys = use_resource(move || async move {
        let _ = DATA_CHANGED.read();
        DOMAIN_SERVICE().get_passkeys(user_id).await
    });
    let auth_methods = use_resource(|| async { DOMAIN_SERVICE().get_auth_methods().await });
    let mut dialog = use_signal(|| PasskeyDialog::None);
    let mut is_loading = use_signal(|| false);

    // The server does not offer passkey login without `PUBLIC_URL`, so the section is
    // hidden to avoid a registration that is bound to fail
    if matches!(
        &*auth_methods.read(),
        Some(Ok(methods)) if !methods.contains(&domain::AuthMethod::Passkey)
    ) {
        return rsx! {};
    }

    // Without username login, a passkey is the only way to log in again, so deleting the
    // last passkey must be prevented
    let last_passkey_protected = matches!(
        &*auth_methods.read(),
        Some(Ok(methods)) if !methods.contains(&domain::AuthMethod::Username)
    ) && matches!(&*passkeys.read(), Some(Ok(passkeys)) if passkeys.len() == 1);

    let mut close_dialog = move || {
        dialog.set(PasskeyDialog::None);
    };

    let save = move |_| async move {
        let mut saved = false;
        is_loading.set(true);
        if let PasskeyDialog::Rename { id, label } = &*dialog.read()
            && let Ok(label) = label.validated.clone()
        {
            match DOMAIN_SERVICE().rename_passkey(user_id, *id, label).await {
                Ok(_) => {
                    saved = true;
                    signal_changed_data();
                }
                Err(err) => {
                    notify("Failed to rename passkey", &err);
                }
            }
        }
        is_loading.set(false);
        if saved {
            close_dialog();
        }
    };
    let delete = move |_| async move {
        let mut deleted = false;
        if let PasskeyDialog::Delete(passkey) = &*dialog.read() {
            is_loading.set(true);
            match DOMAIN_SERVICE().delete_passkey(user_id, passkey.id).await {
                Ok(()) => {
                    deleted = true;
                    signal_changed_data();
                }
                Err(err) => notify("Failed to delete passkey", &err),
            }
            is_loading.set(false);
        }
        if deleted {
            close_dialog();
        }
    };
    let close = move |_| close_dialog();

    rsx! {
        div {
            class: "mt-5",
            Title {
                actions: rsx! {
                    if matches!(&*passkeys.read(), Some(Ok(_))) {
                        a {
                            "data-testid": "add-passkey",
                            onclick: move |_| async move {
                                if is_loading() {
                                    return;
                                }
                                is_loading.set(true);
                                match DOMAIN_SERVICE().register_passkey().await {
                                    Ok(_) => signal_changed_data(),
                                    // Cancelling the ceremony is a normal user action, not a failure
                                    Err(domain::CreateError::Other(err))
                                        if storage::webauthn::Error::is_cancellation(
                                            err.as_ref(),
                                        ) => {}
                                    Err(err) => notify("Failed to register passkey", &err),
                                }
                                is_loading.set(false);
                            },
                            if is_loading() {
                                span {
                                    class: "icon is-small",
                                    i { class: "fas fa-spinner fa-pulse" }
                                }
                            } else {
                                Icon { name: "plus", is_small: true }
                            }
                        }
                    }
                },
                "Passkeys"
            }
        }
        match &*passkeys.read() {
            Some(Ok(passkeys)) => {
                rsx! {
                    if passkeys.is_empty() {
                        NoData { label: "No passkeys", "data-testid": "no-passkeys" }
                    } else {
                        PasskeyTable {
                            passkeys: passkeys.clone(),
                            action: move |passkey: domain::Passkey| rsx! {
                                ItemOptionsButton { on_click: move |_| { *dialog.write() = PasskeyDialog::Options(passkey.clone()); } }
                            },
                        }
                    }
                }
            }
            Some(Err(domain::ReadError::Storage(domain::StorageError::NoConnection))) => {
                rsx! {
                    NoConnection {}
                }
            }
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
        match &*dialog.read() {
            PasskeyDialog::None => rsx! {},
            PasskeyDialog::Options(passkey) => {
                let passkey_rename = passkey.clone();
                let passkey_delete = passkey.clone();
                let mut options = vec![
                    rsx! {
                        MenuOption {
                            icon: "edit".to_string(),
                            text: "Rename passkey".to_string(),
                            "data-testid": "options-rename-passkey",
                            on_click: move |_| {
                                *dialog.write() = PasskeyDialog::Rename {
                                    id: passkey_rename.id,
                                    label: FieldValue {
                                        input: passkey_rename.label.to_string(),
                                        validated: Ok(passkey_rename.label.clone()),
                                        orig: passkey_rename.label.to_string()
                                    },
                                };
                            }
                        },
                    },
                ];
                if !last_passkey_protected {
                    options.push(rsx! {
                        MenuOption {
                            icon: "trash".to_string(),
                            text: "Delete passkey".to_string(),
                            "data-testid": "options-delete-passkey",
                            on_click: move |_| { *dialog.write() = PasskeyDialog::Delete(passkey_delete.clone()); }
                        },
                    });
                }
                rsx! {
                    OptionsMenu {
                        options,
                        on_close: move |_| *dialog.write() = PasskeyDialog::None,
                    }
                }
            },
            PasskeyDialog::Rename { label, .. } => rsx! {
                SaveDialog {
                    title: rsx! { "Rename passkey" },
                    on_close: close,
                    on_save: save,
                    is_loading: is_loading(),
                    disabled: !label.changed() || !label.valid(),
                    InputField {
                        label: "Name".to_string(),
                        "data-testid": "passkey-name",
                        value: label.input.clone(),
                        error: if let Err(err) = &label.validated { err.clone() },
                        has_changed: label.changed(),
                        autofocus: true,
                        on_input: move |event: FormEvent| {
                            if let PasskeyDialog::Rename { label, .. } = &mut *dialog.write() {
                                label.input = event.value();
                                label.validated = domain::Name::new(&label.input)
                                    .map_err(|err| err.to_string());
                            }
                        }
                    }
                }
            },
            PasskeyDialog::Delete(passkey) => rsx! {
                DeleteConfirmationDialog {
                    element_type: "passkey".to_string(),
                    element_name: rsx! { "{passkey.label}" },
                    on_delete: delete,
                    on_cancel: close,
                    is_loading: is_loading(),
                }
            },
        }
    }
}

enum PasskeyDialog {
    None,
    Options(domain::Passkey),
    Rename {
        id: domain::PasskeyID,
        label: FieldValue<domain::Name>,
    },
    Delete(domain::Passkey),
}
