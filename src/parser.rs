pub fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new(); // we create an array []
    let mut current_arg = String::new(); // we create a string to store the current argument
    let mut in_single_quotes = false; // we create a boolean to check if we are in single quotes
    let mut in_double_quotes = false;
    let mut has_content = false; // we create a boolean to check if we have content in the current argument
    let mut is_escaped = false;


    for c in input.chars() { // by for example a, b, c

        if is_escaped {
            if in_double_quotes {
                match c {
                    '"' | '\\' => {    
                        current_arg.push(c);
                    }

                    _ => {
                        current_arg.push('\\');
                        current_arg.push(c);
                    }
                }
            } else {
                current_arg.push(c);
            }
            
            has_content = true;
            is_escaped = false;
            continue
        }
        
        match c {
            '\\' => {
                if in_single_quotes {
                    current_arg.push(c);
                } else {
                    is_escaped = true;
                }
            }

            '\'' => {
                if in_double_quotes {
                    current_arg.push(c);
                } else {
                    in_single_quotes = !in_single_quotes;
                    has_content = true;
                }
            }

            '"' => {
                if in_single_quotes {
                    current_arg.push(c);
                } else {
                    in_double_quotes = !in_double_quotes;
                    has_content = true;
                }
            }

            ' ' | '\t' | '\n' | '\r' => {
                if in_single_quotes || in_double_quotes {
                    current_arg.push(c);
                } else {
                    if has_content || !current_arg.is_empty() {
                        args.push(current_arg.clone());
                        current_arg.clear();
                        has_content = false;
                    }
                }
            }

            _ => {
                current_arg.push(c);
                has_content = true;
            }
        }

    }

    if is_escaped {
        current_arg.push('\\');
    }
    if has_content || !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}