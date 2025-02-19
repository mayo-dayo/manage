use crate::registry;

use anyhow::*;

use semver::*;

fn tag_to_version_or_none<T>(tag: T) -> Option<Version>
where
    T: AsRef<str>,
{
    let tag_as_ref = tag.as_ref();

    Version::parse(tag_as_ref).ok()
}

fn is_compatible_app_version(version: &Version) -> bool {
    // ^0.3
    const COMPARATOR: Comparator = Comparator {
        op: Op::Caret,

        major: 0,

        minor: Some(3),

        patch: None,

        pre: Prerelease::EMPTY,
    };

    COMPARATOR.matches(version)
}

pub fn tags_to_compatible_app_versions<I, T>(tags: I) -> impl Iterator<Item = Version>
where
    //
    I: IntoIterator<Item = T>,
    //
    T: AsRef<str>,
{
    tags
        //
        .into_iter()
        //
        .filter_map(tag_to_version_or_none)
        //
        .filter(is_compatible_app_version)
}

pub async fn get_latest_compatible_app_version() -> Result<Version> {
    let tags = registry::get_app_tags()
        //
        .await
        //
        .context("failed to get registry tags")?;

    let mut versions = tags_to_compatible_app_versions(tags).collect::<Vec<_>>();

    versions.sort_unstable();

    let latest_version = versions
        //
        .pop()
        //
        .expect("the registry should contain at least one compatible version");

    Ok(latest_version)
}

pub fn current_cli_version() -> Version {
    // 0.1.0
    Version {
        major: 0,

        minor: 1,

        patch: 1,

        pre: Prerelease::EMPTY,

        build: BuildMetadata::EMPTY,
    }
}

pub fn is_compatible_cli_version(version: &Version) -> bool {
    // ^0.1
    const COMPARATOR: Comparator = Comparator {
        op: Op::Caret,

        major: 0,

        minor: Some(1),

        patch: None,

        pre: Prerelease::EMPTY,
    };

    COMPARATOR.matches(version)
}
