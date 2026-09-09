mod builtin;

#[allow(unused_imports)]
use std::io::{self, Write, Read, ErrorKind};
use std::os::unix::process::CommandExt;
use std::env;
use std::path::{PathBuf, Path};
use std::process::Command;

use builtin::BuiltIn;

// fn fetchPath() -> Option<Vec<PathBuf>> {
//     if let Some(path_var) = env::var_os("PATH") {
//         Some(env::split_paths(&path_var).collect())
//     } else {
//         None
//     }
// }

// fn is_executable(path: &Path) -> bool {
//     path.is_file() && path.metadata().map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false)
// }

fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new(); // we create an array []
    let mut current_arg = String::new(); // we create a string to store the current argument
    let mut in_single_quotes = false; // we create a boolean to check if we are in single quotes
    let mut in_double_quotes = false;
    let mut has_content = false; // we create a boolean to check if we have content in the current argument

    for c in input.chars() { // by for example
        match c {
            '\'' => {
                if in_double_quotes {
                    current_arg.push(c);
                } else {
                    in_single_quotes = !in_single_quotes;
                    has_content = true;
                }
            }

            '"' => {
                if in_single_quotes {
                    current_arg.push(c);
                } else {
                    in_double_quotes = !in_double_quotes;
                    has_content = true;
                }
            }

            ' ' | '\t' | '\n' | '\r' if !in_single_quotes => {
                if in_single_quotes || in_double_quotes {
                    current_arg.push(c);
                } else {
                    if has_content || !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                        has_content = false;
                    }
                }
            }

            _ => {
                current_arg.push(c);
                has_content = true;
            }
        }

    }

    if has_content || !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    loop {
        print!("$ ");

        let mut input = String::new();
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim() == "exit" {
            break;
        }

        let command: Vec<String> = parse_input(&input);

        if let Some(cmd_name) = command.first() {
            match BuiltIn::from_str(cmd_name.trim().to_lowercase().as_str()) {
                Some(BuiltIn::Echo) => {
                    println!("{}", command[1..].join(" "));
                }
                Some(BuiltIn::Type) => {
                    if let Some(sec_cmd) = command.get(1) {
                        if BuiltIn::from_str(sec_cmd).is_some() {
                            println!("{} is a shell builtin", sec_cmd);
                        } else {
                            if let Ok(path) = which::which(sec_cmd) {
                                println!("{} is {}", sec_cmd, path.display());
                            } else {
                                println!("{} not found", sec_cmd);
                            }
                           
                        }
                    } else {
                        println!("missing argument");
                    }
                }

                Some(BuiltIn::Pwd) => {
                    if let Ok(curr_dir) = env::current_dir() {
                        println!("{}", curr_dir.display());
                    } else {
                        println!("Failed to get current directory");
                    }
                }

                Some(BuiltIn::Cd) => {
                    // if command.len() < 1 {
                        
                    // }

                    if let Some(dir) = command.get(1) {
                        match dir.as_str() {
                            "~" => {
                                if let Some(home_dir) = env::home_dir() {
                                    if let Err(e) = env::set_current_dir(&home_dir) {
                                        if e.kind() == ErrorKind::NotFound {
                                            println!("cd: {}: No such file or directory", dir);
                                        } else {
                                            println!("cd: {}: {}", dir, e);
                                        }
                                    }
                                } else {
                                    println!("cd: Home directory not found");
                                }
                            }

                            _ => {
                                if let Err(e) = env::set_current_dir(dir) {
                                    if e.kind() == ErrorKind::NotFound {
                                        println!("cd: {}: No such file or directory", dir);
                                    } else {
                                        println!("cd: {}: {}", dir, e);
                                    }
                                }
                            }
                        }
                    } else {
                        println!("missing argument");
                    }
                }

                Some(BuiltIn::Exit) => {
                    break;
                }

                None => {
                    if let Ok(path) = which::which(cmd_name) {
                        
                        let mut proc = Command::new(path)
                            .arg0(cmd_name)
                            .args(&command[1..])
                            .spawn();
                        match proc {
                            Ok(mut child) => {
                                let _ = child.wait();
                            }

                            Err(e) => {
                                println!("Failed to execute {}: {}", cmd_name, e);
                            }
                        }

                    } else {
                        println!("{}: command not found", cmd_name);
                    }
                }
            }
        } else {

            //do nothing 
            // println!("{}: command not found", input.trim());
        }



        // if command.first().is_none() {
        //     continue;
        // } else if command[0] == "echo" {
        //     println!("{}", command[1..].join(" "));
        // } else if command[0] == "type" {

        // } else {
        //     println!("{}: command not found", input.trim());
        // }
        
    }
}
