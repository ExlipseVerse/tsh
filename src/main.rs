mod builtin;

#[allow(unused_imports)]
use std::io::{self, Write, Read};
use builtin::BuiltIn;

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
        let parsed_cmd = command.first().and_then(|&cmd| BuiltIn::from_str(cmd));

        if let Some(cmd_name) = command.first() {
            match BuiltIn::from_str(cmd_name) {
                Some(BuiltIn::Echo) => {
                    if command.len() < 2 {
                        println!("missing argument");
                    } else {
                        println!("{}", command[1..].join(" "));
                    }
                }
                Some(BuiltIn::Type) => {
                    if let Some(sec_cmd) = command.get(1) {
                        if BuiltIn::from_str(sec_cmd).is_some() {
                            println!("{} is a shell builtin", sec_cmd);
                        } else {
                            println!("{} not found", sec_cmd);
                        }
                    } else {
                        println!("missing argument");
                    }
                }
                Some(BuiltIn::Exit) => {
                    break;
                }

                None => {
                    println!("{}: command not found", cmd_name);
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
