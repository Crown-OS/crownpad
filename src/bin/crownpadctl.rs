use std::{process::ExitCode, time::Duration};

use crownpad::protocol::protocol;

const TIMEOUT: Duration = Duration::from_secs(2);
const USAGE: &str = "\
control the CrownOS launchpad

usage: crownpadctl <show|hide|toggle|status>

exits 0 if the launchpad ends up visible, 1 if hidden, 2 on error.";

enum Command {
    Show,
    Hide,
    Toggle,
    Status,
}

impl Command {
    fn parse(argument: &str) -> Option<Self> {
        match argument {
            "show" => Some(Self::Show),
            "hide" => Some(Self::Hide),
            "toggle" => Some(Self::Toggle),
            "status" => Some(Self::Status),
            _ => None,
        }
    }
}

fn main() -> ExitCode {
    let Some(command) = std::env::args().nth(1).as_deref().and_then(Command::parse) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(command) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("crownpadctl: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<bool, crownos_ipc::Error> {
    let mut pad = protocol::Client::connect()?;
    let pending = match command {
        Command::Show => pad.show()?,
        Command::Hide => pad.hide()?,
        Command::Toggle => pad.toggle()?,
        Command::Status => pad.is_visible()?,
    };
    pad.wait(pending, Some(TIMEOUT))
}
