use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use crate::shell::Shell;

pub enum Flow {
    Continue,
    Exit
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn {
    Echo,
    Type,
    Exit,
    Pwd,
    Cd,
    Complete,
    Jobs,
    History
}

impl BuiltIn {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "echo" => Some(BuiltIn::Echo),
            "type" => Some(BuiltIn::Type),
            "exit" => Some(BuiltIn::Exit),
            "pwd" => Some(BuiltIn::Pwd),
            "cd" => Some(BuiltIn::Cd),
            "complete" => Some(BuiltIn::Complete),
            "jobs" => Some(BuiltIn::Jobs),
            "history" => Some(BuiltIn::History),
            _ => None,
        }
    }

    pub fn run_builtin(self, args: &[&str], out: &mut dyn Write, shell: &mut Shell) -> Flow {
        match self {
            BuiltIn::Echo => {let _ = writeln!(out, "{}", args.join(" "));}

            BuiltIn::Pwd => match env::current_dir() {
                Ok(d) => { let _= writeln!(out, "{}", d.display());}
                Err(e) => eprintln!("pwd: {}", e),
            }

            BuiltIn::Type => match args.first() {
                Some(name) if BuiltIn::from_str(name).is_some() => {
                    let _ = writeln!(out, "{} is a shell builtin", name);
                }
                Some(name) => match which::which(name) {
                    Ok(p)  => { let _ = writeln!(out, "{} is {}", name, p.display()); }
                    Err(_) => { let _ = writeln!(out, "{} not found", name); }
                }

                None => eprintln!("type: missing argument"),
            }

            BuiltIn::Cd => {
                let Some(dir) = args.first() else {
                    eprintln!("cd: missing argument");
                    return Flow::Continue;
                };

                let target = match *dir {
                    "~" => match env::home_dir() {
                        Some(h) => h,
                        None => { eprintln!("cd: Home directory not found"); return Flow::Continue;}
                    }
                    other => PathBuf::from(other),
                };

                if let Err(e) = env::set_current_dir(&target) {
                    if e.kind() == ErrorKind::NotFound {
                        eprintln!("cd: {}: No such file or directory", dir);
                    } else {
                        eprintln!("cd: {}: {}", dir, e);
                    }
                }
            }

            BuiltIn::Complete => {
                let mut reg = shell.completions.lock().unwrap();
                if let Some(flag) = args.first() {
                    match flag.to_lowercase().as_str() {
                        "-p" => {
                            if let Some(cmd) = args.get(1) {
                                match reg.get(*cmd) {
                                    Some(spec) => { let _ = writeln!(out, "complete {} {}", spec, cmd); }
                                    None => { let _ = writeln!(out, "complete: {}: no completion specification", cmd); }
                                }
                            }
                        }
                        "-c" => {
                            if let (Some(path), Some(cmd)) = (args.get(1), args.get(2)) {
                                reg.insert(cmd.to_string(), format!("-C '{}'", path));
                            }
                        }
                        "-r" => {
                            if let Some(cmd) = args.get(1) {
                                reg.remove(*cmd);
                            }
                        }
                        _ => {}
                    }
                }
            }

            BuiltIn::Jobs => {
                let len = shell.jobs.len();
                let mut done = Vec::new();
                for (i, job) in shell.jobs.iter_mut().enumerate() {
                    let marker = if i + 1 == len { "+" } else if i + 2 == len { "-" } else { " " };
                    match job.child.try_wait() {
                        Ok(Some(_status)) => {
                            let _ = writeln!(out, "[{}]{}  Done\t\t{}", job.id, marker, job.cmd_string);
                            done.push(i);
                        }
                        _ => {
                            let _ = writeln!(out, "[{}]{}  Running\t\t{} &", job.id, marker, job.cmd_string);
                        }
                        
                    }
                }

                for i in done.into_iter().rev() { shell.jobs.remove(i); }
            }

            BuiltIn::History => {
                if let Some(arg) = args.get(0) {
                    match *arg {
                        "-r" => {
                            if let Some(path) = args.get(1) {
                                if let Ok(content) = fs::read_to_string(path) {
                                    for line in content.lines() {
                                        let trimmed = line.trim();
                                        if !trimmed.is_empty() {
                                            shell.history.push(line.to_string());
                                        };
                                    }
                                } else {
                                    let _ = writeln!(out, "Error: Could not read history file '{}'", path);
                                }
                            } else {
                                let _ = writeln!(out, "Error: Missing file path for history -r");
                            }
                        }

                        "-w" => {
                            if let Some(path) = args.get(1) {
                                let history_content = shell.history.join("\n");
                                if let Ok(_) = fs::write(path, format!("{}\n", history_content)) {

                                } else {
                                    let _ = writeln!(out, "Error: Could not write history to file '{}'", path);
                                }
                            } else {
                                let _ = writeln!(out, "Error: Missing file path for history -w");
                            }
                        }

                        "-a" => {
                            if let Some(path) = args.get(1) {
                                
                                let skip = shell.history_index;

                                if let Ok(mut file) = OpenOptions::new().append(true).create(true).open(path) {
                                    for cmd in &shell.history.iter().skip(skip) {
                                        if let Err(_) = writeln!(file, "{}", cmd) {
                                            let _= writeln!(out, "Error: Failed to write data to history file '{}'", path);
                                            break;
                                        }
                                    }

                                    shell.history_index = shell.history.len();
                                } else {
                                    let _ = writeln!(out, "Error: Could not open history file '{}'", path);
                                }
                            } else {
                                let _ = writeln!(out, "Error: Missing file path for history -w");
                            }
                        }

                        _=> {
                            if let Ok(limit) = arg.parse::<usize>() {
                                let mut hist_l: Vec<_> = shell.history.iter().enumerate().rev().take(limit).collect();
                                hist_l.reverse();
                                for (i, cmd_line) in hist_l {
                                    let _= writeln!(out, "{:>5}  {}", i+1, cmd_line);
                                }
                            } else {
                                let _ = writeln!(out, "Error: Invalid history limit '{}'", arg);
                            }
                        }
                    }
                } else {
                    for (i, cmd_line) in shell.history.iter().enumerate() {
                        let _= writeln!(out, "{:>5}  {}", i+1, cmd_line);
                    }
                }
                
            }

            BuiltIn::Exit => return Flow::Exit,
        }

        Flow::Continue
    }
}