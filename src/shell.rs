use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use std::process::Child;

pub struct Job {
	pub id: usize,
    pub pid: u32,
    pub cmd_string: String,
    pub child: Child
}

pub struct Shell {
	pub jobs: Vec<Job>,
	pub completions: Arc<Mutex<HashMap<String, String>>>,
	pub history: Vec<String>,
	pub history_index: usize,
}

impl Shell {
	pub fn new(completions: Arc<Mutex<HashMap<String, String>>>) -> Self {
		Self { jobs: Vec::new(), completions, history: Vec::new(), history_index: 0, }
	}
}