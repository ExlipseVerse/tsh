#[allow(unused_imports)]
use std::io::{self, Write, Read};

fn main() {
    // TODO: Uncomment the code below to pass the first stage
    print!("$ ");

    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    
    println!("{}: command not found", input.trim());
}
