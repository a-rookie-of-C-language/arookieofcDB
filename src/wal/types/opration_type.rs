#[derive(Copy, Clone)]
pub enum OprationType {
    Insert,
    Delete,
    Update,
}

impl From<OprationType> for u8 {
    fn from(op: OprationType) -> u8 {
        match op {
            OprationType::Insert => 0,
            OprationType::Delete => 1,
            OprationType::Update => 2,
        }
    }
}