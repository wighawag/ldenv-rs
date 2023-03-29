// mostly taken form dotenvy

use clap::Arg;
use std::os::unix::process::CommandExt;
use std::process;
use std::env;


macro_rules! die {
    ($fmt:expr) => ({
        eprintln!($fmt);
        process::exit(1);
    });
    ($fmt:expr, $($arg:tt)*) => ({
        eprintln!($fmt, $($arg)*);
        process::exit(1);
    });
}

fn make_command(name: &str, args: Vec<&str>) -> process::Command {
    let mut command = process::Command::new(name);

    for arg in args {
        command.arg(arg);
    }

    return command;
}

fn main() {
    

    let matches = clap::Command::new("dotenvy")
        .about("Run a command using the environment in a .env file")
        .override_usage("dotenvy <COMMAND> [ARGS]...")
        .allow_external_subcommands(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("MODE")
                .short('m')
                .long("mode")
                .takes_value(true)
                .help("mode: .env.<MODE>, default to local"),
        )
        .arg(
            Arg::new("MODE_NAME")
                .short('n')
                .long("modename")
                .takes_value(true)
                .help("modename: environment variable used as mode if none is provided on the command line, default to MODE"),
        )
        .get_matches();

    let mode_name = match matches.value_of("MODE_NAME") {
        None => "MODE",
        Some(mode_name) => mode_name,
    };

    // easier to test
    // for (n,_) in env::vars() {
    //     if !n.eq(mode_name) {
    //         env::remove_var(n)
    //     }
    // }

    let mode = match matches.value_of("MODE") {
        None => match env::var(mode_name) {
            Ok(str) => str,
            Err(_) => "local".to_string(),
        },
        Some(mode) => mode.to_string(),
    };

    
    if !mode.eq("local") {
        let mode_local_path = format!(".env.{}.local",mode);
        let mode_local_result = dotenvy::from_filename(&mode_local_path);
        match mode_local_result {
            Ok(_) => (),
            Err(error) => match error.not_found() {
                true => (),
                false => die!("error: failed to load local mode environment from {}: {}", mode_local_path, error)
            },
        };

        let mode_path = format!(".env.{}",mode);
        let mode_result = dotenvy::from_filename(&mode_path);
        match mode_result {
            Ok(_) => (),
            Err(error) => match error.not_found() {
                true => (),
                false => die!("error: failed to load mode environment from {}: {}", mode_path, error)
            },
        };

    }
    
    let env_local_path = ".env.local";
    let env_local_result = dotenvy::from_filename(&env_local_path);
    match env_local_result {
        Ok(_) => (),
        Err(error) => match error.not_found() {
            true => (),
            false => die!("error: failed to load local environment from {}: {}", env_local_path, error)
        },
    };
    

    let env_path = format!(".env");
    let env_result = dotenvy::from_filename(&env_path);
    match env_result {
        Ok(_) => (),
        Err(error) => match error.not_found() {
            true => (),
            false => die!("error: failed to load environment from {}: {}", env_path, error)
        },
    };


    let mut command = match matches.subcommand() {
        Some((name, matches)) => {
            let args = matches
                .values_of("")
                .map(|v| v.collect())
                .unwrap_or(Vec::new());

            make_command(name, args)
        }
        None => die!("error: missing required argument <COMMAND>"),
    };

    if cfg!(target_os = "windows") {
        match command.spawn().and_then(|mut child| child.wait()) {
            Ok(status) => process::exit(status.code().unwrap_or(1)),
            Err(error) => die!("fatal: {}", error),
        };
    } else {
        let error = command.exec();
        die!("fatal: {}", error);
    };
}