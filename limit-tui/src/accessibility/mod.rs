#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveRegion {
    Polite,
    Assertive,
    Off,
}

pub trait Accessible {
    fn accessible_label(&self) -> String;

    fn should_announce(&self) -> bool;

    fn live_region(&self) -> LiveRegion {
        LiveRegion::Polite
    }
}
