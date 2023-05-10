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

fn load_mode_env_name() -> String {
    
    if let Ok(vars) = dotenvy::from_filename_iter(".env.local") {
        for item in vars {
            if let Ok(item) = item {
                let (key, val) = item;
                if key.eq("MODE_ENV") {
                    return val;
                }
            }
        }
    }

    if let Ok(vars) = dotenvy::from_filename_iter(".env") {
        for item in vars {
            if let Ok(item) = item {
                let (key, val) = item;
                if key.eq("MODE_ENV") {
                    return val;
                }
            }
        }
    }

    return env::var("MODE_ENV").unwrap_or("MODE".to_string())
}


fn load_env(mode_env_name: &str, mode_from_cmd: &Option<&str>, specified_mode: &Option<String>) {
    let mode = match specified_mode.to_owned() {
        Some(m) => m,
        None => match mode_from_cmd {
            // we do not read MODE from env file as this will be used to get which file to load
            None => match env::var(mode_env_name) {
                Ok(str) => str,
                Err(_) => "local".to_string(),
            },
            Some(mode) => mode.to_string(),
        }
    };

    // we also set the mode as env variable
    env::set_var(&mode_env_name, &mode);

    
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

}


fn main() {

    let matches = clap::Command::new("ldenv")
        .about("\
        Run a command using the environment from .env, .env.local (, env.<mode> and env.<mode>.local) files\n\
        It also parse the command to replace `@@<name>[@:<default>]@:` with their env value. Fails if not present and no default provided.\n\
        Note that you can also provide prefix and suffix: `[prefix]@@<name>[@:<default>@:][suffix]`\
        ")
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
            Arg::new("MODE_ENV_NAME")
                .short('n')
                .long("env-mode-name")
                .takes_value(true)
                .help("env-mode-name: environment variable used as mode if none is provided on the command line, default to MODE or the value of env MODE_ENV"),
        )
        .arg(
            Arg::new("NO_PARSE")
                .short('P')
                .long("no-parse")
                .help("no-parse: if set do not parse the args passed in. ").action(clap::ArgAction::SetTrue),
        )
        .get_matches();

        
    let parse = !matches.get_flag("NO_PARSE");


    let mode_env_name = match matches.value_of("MODE_ENV_NAME") {
        None => load_mode_env_name(),
        Some(mode_env_name) => mode_env_name.to_string(),
    };

    let mode_from_cmd = matches.value_of("MODE");


    let mut command = match matches.subcommand() {
        Some((name, matches)) => {
            let mut remove_next = false;
            let mut mode: Option<String> = None;
            let symbol = "@@";
            let mut args = matches
                .values_of("")
                .map(|v| v.collect())
                .unwrap_or(Vec::new());
            
            args.retain(|arg| {
                if remove_next  {
                    mode =Some(arg.to_string());
                    remove_next = false;
                    return false;
                };
                if arg.eq(&symbol) {
                    remove_next = true;
                    false
                } else {
                    true
                }
            });

            if remove_next {
                if mode_from_cmd.is_none() {
                    die!("error: expect to be provided a mode (which set the {} env variable) as last argument", mode_env_name);
                }
            }

            // TODO remove the clone here
            if let Some(mode) = mode.clone() {
                if mode.len() == 0 {
                    die!("error: mode has been specified as argument, but it is empty");
                }
            }

            load_env(&mode_env_name, &mode_from_cmd, &mode);
            

            let args: Vec<String> = args.iter()
            .map(|&s| 
                if parse {
                    let arg_splitted: Vec<&str> = s.split("@@").collect();
                    let mut i = 0;
                    let result: Vec<String> = arg_splitted.iter().map(|s| {
                        i = i +1;
                        if i == 1 {
                            s.to_string()
                        } else {
                            let splitted: Vec<&str> = s.split("@:").collect();
                            let var_name = splitted[0];
                            if var_name.len() == 0 {
                                die!("error: this is not valid : '@@{}' please specify an ENV var name after '@@'", s);
                            }
                            let default_value = if splitted.len() > 2 {
                                Some(splitted[1])
                            } else {
                                None
                            };
                            let suffix = if splitted.len() > 2 {
                                splitted[2]
                            } else if splitted.len() > 1 {
                                splitted[1]
                                
                            } else {
                                ""
                            };
                            let var_names: Vec<&str> = var_name.split(",").collect();
                            let mut potential_value: Option<String> = None;
                            for name in var_names {
                                let splitted_by_colon = name.split(":");
                                let mut j = 0;
                                let actual_name_as_vec: Vec<String> = splitted_by_colon.map(|s| {
                                    j = j +1;
                                    if j % 2 == 1 {
                                        s.to_string()
                                    } else {
                                        if let Ok(name_found) = env::var(s) {
                                            name_found
                                        } else {
                                            "".to_string()
                                        }
                                    }
                                }).collect();
                                // println!("{:?}", actual_name_as_vec);
                                let actual_name = actual_name_as_vec.join("");
                                // println!("{}", actual_name);
                                if let Ok(name_found) = env::var(actual_name) {
                                    potential_value = Some(name_found);
                                    break;
                                }
                            }
                            let arg = if let Some(v) = potential_value {
                                v
                            } else {
                                match default_value {
                                    Some(v) => v.to_string(),
                                    None => die!("\
                                    error: @@{} was specified in the command but there is no env variable named {}.\n\
                                    To prevent this error you can provide a default value with '@@{}@:<default value>@:'\n\
                                    An empty default can be specified with '@@{}@:@:'",
                                    s, 
                                    var_name, var_name, var_name)
                                }
                            };
                            format!("{}{}", arg, suffix)
                        }
                    }).collect();
                    result.join("")
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