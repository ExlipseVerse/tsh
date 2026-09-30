mod builtin;
mod shellhelper;

#[allow(unused_imports)]
use std::io::{self, Write, Read, ErrorKind};
use std::fs::{read_dir, write, metadata, File, OpenOptions};
use std::os::unix::process::CommandExt;
use std::env;
use std::path::{PathBuf, Path};
use std::process::Command;

use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::Config;
use rustyline::config::Configurer;
use rustyline::CompletionType;

use builtin::BuiltIn;
use shellhelper::ShellHelper;

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
    let mut is_escaped = false;


    for c in input.chars() { // by for example a, b, c

        if is_escaped {
            if in_double_quotes {
                match c {
                    '"' | '\\' => {    
                        current_arg.push(c);
                    }

                    _ => {
                        current_arg.push('\\');
                        current_arg.push(c);
                    }
                }
            } else {
                current_arg.push(c);
            }
            
            has_content = true;
            is_escaped = false;
            continue
        }
        
        match c {
            '\\' => {
                if in_single_quotes {
                    current_arg.push(c);
                } else {
                    is_escaped = true;
                }
            }

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

            ' ' | '\t' | '\n' | '\r' => {
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

    if is_escaped {
        current_arg.push('\\');
    }
    if has_content || !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

enum Redirection {
    Stdout(File),
    Stderr(File),
}

fn extract_redirection(args: &[String]) -> (Vec<&str>, Option<Redirection>) {
    if let Some(pos) = args.iter().position(|arg| arg == ">" || arg == "1>" || arg == "2>" || arg == ">>" || arg == "1>>" || arg == "2>>") {
        let redirect_type = &args[pos];
        let (clean_slice, redirect_path) = args.split_at(pos);
        let clean_args: Vec<&str> = clean_slice.iter().map(|s| s.as_str()).collect();


        if let Some(file_name) = redirect_path.get(1) {

            let is_append = redirect_type == ">>" || redirect_type == "1>>" || redirect_type == "2>>";
            let file_result = OpenOptions::new()
                .write(true)
                .create(true)
                .append(is_append)
                .open(file_name);

            match file_result {
                Ok(file) => {
                    if redirect_type == "2>" || redirect_type == "2>>" {
                        return (clean_args, Some(Redirection::Stderr(file)));
                    } else {
                        return (clean_args, Some(Redirection::Stdout(file)));
                    }
                }
                Err(e) => {
                    println!("shell: failed to open redirect file: {}", e);
                    return (clean_args, None);
                }
            }
        } 

        (clean_args, None)
    } else {
        (args.iter().map(|s| s.as_str()).collect(), None)
    }
}

fn main() {
    let config = Config::builder().build();
    let mut rl = Editor::<ShellHelper, _>::with_config(config).expect("Failed to initialize line reader"); //creating the reader editor
    rl.set_completion_type(CompletionType::List);
    rl.set_helper(Some(ShellHelper::new()));
    loop {

        // NEW READER

        let input = match rl.readline("$ ") {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line.as_str());
                line
            }

            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }

            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            }
            Err(err) => {
                println!("Error reading input: {:?}", err);
                break;
            }
        }; 

        //OLD READER

        // print!("$ ");

        // let mut input = String::new();
        // io::stdout().flush().unwrap();
        // io::stdin().read_line(&mut input).unwrap();
        
        if input.trim() == "exit" {
            break;
        }

        let command: Vec<String> = parse_input(&input);

        if let Some(cmd_name) = command.first() {

            let cmd_args = command.get(1..).unwrap_or(&[]);

            match BuiltIn::from_str(cmd_name.trim().to_lowercase().as_str()) {
                Some(BuiltIn::Echo) => {
                    let (args, redirect_o) = extract_redirection(cmd_args);
                    let output = args.join(" ");

                    match redirect_o {
                        Some(Redirection::Stdout(mut file)) => {
                            if let Err(e) = writeln!(file, "{}", output) {
                                println!("shell: write error: {}", e);
                            }
                        }

                        Some(Redirection::Stderr(_)) => {
                            println!("{}", output);
                        }

                        None => {
                            println!("{}", output);
                        }
                    }
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
                    let (_, redirect_o) = extract_redirection(cmd_args);
                    if let Ok(curr_dir) = env::current_dir() {
                        match redirect_o {
                            Some(Redirection::Stdout(mut file)) => {
                                if let Err(e) = writeln!(file, "{}", curr_dir.display()) {
                                    println!("shell: write error: {}", e); 
                                }
                            }

                            Some(Redirection::Stderr(_)) => {
                                println!("{}", curr_dir.display());
                            }
                            None => {
                                println!("{}", curr_dir.display());
                            }
                        }
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
                    if let Ok(path) = which::which(cmd_name) { // reads the path 
                        // .args(&command[1..])
                        //     .spawn();

                        let mut proc = Command::new(path);
                        proc.arg0(cmd_name);
                           
                        let (args, redirect_o) = extract_redirection(cmd_args);
                        proc.args(&args);

                        match redirect_o {
                            Some(Redirection::Stdout(file)) => {
                                proc.stdout(file);
                            }

                            Some(Redirection::Stderr(file)) => {
                                proc.stderr(file);
                            }

                            None => {}
                        }

                        // if let Some(file) = redirect_o {
                        //     proc.stdout(file);
                        // }
                        
                        match proc.spawn() {
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
