use geojson::Feature;

use crate::location::Location;

pub trait Station {
    fn id(&self) -> &str;
    fn location(&self) -> &Location;
    fn name(&self) -> String;
    fn as_feature(&self) -> Feature;
}