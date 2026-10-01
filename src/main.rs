mod builtin;
mod shellhelper;
mod parser;
mod redirection;

#[allow(unused_imports)]
use std::sync::{Arc,Mutex};
use std::collections::HashMap;

use std::io::{self, Write, Read, ErrorKind};
use std::fs::{read_dir, write, metadata, File, OpenOptions};
use std::os::unix::process::CommandExt;
use std::env;
use std::path::{PathBuf, Path};
use std::process::{Command, Child};

use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::Config;
use rustyline::config::Configurer;
use rustyline::CompletionType;

use builtin::BuiltIn;
use shellhelper::ShellHelper;

use parser::parse_input;
use redirection::{Redirection, extract_redirection};

struct Job {
    id: usize,
    pid: u32,
    cmd_string: String,
    child: Child
}

fn main() {
    let mut job_list: Vec<Job> = Vec::new();
    let mut next_job_id = 1;

    let completion_reg = Arc::new(Mutex::new(HashMap::<String,String>::new()));
    let config = Config::builder().build();
    let mut rl = Editor::<ShellHelper, _>::with_config(config).expect("Failed to initialize line reader"); //creating the reader editor
    rl.set_completion_type(CompletionType::List);

    let helper = ShellHelper::new(Arc::clone(&completion_reg));
    rl.set_helper(Some(helper));
    loop {

        let mut indices_to_rm = Vec::new();
        let current_len = job_list.len();

        for (index, job) in job_list.iter_mut().enumerate() {
            match job.child.try_wait() {
                Ok(Some(_status)) => {
                    let symbol = if index == current_len - 1 {
                        "+"
                    } else if index == current_len - 2 {
                        "-"
                    } else {
                        " "
                    };

                    println!("[{}]{}  Done\t\t{}", job.id, symbol, job.cmd_string);
                    indices_to_rm.push(index);
                }

                _=>{}
            }
        }

        for index in indices_to_rm.into_iter().rev() {
            job_list.remove(index);
        }

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

        let mut command: Vec<String> = parse_input(&input);

        if let Some(cmd_name) = command.first().cloned() {

            let run_in_bg = command.last().map(|a| a.as_str()) == Some("&");
            if run_in_bg {
                command.pop();
            }

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

                Some(BuiltIn::Complete) => {
                    if let Some(option) = command.get(1) {
                        match option.to_lowercase().as_str() {
                            "-p" => {
                                if let Some(cmd) = command.get(2) {
                                    let reg = completion_reg.lock().unwrap();
                                    if let Some(spec) = reg.get(cmd) {
                                        println!("complete {} {}", spec, cmd);
                                    } else {
                                        println!("complete: {}: no completion specification", cmd);
                                    }
                                    
                                }
                            }

                            "-c" => { 
                                if let (Some(path), Some(cmd)) = (command.get(2), command.get(3)) {
                                    let mut registry = completion_reg.lock().unwrap();
                                    registry.insert(cmd.to_string(), format!("-C '{}'", path));
                                }
                            }

                            "-r" => {
                                if let Some(cmd) = command.get(2) {
                                    let mut reg = completion_reg.lock().unwrap();
                                    reg.remove(cmd);
                                }
                            }

                            _=> {}
                        }
                    }
                }

                Some(BuiltIn::Jobs) => {
                    let len = job_list.len();
                    let mut to_rm = Vec::new();

                    for (index, job) in job_list.iter_mut().enumerate() {
                        let symbol = if index == len - 1 {
                            "+"
                        } else if index == len - 2 {
                            "-"
                        } else {
                            " "
                        };

                        match job.child.try_wait() {
                            Ok(Some(_status)) => {
                                println!("[{}]{} Done\t\t{}", job.id, symbol, job.cmd_string);
                                to_rm.push(index);
                            }

                            Ok(None) => {
                                println!("[{}]{}  Running\t\t{} &", job.id, symbol, job.cmd_string);
                            }

                            Err(_) => {
                                to_rm.remove(index);
                            }
                        };
                    }

                    for index in to_rm.into_iter().rev() {
                        job_list.remove(index);
                    }

                }

                Some(BuiltIn::Exit) => {
                    break;
                }

                None => {
                    if let Ok(path) = which::which(&cmd_name) { // reads the path 
                        // .args(&command[1..])
                        //     .spawn();



                        let mut proc = Command::new(path);
                        proc.arg0(&cmd_name);
                           
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

                        match proc.spawn() {
                            Ok(mut child) => {
                                if run_in_bg {
                                    let pid = child.id();
                                    println!("[{}] {}", next_job_id, pid);

                                    let cleaned_cmd = input.trim().strip_suffix('&').unwrap_or(&input).trim().to_string();

                                    if job_list.is_empty() {
                                        next_job_id = 1;
                                    }
                                    
                                    job_list.push(Job {
                                        id: next_job_id,
                                        pid: pid,
                                        cmd_string: cleaned_cmd,
                                        child
                                    });

                                    next_job_id += 1;

                                } else {
                                    let _ = child.wait();
                                }
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
