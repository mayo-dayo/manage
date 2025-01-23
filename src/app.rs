use crate::{
    docker::{self, ServerContainer},
    inquire::{self, ServerContainerChoice},
};

use anyhow::{Context, Result};

use bollard::Docker;

use console::Term;

pub struct App {
    docker: Docker,

    term: Term,
}

impl App {
    pub fn try_init() -> Result<Self> {
        let docker = Docker::connect_with_defaults()
            //
            .context("failed to connect to the Docker daemon")?;

        let term = Term::stdout();

        Ok(
            //
            Self {
                //
                docker,
                //
                term,
            },
        )
    }

    pub async fn start(&self) -> Result<()> {
        loop {
            let containers = docker::find_existing_server_containers(&self.docker)
                //
                .await
                //
                .context("failed to find existing server containers")?;

            let action = if containers.is_empty() {
                let Some(mut server_container) = self
                    //
                    .create_server_container()
                    //
                    .await?
                else {
                    return Ok(());
                };

                self.select_server_container_action(&mut server_container)
                    //
                    .await?
            } else {
                self.select_server_container(&containers)
                    //
                    .await?
            };

            match action {
                AppAction::Exit => {
                    return Ok(());
                }

                AppAction::GoBack => {
                    continue;
                }
            }
        }
    }

    async fn create_server_container(&self) -> Result<Option<ServerContainer>> {
        let _ = self.term.clear_screen();

        docker::pull_server_image(
            //
            &self.docker,
        )
        //
        .await
        //
        .context("failed to pull the Docker image")?;

        let _ = self.term.clear_screen();

        println!("Let's create a new server.");

        let Some(parameters) = inquire::server_container_parameters()
            //
            .context("failed to inquire server container parameters")?
        else {
            return Ok(None);
        };

        let server_container = docker::create_server_container(
            //
            &self.docker,
            //
            &parameters,
            //
            None,
        )
        //
        .await
        //
        .context("failed to create the server container")?;

        Ok(Some(server_container))
    }

    async fn select_server_container(
        //
        &self,
        //
        containers: &[ServerContainer],
    ) -> Result<AppAction> {
        let Some(choice) = inquire::select_server_container(
            //
            containers,
        )
        //
        .context("failed to select a server container")?
        else {
            return Ok(AppAction::Exit);
        };

        match choice {
            ServerContainerChoice::Selected(mut server_container) => {
                self.select_server_container_action(
                    //
                    &mut server_container,
                )
                .await
            }

            ServerContainerChoice::CreateNewServer => {
                let Some(mut server_container) = self
                    .create_server_container()
                    //
                    .await?
                else {
                    return Ok(AppAction::GoBack);
                };

                self.select_server_container_action(&mut server_container)
                    //
                    .await
            }

            ServerContainerChoice::Exit => Ok(AppAction::Exit),
        }
    }

    async fn select_server_container_action(
        //
        &self,
        //
        server_container: &mut ServerContainer,
    ) -> Result<AppAction> {
        loop {
            let status = docker::get_container_status(
                //
                &self.docker,
                //
                &server_container.id,
            )
            //
            .await
            //
            .context("failed to get server container status")?;

            let _ = self.term.clear_screen();

            println!(
                //
                "Server {} is currently {}.",
                //
                server_container.name,
                //
                status
            );

            let Some(action) = inquire::select_server_container_action(
                //
                &status,
            )
            //
            .context("failed to select an action")?
            else {
                let _ = self.term.clear_screen();
                return Ok(AppAction::GoBack);
            };

            match action.as_str() {
                "View logs" => {
                    let logs = docker::get_container_logs(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to get the container logs")?;

                    let _ = self.term.clear_screen();

                    println!("{}", logs);

                    println!("\nPress Enter to go back...");

                    let _ = self.term.read_line();

                    let _ = self.term.clear_screen();
                }

                "Manage invites" => {
                    let _ = self.term.clear_screen();

                    self.select_invites_action(
                        //
                        server_container,
                    )
                    .await?;
                }

                "Restart" => {
                    docker::restart_container(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to restart container")?;
                }

                "Stop" => {
                    docker::stop_container(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to stop container")?;
                }

                "Start" => {
                    docker::start_container(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to start container")?;
                }

                "Update" => {
                    let new_server_container = docker::update_container(
                        //
                        &self.docker,
                        //
                        &server_container,
                    )
                    //
                    .await
                    //
                    .context("failed to update container")?;

                    *server_container = new_server_container;
                }
                "Delete" => {
                    let Some(confirmed) = inquire::confirm_delete_server_container(
                        //
                        &server_container.name,
                    )
                    //
                    .context("failed to confirm deletion")?
                    else {
                        let _ = self.term.clear_screen();

                        return Ok(AppAction::GoBack);
                    };

                    if confirmed {
                        docker::remove_server_container(
                            //
                            &self.docker,
                            //
                            &server_container,
                        )
                        //
                        .await?;

                        let _ = self.term.clear_screen();

                        return Ok(AppAction::GoBack);
                    }
                }

                "Go back" => {
                    let _ = self.term.clear_screen();

                    return Ok(AppAction::GoBack);
                }

                _ => (),
            }
        }
    }

    async fn select_invites_action(
        //
        &self,
        //
        server_container: &ServerContainer,
    ) -> Result<()> {
        let _ = self.term.clear_screen();

        loop {
            let Some(action) = inquire::select_invites_action()
                //
                .context("failed to select an invites action")?
            else {
                return Ok(());
            };

            match action.as_str() {
                "Create" => {
                    let Some(parameters) = inquire::invite_parameters()
                        //
                        .context("failed to inquire invite parameters")?
                    else {
                        return Ok(());
                    };

                    let invite_id = docker::create_invite(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                        //
                        parameters.uses,
                        //
                        parameters.perms,
                    )
                    //
                    .await
                    //
                    .context("failed to create an invite")?;

                    let _ = self.term.clear_screen();

                    println!("Invite created:");

                    println!("{}", invite_id);

                    println!("\nPress Enter to go back...");

                    let _ = self.term.read_line();

                    let _ = self.term.clear_screen();
                }

                "List" => {
                    let invites = docker::get_invites(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to get invites")?;

                    let _ = self.term.clear_screen();

                    if invites.is_empty() {
                        println!("No invites found.");
                    } else {
                        println!("Invites:");

                        for invite in &invites {
                            println!(
                                //
                                "ID: {}, uses: {:?}, perms: {}",
                                //
                                invite.id,
                                //
                                invite.uses,
                                //
                                invite.perms
                            );
                        }
                    }

                    println!("\nPress Enter to go back...");

                    let _ = self.term.read_line();

                    let _ = self.term.clear_screen();
                }

                "Delete" => {
                    let invites = docker::get_invites(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                    )
                    //
                    .await
                    //
                    .context("failed to get invites")?;

                    let Some(invite_id) = inquire::select_invite_to_delete(
                        //
                        &invites,
                    )
                    //
                    .context("failed to select an invite to delete")?
                    else {
                        return Ok(());
                    };

                    docker::delete_invite(
                        //
                        &self.docker,
                        //
                        &server_container.id,
                        //
                        &invite_id,
                    )
                    //
                    .await
                    //
                    .context("failed to delete invite")?;

                    let _ = self.term.clear_screen();

                    println!("Invite deleted.");

                    println!("\nPress Enter to go back...");

                    let _ = self.term.read_line();

                    let _ = self.term.clear_screen();
                }

                "Go back" => {
                    let _ = self.term.clear_screen();

                    return Ok(());
                }

                _ => (),
            }
        }
    }
}

pub enum AppAction {
    Exit,

    GoBack,
}
