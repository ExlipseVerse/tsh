use crate::builtin::BuiltIn;
use std::env;
use std::fs::{read_dir, metadata};
use std::io::{self, Write};
use std::sync::Mutex;
use rustyline::completion::{Completer, Pair, FilenameCompleter, longest_common_prefix};
use rustyline::Context;
use rustyline::Result;
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::Helper;


pub struct ShellHelper;


impl Helper for ShellHelper {}
impl Highlighter for ShellHelper {}
impl Validator for ShellHelper {}
impl Hinter for ShellHelper {
	type Hint = String;
}

// impl ShellHelper {
// 	pub fn new() -> Self {
// 		Self {
// 			last_completion: Mutex::new(None),
// 		}
// 	}
// }

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
// !matches[0].replacement.ends_with(std::path::MAIN_SEPARATOR)
		if line[..pos].contains(' ') {
			let (start, mut matches) = FilenameCompleter::new().complete_path(line, pos)?;
			if matches.len() == 1 {
                matches[0].replacement.push(' ');
            }

            return Ok((start, matches));
		}

		

		let builtins = [
			BuiltIn::Cd,
			BuiltIn::Pwd,
			BuiltIn::Echo,
			BuiltIn::Exit,
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

		

		// if candidates.len() == 1 {
		// 	let mut last = self.last_completion.lock().unwrap();
		// 	*last = None;
		// 	return Ok((0, candidates));
		// }

		// let mut last = self.last_completion.lock().unwrap();
		// let curr_count = match &*last {
		// 	Some((prev_p, count)) if prev_p == prefix => count + 1,
		// 	_ => 1
		// };

		// *last = Some((prefix.to_string(), curr_count));


		// if curr_count == 1 {
		// 	print!("\x07");
        //     let _ = io::stdout().flush();
        //     return Ok((0, Vec::new()));
		// } else {
		// 	println!();

		// 	let names: Vec<String> = candidates.iter().map(|c| c.display.clone()).collect();
		// 	println!("{}", names.join(" "));

		// 	*last = None;

		// 	return Ok((0, Vec::new()));
		// }

		Ok((0, candidates))
	}
}