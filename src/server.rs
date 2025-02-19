use crate::labels::*;
use crate::parameters::Parameters;

use std::cmp::Ordering;

use bollard::models::ContainerSummary;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Server {
    pub id: String,

    pub state: String,

    pub parameters: Parameters,
}

impl Eq for Server {
    //
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for Server {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parameters.name.cmp(&other.parameters.name)
    }
}

impl PartialOrd for Server {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.parameters.name.partial_cmp(&other.parameters.name)
    }
}

impl TryFrom<ContainerSummary> for Server {
    type Error = ();

    fn try_from(summary: ContainerSummary) -> Result<Self, Self::Error> {
        let ContainerSummary {
            //
            id,
            //
            state,
            //
            labels,
            //
            ..
        } = summary;

        let id = id.ok_or(())?;

        let state = state.ok_or(())?;

        let parameters = labels
            //
            .map(|labels| {
                labels
                    //
                    .get(LABEL_KEY_PARAMETERS)
                    //
                    .map(|value| Parameters::try_from(value.as_str()).ok())
                    //
                    .flatten()
            })
            //
            .flatten()
            //
            .ok_or(())?;

        Ok(Self {
            //
            id,
            //
            state,
            //
            parameters,
        })
    }
}
