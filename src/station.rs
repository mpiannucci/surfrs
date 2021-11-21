use crate::location::Location;

pub trait Station {
    fn id(&self) -> &str;
    fn location(&self) -> &Location;
    fn name(&self) -> String;
}