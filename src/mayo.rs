use crate::labels::*;
use crate::registry;
use crate::server::Server;
use crate::server::parameters::Parameters;
use crate::versioning;

use std::collections::HashMap;

use anyhow::*;

use bollard::Docker;
use bollard::container;
use bollard::container::CreateContainerOptions;
use bollard::container::StartContainerOptions;
use bollard::container::ListContainersOptions;
use bollard::image::CreateImageOptions;
use bollard::image::ListImagesOptions;
use bollard::models::*;

use futures_util::StreamExt;

use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

use semver::Version;

pub struct Mayo {
    docker: Docker,
}

impl From<Docker> for Mayo {
    fn from(docker: Docker) -> Self {
        Self { docker }
    }
}

impl Mayo {
    pub fn try_new() -> Result<Self> {
        Docker::connect_with_defaults()
            //
            .map(Into::into)
            //
            .context("failed to connect to the Docker daemon")
    }

    async fn pull_image(&self, reference: &str) -> Result<()> {
        let options = CreateImageOptions {
            //
            from_image: reference,
            //
            ..Default::default()
        };

        let mut stream = self
            //
            .docker
            //
            .create_image(
                //
                Some(options),
                //
                None,
                //
                None,
            );

        let multi_progress = MultiProgress::new();

        let mut layers = HashMap::<String, ProgressBar>::default();

        while let Some(message_result) = stream.next().await {
            let CreateImageInfo {
                id,

                status,

                progress_detail,
                ..
            } = message_result
                //
                .context("failed to read a message")?;

            if let Some(id) = id {
                let progress_bar = layers
                    //
                    .entry(id.clone())
                    //
                    .or_insert_with(|| {
                        let progress_bar = ProgressBar::new(0);

                        progress_bar.set_style(
                            ProgressStyle::with_template(
                                "{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)",
                            )
                            //
                            .unwrap()
                            //
                            .progress_chars("##-"),
                        );

                        progress_bar.set_message(id.clone());

                        multi_progress.add(progress_bar)
                    });

                if let Some(status) = status {
                    let message = format!("{id}: {status}");

                    if let "Download complete" | "Pull complete" | "Already exists" = status.as_str() {
                        progress_bar.finish_with_message(message);
                    } else {
                        progress_bar.set_message(message);
                    }
                }

                if let Some(progress_detail) = progress_detail {
                    if let Some(total) = progress_detail.total {
                        progress_bar.set_length(total.try_into().unwrap());
                    }

                    if let Some(current) = progress_detail.current {
                        progress_bar.set_position(current.try_into().unwrap());
                    }
                }
            }
        }

        multi_progress
            //
            .clear()
            //
            .context("failed to clear multi progres")?;

        Ok(())
    }

    async fn find_existing_image_by_reference(&self, reference: &str) -> Result<Option<String>> {
        let mut filters = HashMap::default();

        filters.insert("reference", vec![reference]);

        let options = ListImagesOptions {
            all: false,

            filters,

            digests: false,
        };

        let mut response = self
            //
            .docker
            //
            .list_images(Some(options))
            //
            .await
            //
            .context("failed to list images")?;

        let image_id = response
            //
            .pop()
            //
            .map(|summary| summary.id);

        Ok(image_id)
    }

    async fn version_to_image(&self, version: &Version) -> Result<String> {
        // e.g. ghcr.io/mayo-dayo/app:0.2.0
        let reference = format!(
            //
            "{}/{}/{}:{}",
            //
            registry::REGISTRY,
            //
            registry::USERNAME,
            //
            registry::REPOSITORY,
            //
            version
        );

        loop {
            let image_id = self
                //
                .find_existing_image_by_reference(&reference)
                //
                .await
                //
                .context("failed to look for an existing image")?;

            if let Some(image_id) = image_id {
                return Ok(image_id);
            } else {
                self.pull_image(&reference)
                    //
                    .await
                    //
                    .context("failed to pull the image")?;
            }
        }
    }

    pub async fn create_server(&self, parameters: Parameters) -> Result<String> {
        let image_id = self
            //
            .version_to_image(&parameters.version)
            //
            .await
            //
            .context("failed to get the image")?;

        let mut labels = HashMap::<String, String>::default();

        labels.insert(
            //
            LABEL_KEY_CLI_VERSION.to_string(),
            //
            versioning::current_cli_version().to_string(),
        );

        labels.insert(
            //
            LABEL_KEY_PARAMETERS.to_string(),
            //
            parameters.encode_for_label(),
        );

        let Parameters {
            name,

            version: _,

            port,

            authentication,

            tls,
        } = parameters;

        const MAYO_DATA_PATH: &str = "/mayo/.data";

        let mut env = vec![
            //
            format!("BUN_PORT={port}"),
            //
            format!("MAYO_DATA_PATH={MAYO_DATA_PATH}"),
        ];

        if let Some((crt, key)) = tls.into_inner() {
            env.extend_from_slice(&[
                //
                format!("MAYO_TLS_CRT={crt}"),
                //
                format!("MAYO_TLS_KEY={key}"),
            ])
        }

        if authentication.is_required() {
            env.push("MAYO_AUTHENTICATION=required".to_string());
        }

        let mounts = vec![Mount {
            target: Some(MAYO_DATA_PATH.to_string()),

            source: Some(format!("mayo-{name}-volume")),

            typ: Some(MountTypeEnum::VOLUME),

            read_only: Some(false),

            ..Default::default()
        }];

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
            image: Some(image_id),

            env: Some(env),

            labels: Some(labels),

            host_config: Some(host_config),

            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: format!("mayo-{name}"),

            ..Default::default()
        };

        let ContainerCreateResponse {
            //
            id,
            ..
        } = self
            //
            .docker
            //
            .create_container(Some(options), config)
            //
            .await
            //
            .context("failed to create a container")?;

        self
            //
            .docker
            //
            .start_container(&id, None::<StartContainerOptions<String>>)
            //
            .await
            //
            .context("failed to start the container")?;

        Ok(id)
    }

    pub async fn list_servers(&self) -> Result<Vec<Server>> {
        let mut summaries = {
            let mut filters = HashMap::default();

            // filter out non-mayo containers
            filters.insert("label", vec![LABEL_KEY_CLI_VERSION]);

            let options = ListContainersOptions {
                all: true,

                limit: None,

                size: false,

                filters,
            };

            let response = self
                //
                .docker
                //
                .list_containers(Some(options))
                //
                .await
                //
                .context("failed to list containers")?;

            response
        };

        // filter out containers with incompatible cli version
        {
            summaries.retain(|summary| {
                let ContainerSummary {
                    //
                    labels,
                    ..
                } = summary;

                let is_cli_compatible = labels
                    //
                    .as_ref()
                    //
                    .map(|labels| {
                        labels
                            //
                            .get(LABEL_KEY_CLI_VERSION)
                            //
                            .map(|value| {
                                //
                                Version::parse(value)
                                    //
                                    .ok()
                            })
                            //
                            .flatten()
                            //
                            .map(|version| versioning::is_compatible_cli_version(&version))
                    })
                    //
                    .flatten()
                    //
                    .unwrap_or(false);

                is_cli_compatible
            });
        }

        // filter out containers with incompatible app version
        {
            let mut is_image_compatible = HashMap::<String, bool>::default();

            let mut index = 0;

            while let Some(summary) = summaries.get(index) {
                let ContainerSummary {
                    //
                    image_id,
                    ..
                } = summary;

                if let Some(image_id) = image_id {
                    if let Some(true) = is_image_compatible.get(image_id) {
                        index += 1;

                        continue;
                    }

                    let ImageInspect {
                        //
                        repo_tags,
                        ..
                    } = self
                        //
                        .docker
                        //
                        .inspect_image(&image_id)
                        //
                        .await
                        //
                        .context("failed to inspect an image")?;

                    if let Some(repo_tags) = repo_tags {
                        let tags = repo_tags
                            //
                            .iter()
                            //
                            .flat_map(|repo_tag| {
                                repo_tag
                                    //
                                    .split_once(':')
                                    //
                                    .map(|(_, tag)| tag)
                            });

                        let has_compatible_version = versioning::tags_to_compatible_app_versions(tags)
                            //
                            .next()
                            //
                            .is_some();

                        is_image_compatible.insert(
                            //
                            image_id.clone(),
                            //
                            has_compatible_version,
                        );

                        if has_compatible_version {
                            index += 1;

                            continue;
                        }
                    }
                }

                summaries.swap_remove(index);
            }
        }

        let servers = summaries
            //
            .into_iter()
            //
            .filter_map(|summary| Server::try_from(summary).ok())
            //
            .collect::<Vec<_>>();

        Ok(servers)
    }
}
