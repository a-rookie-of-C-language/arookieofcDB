#[derive(Debug, Clone)]
pub struct BaseArgs {
    pub arg_name: String,
    pub arg_value: ArgsType,
    pub args: Args,
    pub description: String,
    pub help: String,
}

#[derive(Debug, Clone)]
pub enum ArgsType {
    Integer(i64),
    String(String),
    Boolean(bool),
    Float(f64),
}

#[derive(Debug, Clone, Copy)]
pub enum Args {
    Short,
    Long,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedArgs {
    pub args: Vec<BaseArgs>,
}

impl ParsedArgs {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn from_vec(args: Vec<BaseArgs>) -> Self {
        Self { args }
    }

    pub fn get(&self, name: &str) -> Option<&BaseArgs> {
        self.args.iter().find(|a| a.arg_name == name)
    }

    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|a| match &a.arg_value {
            ArgsType::String(s) => Some(s.as_str()),
            _ => None,
        })
    }

    pub fn get_integer(&self, name: &str) -> Option<i64> {
        self.get(name).and_then(|a| match &a.arg_value {
            ArgsType::Integer(i) => Some(*i),
            _ => None,
        })
    }

    pub fn get_boolean(&self, name: &str) -> Option<bool> {
        self.get(name).and_then(|a| match &a.arg_value {
            ArgsType::Boolean(b) => Some(*b),
            _ => None,
        })
    }

    pub fn get_float(&self, name: &str) -> Option<f64> {
        self.get(name).and_then(|a| match &a.arg_value {
            ArgsType::Float(f) => Some(*f),
            _ => None,
        })
    }
}
