pub mod chart;
pub mod phrases;

pub trait TimestampedEvent {
    fn get_timestamp(&self) -> u32;
}
