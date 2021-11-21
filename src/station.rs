use crate::location::Location;

pub trait Station {
    fn id(&self) -> String;
    fn location(&self) -> Location;
    fn name(&self) -> String;
}