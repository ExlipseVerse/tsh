use crate::builtin::BuiltIn;
use std::env;
use std::process::Command;
use std::fs::{read_dir};
use std::io::{self, Write};
use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use rustyline::completion::{Completer, Pair, FilenameCompleter, longest_common_prefix};
use rustyline::Context;
use rustyline::Result;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::Helper;


pub struct ShellHelper {
	pub registered_completions: Arc<Mutex<HashMap<String, String>>>,
}


impl Helper for ShellHelper {}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}
impl Hinter for ShellHelper {
	type Hint = String;
}

impl ShellHelper {
	pub fn new(completions: Arc<Mutex<HashMap<String, String>>>) -> Self {
		Self {
			registered_completions: completions,
		}
	}
}

impl Completer for ShellHelper {
	type Candidate = Pair;

	fn complete (
		&self,
		line: &str,
		pos: usize,
		_ctx: &Context<'_>,
	) -> Result<(usize, Vec<Self::Candidate>)> {
		let mut candidates = Vec::new();
		
		if line[..pos].trim().is_empty() {
			return Ok((0, candidates));
		}

		let prefix = &line[..pos];

		if line[..pos].contains(' ') {
			let words: Vec<&str> = line[..pos].split_whitespace().collect();
			if let Some(&first_wrd) = words.first() {
				let reg = self.registered_completions.lock().unwrap();
				if let Some(opts) = reg.get(first_wrd) {
					let curr_arg = if line[..pos].ends_with(' ') { "" } else { words.last().copied().unwrap_or("") };
					let start = pos - curr_arg.len();

					if opts.starts_with("-C ") || opts.starts_with("-c ") {
						let raw_p = &opts[3..];
						let script_path = raw_p
							.trim_matches('\'')
							.to_string();

						let prev_word = if words.len() > 1 {
							words[words.len()-2]
						} else {
							first_wrd
						};

						if let Ok(output) = Command::new(&script_path)
							.arg(first_wrd)				
							.arg(curr_arg)
							.arg(prev_word)
							.env("COMP_LINE", line)
							.env("COMP_POINT", pos.to_string())
							.output()
						{
							if output.status.success() {
								let stdout_str = String::from_utf8_lossy(&output.stdout);
								let mut s_candidates = Vec::new();

								for li in stdout_str.lines() {
									let candidate = li.trim();
									if !candidate.is_empty() && candidate.starts_with(curr_arg) {
										s_candidates.push(Pair {
											display: candidate.to_string(),
											replacement: format!("{} ", candidate),
										})
									}
								}

								if !s_candidates.is_empty() {
									let paths: Vec<String> = s_candidates.iter().map(|c| c.display.clone()).collect();
									if let Some(prefix_str) = longest_common_prefix(&paths) {
										if prefix_str.len() > curr_arg.len() {
											let is_single = s_candidates.len() == 1;
											let replacement = if is_single {
												format!("{} ", prefix_str)
											} else {
												prefix_str.to_string()
											};
											return Ok((start, vec![Pair {
												display: prefix_str.to_string(),
												replacement,
											}]));
										}
									}
									return Ok((start, s_candidates));
								}
							}
						}
					}

					let mut custom_matches = Vec::new();
					for opt in opts.split_whitespace() {
						if opt.starts_with(curr_arg) {
							custom_matches.push(Pair {
								display: opt.to_string(),
								replacement: format!("{} ", opt),
							});
						}
					}

					if !custom_matches.is_empty() {
						return Ok((start, custom_matches));
					}
				}
			}

			let (start, n_matches) = FilenameCompleter::new().complete_path(line, pos)?;
			if n_matches.is_empty() {
				print!("\x07"); //bell
				let _ = io::stdout().flush();
				return Ok((0, Vec::new()));
			}

			let mut matches = Vec::new();
			for candidate in n_matches {
				let mut display = candidate.display.to_string();
				let mut replacement = candidate.replacement.to_string();

				let is_dir = display.ends_with(std::path::MAIN_SEPARATOR) || replacement.ends_with(std::path::MAIN_SEPARATOR);

				if is_dir {
					if !display.ends_with(std::path::MAIN_SEPARATOR) {
						display.push(std::path::MAIN_SEPARATOR);
					}
					if !replacement.ends_with(std::path::MAIN_SEPARATOR) {
						replacement.push(std::path::MAIN_SEPARATOR);
					}
				} else {
					if !replacement.ends_with(' ') {
						replacement.push(' ');
					}
				}

				matches.push(Pair {
					display,
					replacement
				});
			}

			if matches.len() == 1 {
				return Ok((start, matches));
			}

			if !matches.is_empty() {
				let paths: Vec<String> = matches.iter().map(|c| c.display.clone()).collect();
				let lcp = longest_common_prefix(&paths);
				if let Some(prefix_str) = lcp {
					let typed = &line[start..pos];
					if prefix_str.len() > typed.len() {
						let is_single = matches.len() == 1;
						let repl_str = if is_single {
							matches[0].replacement.clone()
						} else {
							prefix_str.to_string()
						};

						return Ok((start, vec![Pair {
							display: prefix_str.to_string(),
							replacement: repl_str,
						}]));
					}
				}
			}

            return Ok((start, matches));
		}

		

		let builtins = [
			BuiltIn::Cd,
			BuiltIn::Pwd,
			BuiltIn::Echo,
			BuiltIn::Exit,
			BuiltIn::Complete
		];

		for builtin in &builtins {
			let builtin_str = format!("{:?}", builtin).to_lowercase();
			if builtin_str.starts_with(prefix) {
				candidates.push(Pair {
					display: builtin_str.clone(),
					replacement: format!("{} ", builtin_str),
				});
			}
		}

		if let Some(path_var) = env::var_os("PATH") {
			for path in env::split_paths(&path_var) {
				if let Ok(entries) = read_dir(path) {
					for entry in entries.flatten() {
						if let Ok(file_name) = entry.file_name().into_string() {
							if file_name.starts_with(prefix) {
								if let Ok(metadata) = entry.metadata() {
									if metadata.is_file() {
										candidates.push(Pair {
											display: file_name.clone(),
											replacement: format!("{} ", file_name),
										});
									}
								}
							}
						}
					}
				}
			}
		}

		candidates.sort_by(|a, b| a.display.cmp(&b.display));
		candidates.dedup_by(|a, b| a.display == b.display);

		if candidates.is_empty() {
			print!("\x07"); // bell  character ~ bell code
			let _ = io::stdout().flush();
			candidates.push(Pair {
				display: prefix.to_string(),
				replacement: prefix.to_string(),
			});
			return Ok((0, candidates));
		}

		let display_names: Vec<String> = candidates.iter().map(|c| c.display.clone()).collect();
		let lcp = longest_common_prefix(&display_names);

		if let Some(prefix_str) = lcp {
			if prefix_str.len() > prefix.len() {
				let is_full_match = candidates.len() == 1 && prefix_str == candidates[0].display;

				let replacement_string = if is_full_match {
					format!("{} ", prefix_str)
				} else {
					prefix_str.to_string()
				};

				return Ok((0, vec![Pair { 
					display: replacement_string.clone(), 
					replacement: replacement_string 
				}]));
			}
		}

		Ok((0, candidates))
	}
}