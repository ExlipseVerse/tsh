use std::fs::{File, OpenOptions};

pub enum Redirection {
    Stdout(File),
    Stderr(File),
}

pub fn extract_redirection(args: &[String]) -> (Vec<&str>, Option<Redirection>) {
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