#[allow(unused_imports)]
use std::io::{self, Write, Read};

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

        let command: Vec<&str> = input.split_whitespace().collect();
        if command.first().is_none() {
            continue;
        } else if command[0] == "echo" {
            println!("{}", command[1..].join(" "));
        } else {
            println!("{}: command not found", input.trim());
        }
        
    }
}
