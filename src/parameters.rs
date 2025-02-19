use crate::labels::*;
use crate::versioning;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::fs;

use anyhow::*;

use inquire::Confirm;
use inquire::CustomType;
use inquire::error::CustomUserError;
use inquire::parser::CustomTypeParser;
use inquire::validator::Validation;

use names::Generator;

use rustls_pemfile::Item::*;

use semver::Version;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Parameters {
    pub name: Name,

    pub version: Version,

    pub port: Port,

    pub authentication: Authentication,

    pub tls: Tls,
}

impl TryFrom<HashMap<String, String>> for Parameters {
    type Error = ();

    fn try_from(labels: HashMap<String, String>) -> Result<Self, Self::Error> {
        labels
            //
            .get(LABEL_KEY_PARAMETERS)
            //
            .map(|value| serde_json::from_str::<Parameters>(value).ok())
            //
            .flatten()
            //
            .ok_or(())
    }
}

impl Parameters {
    pub async fn inquire() -> Result<Option<Self>> {
        let Some(port) = Port::inquire()?
        //
        else {
            return Ok(None);
        };

        let Some(authentication) = Authentication::inquire()?
        //
        else {
            return Ok(None);
        };

        let Some(tls) = Tls::inquire()?
        //
        else {
            return Ok(None);
        };

        let version = versioning::get_latest_compatible_app_version()
            //
            .await
            //
            .context("failed to get the latest compatible version")?;

        let name = Name::generate();

        Ok(Some(Self {
            name,

            version,

            port,

            authentication,

            tls,
        }))
    }

    pub fn encode_for_label(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Name(String);

impl Eq for Name {
    //
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Name {
    pub fn generate() -> Self {
        let name = Generator::default()
            //
            .next()
            //
            .unwrap();

        Self(name)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Port(u16);

impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Port {
    pub fn inquire() -> Result<Option<Self>> {
        CustomType::<u16>::new("Which port would you like the server to use?")
            //
            .with_default(8080)
            //
            .prompt_skippable()
            //
            .context("failed to inquire the port")
            //
            .map(|option| {
                option
                    //
                    .map(Self)
            })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Authentication(bool);

impl fmt::Display for Authentication {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 {
            f.write_str("required")
        } else {
            f.write_str("optional")
        }
    }
}

impl Authentication {
    pub fn inquire() -> Result<Option<Self>> {
        Confirm::new("Would you like to disable mandatory authentication?")
            //
            .with_default(false)
            //
            .with_help_message("Users will be able to browse and stream the audio without authenticating.")
            //
            .prompt_skippable()
            //
            .context("failed to inquire the authentication")
            //
            .map(|option| {
                option
                    //
                    .map(|value| Self(!value))
            })
    }

    pub fn is_required(&self) -> bool {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tls(Option<(String, String)>);

impl fmt::Display for Tls {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_some() {
            f.write_str("enabled")
        } else {
            f.write_str("disabled")
        }
    }
}

impl Tls {
    pub fn inquire() -> Result<Option<Self>> {
        let Some(confirmed) = Confirm::new("Would you like the server to use TLS?")
            //
            .with_default(true)
            //
            .prompt_skippable()
            //
            .context("failed to inquire the TLS confirmation")?
        //
        else {
            return Ok(None);
        };

        let mut result = Self(None);

        if confirmed {
            let file_content_parser: CustomTypeParser<String> = &|file_path| {
                fs::read_to_string(
                    //
                    file_path,
                )
                //
                .map_err(|_| ())
            };

            let crt_validator = |input: &String| -> ::std::result::Result<Validation, CustomUserError> {
                ::std::result::Result::Ok(
                    //
                    rustls_pemfile::read_one_from_slice(
                        //
                        input.as_bytes(),
                    )
                    //
                    .map(|item| {
                        if let Some((
                            //
                            X509Certificate(_),
                            //
                            _,
                        )) = item
                        {
                            Validation::Valid
                        } else {
                            Validation::Invalid("Not a certificate 😣".into())
                        }
                    })
                    //
                    .unwrap_or_else(|_| Validation::Invalid("Not a PEM file 😵".into())),
                )
            };

            let Some(crt) = CustomType::<String>::new("Please enter the path to your certificate:")
                //
                .with_parser(file_content_parser)
                //
                .with_validator(crt_validator)
                //
                .with_error_message("Couldn't read from this file 😵‍💫")
                //
                .prompt_skippable()
                //
                .context("failed to inquire the tls certificate path")?
            //
            else {
                return Ok(None);
            };

            let key_validator = |input: &String| -> ::std::result::Result<Validation, CustomUserError> {
                ::std::result::Result::Ok(
                    //
                    rustls_pemfile::read_one_from_slice(
                        //
                        input.as_bytes(),
                    )
                    //
                    .map(|item| {
                        if let Some((
                            //
                            Pkcs1Key(_) | Pkcs8Key(_) | Sec1Key(_),
                            //
                            _,
                        )) = item
                        {
                            Validation::Valid
                        } else {
                            Validation::Invalid("Not a private key 😣".into())
                        }
                    })
                    //
                    .unwrap_or_else(|_| Validation::Invalid("Not a PEM file 😵".into())),
                )
            };

            let Some(key) = CustomType::<String>::new("Please enter the path to your private key:")
                //
                .with_parser(file_content_parser)
                //
                .with_validator(key_validator)
                //
                .with_error_message("Couldn't read from this file 😵‍💫 Does it exist 🤔")
                //
                .prompt_skippable()
                //
                .context("failed to inquire the tls private key path")?
            //
            else {
                return Ok(None);
            };

            result = Self(Some((crt, key)));
        }

        Ok(Some(result))
    }

    pub fn into_inner(self) -> Option<(String, String)> {
        self.0
    }
}
