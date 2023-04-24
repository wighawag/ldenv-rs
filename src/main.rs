// mostly taken form dotenvy

use clap::Arg;
use std::process;
use std::env;


#[cfg(not(target_os = "windows"))]
use std::os::unix::process::CommandExt;


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

fn make_command(name: &str, args: Vec<String>) -> process::Command {
    let mut command = process::Command::new(name);

    for arg in args {
        command.arg(arg);
    }

    return command;
}

fn load_env_mode_name() -> String {
    
    if let Ok(vars) = dotenvy::from_filename_iter(".env.local") {
        for item in vars {
            if let Ok(item) = item {
                let (key, val) = item;
                if key.eq("ENV_MODE") {
                    return val;
                }
            }
        }
    }

    if let Ok(vars) = dotenvy::from_filename_iter(".env") {
        for item in vars {
            if let Ok(item) = item {
                let (key, val) = item;
                if key.eq("ENV_MODE") {
                    return val;
                }
            }
        }
    }

    return env::var("ENV_MODE").unwrap_or("MODE".to_string())
}

fn main() {

    let matches = clap::Command::new("ldenv")
        .about("Run a command using the environment from .env, .env.local (, env.<mode> and env.<mode>.local) files\n It also parse the command to replace `@@<name>[:@:<default>]` with their env value. Fails if not present and no default provided.\n Note that you can also provide prefix and suffix: `[prefix]@@<name>[:@:<default>][:@:suffix]`")
        .override_usage("ldenv [OPTIONS] <COMMAND> [...COMMAND_ARGS]")
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
            Arg::new("ENV_MODE_NAME")
                .short('n')
                .long("env-mode-name")
                .takes_value(true)
                .help("env-mode-name: environment variable used as mode if none is provided on the command line, default to MODE or the value of env ENV_MODE"),
        )
        .arg(
            Arg::new("NO_PARSE")
                .short('P')
                .long("no-parse")
                .help("no-parse: if set do not parse the args passed in. ").action(clap::ArgAction::SetTrue),
        )
        .get_matches();

        
    let parse = !matches.get_flag("NO_PARSE");


    let env_mode_name = match matches.value_of("ENV_MODE_NAME") {
        None => load_env_mode_name(),
        Some(env_mode_name) => env_mode_name.to_string(),
    };

    // easier to test
    // for (n,_) in env::vars() {
    //     if !n.eq(env_mode_name) {
    //         env::remove_var(n)
    //     }
    // }

    let mode = match matches.value_of("MODE") {
        // we do not read MODE from env file as this will be used to get which file to load
        None => match env::var(&env_mode_name) {
            Ok(str) => str,
            Err(_) => "local".to_string(),
        },
        Some(mode) => mode.to_string(),
    };

    // we also set the mode as env variable
    env::set_var(&env_mode_name, &mode);

    
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

            let args: Vec<String> = args.iter()
            .map(|&s| 
                if parse {
                    // TODO support multiple @@ in one arg
                    // use loop over find ? to get all index
                    if let Some(i) = s.find("@@") {
                        let splitted: Vec<&str> = s[i+2..].split(":@:").collect();
                        let var_name = splitted[0];
                        if var_name.len() == 0 {
                            die!("error: the empty '@@{}' in '{}' is not valid, please specify an ENV var name after the '@@'", var_name, s);
                        }
                        let default_value = if splitted.len() > 1 {
                            Some(splitted[1])
                        } else {
                            None
                        };
                        let arg = if let Ok(v) = env::var(var_name) {
                            v
                        } else {
                            match default_value {
                                Some(v) => v.to_string(),
                                None => die!("error: {} was specified in the command but there is no env variable named {}. you can provide an empty default value with '@@{}:@:<default value>' including an empty default via '@@{}:@:'", s, var_name, var_name, var_name)
                            }
                        };
                        let arg = if splitted.len() > 2 {
                            format!("{}{}", arg, splitted[2])
                        } else {
                            arg
                        };
                        if i > 0 {
                            let prefix = &s[..i];
                            format!("{}{}", prefix, arg)
                        } else {
                            arg
                        }
                    } else {
                        s.to_string()
                    }
                } else {
                    s.to_string()
                }
            ).collect();
            make_command(name, args)
        }
        None => die!("error: missing required argument <COMMAND>"),
    };

    #[cfg(target_os = "windows")] {
        match command.spawn().and_then(|mut child| child.wait()) {
            Ok(status) => process::exit(status.code().unwrap_or(1)),
            Err(error) => die!("fatal: {}", error),
        };
    }

    #[cfg(not(target_os = "windows"))]{
        let error = command.exec();
        die!("fatal: {}", error);
    }
}