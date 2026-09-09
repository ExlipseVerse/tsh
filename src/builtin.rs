
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltIn {
    Echo,
    Type,
    Exit,
    Pwd,
    Cd,
}

impl BuiltIn {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "echo" => Some(BuiltIn::Echo),
            "type" => Some(BuiltIn::Type),
            "exit" => Some(BuiltIn::Exit),
            "pwd" => Some(BuiltIn::Pwd),
            "cd" => Some(BuiltIn::Cd),
            _ => None,
        }
    }
}