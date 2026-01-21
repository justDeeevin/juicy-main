#[cfg(feature = "clap")]
#[juicy_main::juicy]
fn main(args: Cli) {
    println!("Hello, {}!", args.name);
}

#[cfg(feature = "clap")]
struct Cli {
    name: String,
}

#[cfg(feature = "clap")]
impl clap::Parser for Cli {}

#[cfg(feature = "clap")]
impl clap::CommandFactory for Cli {
    fn command() -> clap::Command {
        clap::Command::new("name").arg(clap::arg!(--name <NAME>).required(true))
    }

    fn command_for_update() -> clap::Command {
        Self::command()
    }
}

#[cfg(feature = "clap")]
impl clap::FromArgMatches for Cli {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        Ok(Self {
            name: matches
                .get_one::<String>("name")
                .ok_or(<Self as clap::CommandFactory>::command().error(
                    clap::error::ErrorKind::MissingRequiredArgument,
                    "name was not provided",
                ))?
                .clone(),
        })
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        self.name = matches
            .get_one::<String>("name")
            .ok_or(<Self as clap::CommandFactory>::command().error(
                clap::error::ErrorKind::MissingRequiredArgument,
                "name was not provided",
            ))?
            .clone();
        Ok(())
    }
}

#[cfg(not(feature = "clap"))]
#[juicy_main::juicy]
fn main(args: Vec<String>) {
    println!("Hello, {}!", args[1]);
}
