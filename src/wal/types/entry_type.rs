#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum EntryType {
    Log,
    Checkpoint,
}

impl From<EntryType> for u8 {
    fn from(et: EntryType) -> u8 {
        match et {
            EntryType::Log => 0,
            EntryType::Checkpoint => 1,
        }
    }
}