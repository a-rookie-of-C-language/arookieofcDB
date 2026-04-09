#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtlState {
    NotFound,
    NoExpire,
    Seconds(i64),
}