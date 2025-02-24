use anyhow::*;

use inquire::CustomType;
use inquire::MultiSelect;
use inquire::list_option::ListOption;

#[derive(Clone, Debug)]
pub struct Invite {
    pub perms: u16,

    pub uses: Option<u16>,
}

impl Invite {
    pub fn inquire() -> Result<Option<Self>> {
        let Some(uses) = CustomType::<u16>::new(
            "Please enter the maximum number times this invite can be used, or zero for no limit:",
        )
        //
        .with_default(1)
        //
        .prompt_skippable()
        //
        .context("failed to inquire uses")?
        else {
            return Ok(None);
        };

        let uses = if uses == 0 { Some(uses) } else { None };

        let options = vec![
            //
            ListOption::new(0, "Upload"),
            //
            ListOption::new(1, "Remove"),
        ];

        let Some(permissions) = MultiSelect::new("Please select permissions:", options)
            //
            .with_help_message("Users will receive these permissions after creating an account using this invite.")
            // 😎
            .with_vim_mode(true)
            //
            .prompt_skippable()
            //
            .context("failed to inquire permissions")?
        else {
            return Ok(None);
        };

        let perms = permissions
            //
            .into_iter()
            //
            .map(|permission| {
                match permission.index {
                    //
                    0 => 1 << 0,
                    //
                    1 => 1 << 1,
                    //
                    _ => unreachable!(),
                }
            })
            //
            .fold(0, |acc, flag| acc | flag);

        Ok(Some(Self { perms, uses }))
    }
}
