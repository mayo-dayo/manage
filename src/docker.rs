use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};

use bollard::{container, container::*, exec::*, image::*, models::*, volume::*, Docker};

use futures::StreamExt;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use serde::Deserialize;

const IMAGE: &str = "ghcr.io/mayo-dayo/app";

const VERSION: &str = "0.2";

const DATA_STORAGE_FOLDER_PATH: &str = "/mayo/.data";

#[derive(Clone, Debug)]
pub struct ServerContainer {
    pub id: String,

    pub name: String,

    pub crt_path: PathBuf,

    pub key_path: PathBuf,

    pub port: u16,
}

pub async fn find_existing_server_containers(
    //
    docker: &Docker,
) -> Result<Vec<ServerContainer>> {
    let mut filters = HashMap::new();

    filters.insert(
        //
        "ancestor".to_string(),
        //
        vec![format!("{}:{}", IMAGE, VERSION)],
    );

    filters.insert(
        //
        "label".to_string(),
        //
        vec![format!("version={}", VERSION), "name".to_string()],
    );

    let options = Some(ListContainersOptions {
        all: true,

        filters,

        ..Default::default()
    });

    let container_summaries = docker
        //
        .list_containers(options)
        //
        .await?;

    let mut containers = Vec::new();

    for container_summary in container_summaries {
        let ContainerSummary {
            //
            id,
            //
            labels,
            //
            ..
        } = container_summary;

        if let (
            //
            Some(id),
            //
            Some(mut labels),
        ) = (id, labels)
        {
            if let (
                //
                Some(name),
                //
                Some(crt_path_str),
                //
                Some(key_path_str),
                //
                Some(port_str),
            ) = (
                //
                labels.remove("name"),
                //
                labels.remove("crt_path"),
                //
                labels.remove("key_path"),
                //
                labels.remove("port"),
            ) {
                let crt_path = PathBuf::from(crt_path_str);

                let key_path = PathBuf::from(key_path_str);

                if let Ok(port) = port_str.parse::<u16>() {
                    containers.push(ServerContainer {
                        id,

                        name,

                        crt_path,

                        key_path,

                        port,
                    });
                }
            }
        }
    }

    Ok(containers)
}

pub async fn pull_server_image(
    //
    docker: &Docker,
) -> Result<()> {
    let multi_progress = MultiProgress::new();

    let mut layers = HashMap::new();

    let mut stream = docker
        //
        .create_image(
            //
            Some(
                //
                CreateImageOptions {
                    from_image: format!("{IMAGE}:{VERSION}"),

                    ..Default::default()
                },
            ),
            //
            None,
            //
            None,
        );

    while let Some(message_result) = stream.next().await {
        let CreateImageInfo {
            //
            id,
            //
            progress_detail,
            //
            status,
            //
            ..
        } = message_result?;

        if let Some(layer_id) = id {
            let progress_bar = layers
                //
                .entry(layer_id.clone())
                //
                .or_insert_with(|| {
                    let progress_bar = multi_progress.add(ProgressBar::new(0));

                    progress_bar.set_style(
                        ProgressStyle::default_spinner()
                            //
                            .template("{msg}")
                            //
                            .unwrap(),
                    );

                    progress_bar.set_message(layer_id.clone());

                    progress_bar
                });

            if let Some(progress_detail) = progress_detail {
                if let (
                    //
                    Some(current),
                    //
                    Some(total),
                ) = (
                    //
                    progress_detail.current,
                    //
                    progress_detail.total,
                ) {
                    if total > 0 {
                        let total_u64 = total as u64;

                        if progress_bar.length().unwrap_or(0) != total_u64 {
                            progress_bar.set_length(total_u64);

                            progress_bar.set_style(
                                ProgressStyle::default_bar()
                                    //
                                    .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                                    //
                                    .unwrap()
                                    //
                                    .progress_chars("##-"),
                            );
                        }

                        if current >= 0 {
                            progress_bar.set_position(
                                current
                                    //
                                    .try_into()
                                    //
                                    .unwrap(),
                            );
                        }
                    }
                }
            }

            if let Some(status) = status {
                match status.as_str() {
                    "Download complete" | "Pull complete" | "Already exists" => {
                        progress_bar.finish_with_message(
                            //
                            format!("{}: {}", layer_id, status),
                        );
                    }

                    _ => {
                        progress_bar.set_message(
                            //
                            format!("{}: {}", layer_id, status),
                        );
                    }
                }
            }
        }
    }

    multi_progress.clear()?;

    Ok(())
}

#[derive(Clone, Debug)]
pub struct ServerContainerParameters {
    pub crt_path: PathBuf,

    pub key_path: PathBuf,

    pub port: u16,
}

pub async fn create_server_container(
    docker: &Docker,

    parameters: &ServerContainerParameters,

    name: Option<&str>,
) -> Result<ServerContainer> {
    let ServerContainerParameters {
        crt_path,

        key_path,

        port,
    } = parameters;

    let name = name.map(ToString::to_string).unwrap_or_else(|| {
        //
        names::Generator::default()
            //
            .next()
            //
            .unwrap()
    });

    let mut labels = HashMap::new();

    labels.insert(
        //
        "name".to_string(),
        //
        name.clone(),
    );

    labels.insert(
        //
        "version".to_string(),
        //
        VERSION.to_string(),
    );

    labels.insert(
        //
        "crt_path".to_string(),
        //
        crt_path.to_string_lossy().into_owned(),
    );

    labels.insert(
        //
        "key_path".to_string(),
        //
        key_path.to_string_lossy().into_owned(),
    );

    labels.insert(
        //
        "port".to_string(),
        //
        port.to_string(),
    );

    let env = vec![
        //
        format!("BUN_PORT={port}"),
        //
        format!("MAYO_DATA_PATH={DATA_STORAGE_FOLDER_PATH}"),
        //
        format!("TLS_CRT_PATH={DATA_STORAGE_FOLDER_PATH}/crt.pem"),
        //
        format!("TLS_KEY_PATH={DATA_STORAGE_FOLDER_PATH}/key.pem"),
    ];

    let mounts = vec![
        //
        Mount {
            target: Some(format!("{DATA_STORAGE_FOLDER_PATH}/crt.pem")),

            source: Some(crt_path.to_string_lossy().into_owned()),

            typ: Some(MountTypeEnum::BIND),

            read_only: Some(true),

            ..Default::default()
        },
        //
        Mount {
            target: Some(format!("{DATA_STORAGE_FOLDER_PATH}/key.pem")),

            source: Some(key_path.to_string_lossy().into_owned()),

            typ: Some(MountTypeEnum::BIND),

            read_only: Some(true),

            ..Default::default()
        },
        //
        Mount {
            target: Some(DATA_STORAGE_FOLDER_PATH.to_string()),

            source: Some(format!("mayo-{name}")),

            typ: Some(MountTypeEnum::VOLUME),

            read_only: Some(false),

            ..Default::default()
        },
    ];

    let host_config = HostConfig {
        mounts: Some(mounts),

        network_mode: Some("host".to_string()),

        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),

            maximum_retry_count: None,
        }),

        ..Default::default()
    };

    let config = container::Config {
        image: Some(format!("{IMAGE}:{VERSION}")),

        env: Some(env),

        labels: Some(labels),

        host_config: Some(host_config),

        ..Default::default()
    };

    let options = Some(CreateContainerOptions {
        name: format!("mayo-{}", name),

        ..Default::default()
    });

    let ContainerCreateResponse {
        //
        id,
        //
        ..
    } = docker.create_container(options, config).await?;

    docker
        //
        .start_container(&id, None::<StartContainerOptions<String>>)
        //
        .await?;

    Ok(
        //
        ServerContainer {
            //
            id,

            //
            name,

            //
            crt_path: crt_path.clone(),

            //
            key_path: key_path.clone(),

            //
            port: *port,
        },
    )
}

pub async fn get_container_status(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<String> {
    let container_info = docker
        //
        .inspect_container(container_id, None::<InspectContainerOptions>)
        //
        .await
        //
        .context("failed to inspect the container")?;

    let status = container_info
        //
        .state
        //
        .as_ref()
        //
        .and_then(|state| {
            state
                .status
                //
                .as_ref()
                //
                .map(|s| s.to_string())
        })
        //
        .unwrap_or_else(|| "unknown".to_string());

    Ok(status)
}

pub async fn restart_container(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<()> {
    docker
        //
        .restart_container(container_id, None::<RestartContainerOptions>)
        //
        .await?;

    Ok(())
}

pub async fn stop_container(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<()> {
    docker
        //
        .stop_container(container_id, None::<StopContainerOptions>)
        //
        .await?;

    Ok(())
}

pub async fn start_container(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<()> {
    docker
        //
        .start_container(container_id, None::<StartContainerOptions<String>>)
        //
        .await?;

    Ok(())
}

pub async fn remove_container(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<()> {
    let options = Some(RemoveContainerOptions {
        //
        force: true,
        //
        ..Default::default()
    });

    docker
        //
        .remove_container(container_id, options)
        //
        .await?;

    Ok(())
}

pub async fn remove_server_container(
    //
    docker: &Docker,
    //
    server_container: &ServerContainer,
) -> Result<()> {
    remove_container(
        //
        docker,
        //
        &server_container.id,
    )
    //
    .await
    //
    .context("failed to remove the container")?;

    let volume_name = format!(
        //
        "mayo-{}",
        //
        server_container.name
    );

    let options = Some(RemoveVolumeOptions { force: true });

    docker
        //
        .remove_volume(
            //
            &volume_name,
            //
            options,
        )
        //
        .await
        //
        .context("failed to remove the volume")?;

    Ok(())
}

pub async fn get_container_logs(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<String> {
    let options = Some(LogsOptions::<String> {
        //
        stdout: true,
        //
        stderr: true,
        //
        follow: false,
        //
        timestamps: false,
        //
        tail: "all".to_string(),
        //
        ..Default::default()
    });

    let mut logs_stream = docker.logs(
        //
        container_id,
        //
        options,
    );

    let mut logs = String::new();

    while let Some(log) = logs_stream
        //
        .next()
        //
        .await
    {
        match log? {
            LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                logs.push_str(
                    //
                    &String::from_utf8_lossy(&message),
                );
            }

            _ => (),
        }
    }

    Ok(logs)
}
pub async fn create_invite(
    //
    docker: &Docker,
    //
    container_id: &str,
    //
    uses: Option<u32>,
    //
    perms: u32,
) -> Result<String> {
    let uses = match uses {
        Some(value) => value.to_string(),

        None => "null".to_string(),
    };

    let script = format!(
        r#"
import {{ Database }} from "bun:sqlite";
const database =
  //
  new Database(
    "{DATA_STORAGE_FOLDER_PATH}/db.sqlite"
  );
const id =
  //
  Bun.randomUUIDv7();
database
  //
  .query(`
    insert into

      invites

    (
      'id',

      'uses',

      'perms'
    )

    values

    (
      ?1,

      ?2,

      ?3
    );
  `)
  //
  .run(
    //
    id,
    //
    {uses},
    //
    {perms}
  );
console.write(
  id
);
    "#
    );

    let exec_create_result = docker
        //
        .create_exec(
            //
            container_id,
            //
            CreateExecOptions {
                //
                cmd: Some(vec!["bun", "-e", &script]),
                //
                attach_stdout: Some(true),
                //
                attach_stderr: Some(true),
                //
                ..Default::default()
            },
        )
        //
        .await?;

    let exec_stream = docker
        //
        .start_exec(
            //
            &exec_create_result.id,
            //
            None::<StartExecOptions>,
        )
        //
        .await?;

    match exec_stream {
        StartExecResults::Attached { mut output, .. } => {
            let mut output_vec = Vec::new();

            while let Some(message) = output
                //
                .next()
                //
                .await
            {
                match message? {
                    LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                        output_vec.extend_from_slice(&message);
                    }

                    _ => (),
                }
            }

            let invite_id = String::from_utf8_lossy(&output_vec)
                //
                .trim()
                //
                .to_string();

            Ok(invite_id)
        }

        StartExecResults::Detached => Err(anyhow!("exec process detached")),
    }
}

#[derive(Clone, Debug)]
pub struct InviteParameters {
    pub uses: Option<u32>,

    pub perms: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Invite {
    pub id: String,

    pub uses: Option<u32>,

    pub perms: u32,
}

pub async fn get_invites(
    //
    docker: &Docker,
    //
    container_id: &str,
) -> Result<Vec<Invite>> {
    let script = format!(
        r#"
import {{ Database }} from "bun:sqlite";

const database =
  //
  new Database("{DATA_STORAGE_FOLDER_PATH}/db.sqlite");

const invites = database
  //
  .query(`
    select id, uses, perms from invites;
  `)
  //
  .all();

console.write(JSON.stringify(invites));
        "#
    );

    let exec_create_result = docker
        //
        .create_exec(
            //
            container_id,
            //
            CreateExecOptions {
                //
                cmd: Some(vec!["bun", "-e", &script]),
                //
                attach_stdout: Some(true),
                //
                attach_stderr: Some(true),
                //
                ..Default::default()
            },
        )
        //
        .await?;

    let exec_stream = docker
        //
        .start_exec(
            //
            &exec_create_result.id,
            //
            None::<StartExecOptions>,
        )
        //
        .await?;

    match exec_stream {
        StartExecResults::Attached { mut output, .. } => {
            let mut output_vec = Vec::new();

            while let Some(message) = output
                //
                .next()
                //
                .await
            {
                match message? {
                    LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                        output_vec.extend_from_slice(&message);
                    }
                    _ => (),
                }
            }

            let output_str = String::from_utf8_lossy(&output_vec);

            let invites = serde_json::from_str::<Vec<Invite>>(&output_str)
                //
                .context("failed to parse invites")?;

            Ok(invites)
        }
        StartExecResults::Detached => Err(anyhow!("exec process detached")),
    }
}

pub async fn delete_invite(
    //
    docker: &Docker,
    //
    container_id: &str,
    //
    invite_id: &str,
) -> Result<()> {
    let script = format!(
        r#"
import {{ Database }} from "bun:sqlite";

const database =
  //
  new Database("{DATA_STORAGE_FOLDER_PATH}/db.sqlite");

database
  //
  .query(`
    delete from invites where id = ?1;
  `)
  //
  .run(
    //
    "{invite_id}"
  );
        "#
    );

    let exec_create_result = docker
        //
        .create_exec(
            //
            container_id,
            //
            CreateExecOptions {
                //
                cmd: Some(vec!["bun", "-e", &script]),
                //
                attach_stdout: Some(true),
                //
                attach_stderr: Some(true),
                //
                ..Default::default()
            },
        )
        //
        .await?;

    let exec_stream = docker
        //
        .start_exec(
            //
            &exec_create_result.id,
            //
            None::<StartExecOptions>,
        )
        //
        .await?;

    match exec_stream {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(message) = output
                //
                .next()
                //
                .await
            {
                let _ = message?;
            }

            Ok(())
        }
        StartExecResults::Detached => Err(anyhow!("exec process detached")),
    }
}

pub async fn update_container(
    //
    docker: &Docker,
    //
    server_container: &ServerContainer,
) -> Result<ServerContainer> {
    remove_container(
        //
        docker,
        //
        &server_container.id,
    )
    //
    .await?;

    pull_server_image(
        //
        docker,
    )
    //
    .await?;

    let parameters = ServerContainerParameters {
        crt_path: server_container.crt_path.clone(),

        key_path: server_container.key_path.clone(),

        port: server_container.port,
    };

    let new_server_container = create_server_container(
        //
        docker,
        //
        &parameters,
        //
        Some(&server_container.name),
    )
    //
    .await?;

    Ok(new_server_container)
}
