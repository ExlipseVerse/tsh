mod builtin;
mod shellhelper;
mod parser;
mod redirection;
mod shell;

#[allow(unused_imports)]

use shell::{Shell, Job};

use std::sync::{Arc,Mutex};
use std::collections::HashMap;

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::Config;
use rustyline::config::Configurer;
use rustyline::CompletionType;

use builtin::{BuiltIn, Flow};
use shellhelper::ShellHelper;

use parser::parse_input;
use redirection::{Redirection, extract_redirection};


fn run_pipeline(input: &str, shell: &mut Shell) -> bool {
    if !input.contains('|') {
        return false;
    }

    let parts: Vec<&str> = input.split("|").collect();

    if parts.len() != 2 {
        eprintln!("error");
        return true;
    }

    let cmd1_args = parse_input(parts[0].trim());
    let cmd2_args = parse_input(parts[1].trim());

    if cmd1_args.is_empty() || cmd2_args.is_empty() {
        eprintln!("error: Invalid command structure around");
        return true;
    }

    let mut cap_output: Vec<u8> = Vec::new();

    let cmd1_name = &cmd1_args[0];
    let (args1, _) = extract_redirection(&cmd1_args[1..]);

    if let Some(b) = BuiltIn::from_str(cmd1_name.trim().to_lowercase().as_str()) {
        let _flow = b.run_builtin(&args1, &mut cap_output, shell);
    } else if let Ok(p1) = which::which(cmd1_name) {
        let mut proc1 = Command::new(p1);
        proc1.arg0(cmd1_name);
        proc1.args(&args1);
        proc1.stdout(Stdio::piped());

        match proc1.spawn() {
            Ok(mut child1) => {
                if let Some(mut stdout1) = child1.stdout.take() {
                    let _ = io::copy(&mut stdout1, &mut cap_output);
                }
                let _ = child1.wait();
            }
            Err(e) => {
                eprintln!("Failed to execute first command: {}", e);
                return true;
            }
        }
    } else {
        eprintln!("{}: command not found", cmd1_name);
        return true;
    }


    let cmd2_name = &cmd2_args[0];
    let (args2, redirect_o2) = extract_redirection(&cmd2_args[1..]);

    if let Some(b2) = BuiltIn::from_str(cmd2_name.trim().to_lowercase().as_str()) {
        match redirect_o2 {
            Some(Redirection::Stdout(mut file)) => {
                let _flow = b2.run_builtin(&args2, &mut file, shell);
            }
            _ => {
                let _flow = b2.run_builtin(&args2, &mut io::stdout(), shell);
            }
        }
    } else if let Ok(path2) = which::which(cmd2_name) {
        let mut proc2 = Command::new(path2);
        proc2.arg0(cmd2_name);
        proc2.args(&args2);
        proc2.stdin(Stdio::piped());

        match redirect_o2 {
            Some(Redirection::Stdout(file)) => { proc2.stdout(file); }
            Some(Redirection::Stderr(file)) => { proc2.stderr(file); }
            None => {}
        }

        match proc2.spawn() {
            Ok(mut child2) => {
                if let Some(mut stdin2) = child2.stdin.take() {
                    use io::Write;
                    let _ = stdin2.write_all(&cap_output);
                }
                let _ = child2.wait();
            }
            Err(e) => {
                eprintln!("Failed to execute second command: {}", e);
                return true;
            }
        }
    } else {
        eprintln!("{}: command not found", cmd2_name);
    }

    true
}

fn main() {
    let completions = Arc::new(Mutex::new(HashMap::<String,String>::new()));
    let helper = ShellHelper::new(Arc::clone(&completions));
    let mut shell = Shell::new(completions);

    let config = Config::builder().build();
    let mut rl = Editor::<ShellHelper, _>::with_config(config).expect("Failed to initialize line reader"); //creating the reader editor
    rl.set_completion_type(CompletionType::List);
    rl.set_helper(Some(helper));

    loop {

        // Background job reap
        let mut finished = Vec::new();
        let len = shell.jobs.len();
        for (i, job) in shell.jobs.iter_mut().enumerate() {
            if let Ok(Some(_)) = job.child.try_wait() {
                let marker = if i + 1 == len { "+" } else if i + 2 == len { "-" } else { " " };
                println!("[{}]{}  Done\t\t{}", job.id, marker, job.cmd_string);
                finished.push(i);
            }
        }
        for index in finished.into_iter().rev() {
            shell.jobs.remove(index);
        }

        // NEW READER

        let input = match rl.readline("$ ") {
            Ok(line) => {
                if line.trim().is_empty() {continue;}
                let _ = rl.add_history_entry(line.as_str());
                line
            }
            Err(ReadlineError::Interrupted) => { println!("^C"); continue; }
            Err(ReadlineError::Eof)         => { println!("exit"); break; }
            Err(err) => { println!("Error reading input: {:?}", err); break; }
        }; 

        //OLD READER

        // print!("$ ");

        // let mut input = String::new();
        // io::stdout().flush().unwrap();
        // io::stdin().read_line(&mut input).unwrap();
        if run_pipeline(&input, &mut shell) {
            continue;
        }

        let mut command: Vec<String> = parse_input(&input);

        let Some(cmd_name) = command.first().cloned() else {continue}; 

        let run_in_bg = command.last().map(|a| a.as_str()) == Some("&");
        if run_in_bg {
            command.pop();
        }

        let cmd_args = command.get(1..).unwrap_or(&[]);
        let (args, redirect_o) = extract_redirection(cmd_args);

        if let Some(b) = BuiltIn::from_str(cmd_name.trim().to_lowercase().as_str()) {

            let flow = match redirect_o {
                Some(Redirection::Stdout(mut file)) => b.run_builtin(&args, &mut file, &mut shell),
                _ => b.run_builtin(&args, &mut io::stdout(), &mut shell),
            };
            if let Flow::Exit = flow { break; }

        } else if let Ok(path) = which::which(&cmd_name) {

            let mut proc = Command::new(path);
            proc.arg0(&cmd_name);
            proc.args(&args);

            match redirect_o {
                Some(Redirection::Stdout(file)) => { proc.stdout(file); }
                Some(Redirection::Stderr(file)) => { proc.stderr(file); }
                None => {}
            }

            match proc.spawn() {
                Ok(mut child) => {
                    if run_in_bg {
                        let pid = child.id();
                        
                        let mut job_id = 1;
                        while shell.jobs.iter().any(|j| j.id == job_id) {
                            job_id += 1;
                        }

                        println!("[{}] {}", job_id, pid);

                        let cmd_string = input.trim().strip_suffix('&').unwrap_or(&input).trim().to_string();
                        shell.jobs.push(Job {
                            id: job_id,
                            pid: pid,
                            cmd_string: cmd_string,
                            child
                        });

                        job_id += 1;

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


//         match BuiltIn::from_str(cmd_name.trim().to_lowercase().as_str()) {
    //             Some(BuiltIn::Echo) => {
                    
    //                 let output = args.join(" ");

    //                 match redirect_o {
    //                     Some(Redirection::Stdout(mut file)) => {
    //                         if let Err(e) = writeln!(file, "{}", output) {
    //                             println!("shell: write error: {}", e);
    //                         }
    //                     }

    //                     Some(Redirection::Stderr(_)) => {
    //                         println!("{}", output);
    //                     }

    //                     None => {
    //                         println!("{}", output);
    //                     }
    //                 }
    //             }
    //             Some(BuiltIn::Type) => {
    //                 if let Some(sec_cmd) = command.get(1) {
    //                     if BuiltIn::from_str(sec_cmd).is_some() {
    //                         println!("{} is a shell builtin", sec_cmd);
    //                     } else {
    //                         if let Ok(path) = which::which(sec_cmd) {
    //                             println!("{} is {}", sec_cmd, path.display());
    //                         } else {
    //                             println!("{} not found", sec_cmd);
    //                         }
                           
    //                     }
    //                 } else {
    //                     println!("missing argument");
    //                 }
    //             }

    //             Some(BuiltIn::Pwd) => {
    //                 let (_, redirect_o) = extract_redirection(cmd_args);
    //                 if let Ok(curr_dir) = env::current_dir() {
    //                     match redirect_o {
    //                         Some(Redirection::Stdout(mut file)) => {
    //                             if let Err(e) = writeln!(file, "{}", curr_dir.display()) {
    //                                 println!("shell: write error: {}", e); 
    //                             }
    //                         }

    //                         Some(Redirection::Stderr(_)) => {
    //                             println!("{}", curr_dir.display());
    //                         }
    //                         None => {
    //                             println!("{}", curr_dir.display());
    //                         }
    //                     }
    //                 } else {
    //                     println!("Failed to get current directory");
    //                 }
    //             }

    //             Some(BuiltIn::Cd) => {
    //                 if let Some(dir) = command.get(1) {
    //                     match dir.as_str() {
    //                         "~" => {
    //                             if let Some(home_dir) = env::home_dir() {
    //                                 if let Err(e) = env::set_current_dir(&home_dir) {
    //                                     if e.kind() == ErrorKind::NotFound {
    //                                         println!("cd: {}: No such file or directory", dir);
    //                                     } else {
    //                                         println!("cd: {}: {}", dir, e);
    //                                     }
    //                                 }
    //                             } else {
    //                                 println!("cd: Home directory not found");
    //                             }
    //                         }

    //                         _ => {
    //                             if let Err(e) = env::set_current_dir(dir) {
    //                                 if e.kind() == ErrorKind::NotFound {
    //                                     println!("cd: {}: No such file or directory", dir);
    //                                 } else {
    //                                     println!("cd: {}: {}", dir, e);
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 } else {
    //                     println!("missing argument");
    //                 }
    //             }

    //             Some(BuiltIn::Complete) => {
    //                 if let Some(option) = command.get(1) {
    //                     match option.to_lowercase().as_str() {
    //                         "-p" => {
    //                             if let Some(cmd) = command.get(2) {
    //                                 let reg = completion_reg.lock().unwrap();
    //                                 if let Some(spec) = reg.get(cmd) {
    //                                     println!("complete {} {}", spec, cmd);
    //                                 } else {
    //                                     println!("complete: {}: no completion specification", cmd);
    //                                 }
                                    
    //                             }
    //                         }

    //                         "-c" => { 
    //                             if let (Some(path), Some(cmd)) = (command.get(2), command.get(3)) {
    //                                 let mut registry = completion_reg.lock().unwrap();
    //                                 registry.insert(cmd.to_string(), format!("-C '{}'", path));
    //                             }
    //                         }

    //                         "-r" => {
    //                             if let Some(cmd) = command.get(2) {
    //                                 let mut reg = completion_reg.lock().unwrap();
    //                                 reg.remove(cmd);
    //                             }
    //                         }

    //                         _=> {}
    //                     }
    //                 }
    //             }

    //             Some(BuiltIn::Jobs) => {
    //                 let len = job_list.len();
    //                 let mut to_rm = Vec::new();

    //                 for (index, job) in job_list.iter_mut().enumerate() {
    //                     let symbol = if index == len - 1 {
    //                         "+"
    //                     } else if index == len - 2 {
    //                         "-"
    //                     } else {
    //                         " "
    //                     };

    //                     match job.child.try_wait() {
    //                         Ok(Some(_status)) => {
    //                             println!("[{}]{} Done\t\t{}", job.id, symbol, job.cmd_string);
    //                             to_rm.push(index);
    //                         }

    //                         Ok(None) => {
    //                             println!("[{}]{}  Running\t\t{} &", job.id, symbol, job.cmd_string);
    //                         }

    //                         Err(_) => {
    //                             to_rm.remove(index);
    //                         }
    //                     };
    //                 }

    //                 for index in to_rm.into_iter().rev() {
    //                     job_list.remove(index);
    //                 }

    //             }

    //             Some(BuiltIn::Exit) => {
    //                 break;
    //             }

    //             None => {
    //                 if let Ok(path) = which::which(&cmd_name) { // reads the path 
    //                     // .args(&command[1..])
    //                     //     .spawn();



                        
    //             }
    //         }
    //     } else {

    //         //do nothing 
    //         // println!("{}: command not found", input.trim());
    //     }



    //     // if command.first().is_none() {
    //     //     continue;
    //     // } else if command[0] == "echo" {
    //     //     println!("{}", command[1..].join(" "));
    //     // } else if command[0] == "type" {

    //     // } else {
    //     //     println!("{}: command not found", input.trim());
    //     // }
        
    // }