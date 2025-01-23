use crate::docker::{Invite, InviteParameters, ServerContainer, ServerContainerParameters};

use std::{fs, process};

use anyhow::{Context, Result};

use inquire::{
    error::CustomUserError,
    validator::{ErrorMessage, Validation},
    Confirm, CustomType, InquireError, Select, Text,
};

macro_rules! prompt {
    ($expr:expr) => {
        match $expr.prompt_skippable() {
            Err(e) if matches!(e, InquireError::OperationInterrupted) => process::exit(0),

            result => match result? {
                Some(value) => value,

                None => return Ok(None),
            },
        }
    };
}

fn file_path_validator(
    //
    input: &str,
) -> Result<Validation, CustomUserError> {
    Ok(
        //
        fs::metadata(input)
            //
            .map(|metadata| {
                if metadata
                    //
                    .file_type()
                    //
                    .is_file()
                {
                    Validation::Valid
                } else {
                    Validation::Invalid(ErrorMessage::Custom(
                        //
                        "This is not a file".to_string(),
                    ))
                }
            })
            //
            .unwrap_or_else(|e| {
                Validation::Invalid(ErrorMessage::Custom(
                    //
                    format!("Unable to access the file: {e}"),
                ))
            }),
    )
}

pub fn server_container_parameters() -> Result<Option<ServerContainerParameters>> {
    let crt_path = prompt!(
        //
        Text::new("Enter the path to your TLS certificate:")
            //
            .with_validator(file_path_validator)
    );

    let key_path = prompt!(
        //
        Text::new("Enter the path to your TLS key:")
            //
            .with_validator(file_path_validator)
    );

    let port = prompt!(
        //
        CustomType::<u16>::new("Enter the port for the HTTP server to listen at:")
            //
            .with_default(443)
    );

    Ok(
        //
        Some(
            //
            ServerContainerParameters {
                crt_path: fs::canonicalize(&crt_path)
                    //
                    .context("failed to canonicalize the TLS certificate path")?,

                key_path: fs::canonicalize(&key_path)
                    //
                    .context("failed to canonicalize the TLS key path")?,

                port,
            },
        ),
    )
}

#[derive(Clone)]
pub enum ServerContainerChoice {
    Selected(ServerContainer),

    CreateNewServer,

    Exit,
}

pub fn select_server_container(
    //
    containers: &[ServerContainer],
) -> Result<Option<ServerContainerChoice>> {
    let mut options = vec![];

    options.extend(
        //
        containers
            //
            .iter()
            //
            .map(|container| container.name.clone()),
    );

    options.push("Create a new server".to_string());

    options.push("Exit".to_string());

    let selection = prompt!(
        //
        Select::new(
            //
            "Select the server:",
            //
            options
        )
    );

    match selection.as_str() {
        "Create a new server" => Ok(Some(ServerContainerChoice::CreateNewServer)),

        "Exit" => Ok(Some(ServerContainerChoice::Exit)),

        _ => {
            let container = containers
                //
                .iter()
                //
                .find(|container| container.name == selection)
                //
                .cloned();

            Ok(
                //
                container.map(ServerContainerChoice::Selected),
            )
        }
    }
}

pub fn get_server_container_actions(
    //
    status: &str,
) -> Vec<String> {
    let mut options = vec![];

    options.push("Update".to_string());

    options.push("View logs".to_string());

    match status {
        "running" => {
            options.push("Manage invites".to_string());

            options.push("Restart".to_string());

            options.push("Stop".to_string());
        }

        "exited" | "created" | "dead" | "paused" => {
            options.push("Start".to_string());
        }

        _ => (),
    }

    options.push("Delete".to_string());

    options
}

pub fn select_invites_action() -> Result<Option<String>> {
    let options = vec![
        //
        "Create".to_string(),
        //
        "List".to_string(),
        //
        "Delete".to_string(),
        //
        "Go back".to_string(),
    ];

    let action = prompt!(
        //
        Select::new(
            //
            "Select an action:",
            //
            options
        )
    );

    Ok(Some(action))
}

pub fn select_invite_to_delete(
    //
    invites: &[Invite],
) -> Result<Option<String>> {
    let options = invites
        //
        .iter()
        //
        .map(|invite| invite.id.clone())
        //
        .collect::<Vec<_>>();

    if options.is_empty() {
        println!("No invites to delete.");

        return Ok(None);
    }

    let invite_id = prompt!(
        //
        Select::new(
            //
            "Select an invite to delete:",
            //
            options
        )
    );

    Ok(Some(invite_id))
}

pub fn invite_parameters() -> Result<Option<InviteParameters>> {
    let prompt = "Enter the number of uses (how many times the invite can be used) or leave blank for unlimited:";

    let uses = prompt!(
        //
        Text::new(prompt)
            //
            .with_validator(|input: &str| {
                if input
                    //
                    .trim()
                    //
                    .is_empty()
                {
                    Ok(Validation::Valid)
                } else {
                    match input
                        //
                        .trim()
                        //
                        .parse::<u32>()
                    {
                        Ok(0) => Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Cannot be zero".to_string(),
                        ))),

                        Ok(_) => Ok(Validation::Valid),

                        Err(_) => Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Please enter a valid number or leave blank".to_string(),
                        ))),
                    }
                }
            })
    );

    let uses = if uses
        //
        .trim()
        //
        .is_empty()
    {
        None
    } else {
        Some(
            uses
                //
                .trim()
                //
                .parse::<u32>()
                //
                .unwrap(),
        )
    };

    let prompt = "Enter permissions:";

    let perms = prompt!(
        //
        CustomType::<u32>::new(prompt)
            //
            .with_default(0)
    );

    Ok(
        //
        Some(
            //
            InviteParameters {
                //
                uses,
                //
                perms,
            },
        ),
    )
}

pub fn select_server_container_action(
    //
    status: &str,
) -> Result<Option<String>> {
    let mut options = get_server_container_actions(status);

    if options.is_empty() {
        return Ok(None);
    }

    options.push("Go back".to_string());

    let action = prompt!(
        //
        Select::new(
            //
            "Select an action:",
            //
            options
        )
    );

    Ok(Some(action))
}

pub fn confirm_delete_server_container(
    //
    server_name: &str,
) -> Result<Option<bool>> {
    let confirmation = prompt!(
        //
        Confirm::new(&format!(
            //
            "Are you sure you want to delete the server '{}'?",
            //
            server_name
        ))
        //
        .with_default(false)
    );

    Ok(Some(confirmation))
}
