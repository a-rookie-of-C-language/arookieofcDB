
pub struct BaseArgs {
    pub arg_name: String,
    pub arg_value: ArgsType,
    pub args: Args,
    pub description: String,
    pub help: String,
}

pub enum ArgsType {
    Integer,
    String,
    Boolean,
    Float,
}

pub enum Args {
    Short,
    Long,
}
