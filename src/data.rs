use serde::{Serialize,Deserialize};

/// Mark Neighbourhood or regions near that specific location
pub type Neighbourhood = String;
/// Mark region/area greater than Neighbourhood
pub type Area = String;
/// Mark region greater than Area
pub type County = String;

#[derive(Serialize,Deserialize, Debug)]
pub struct Date{
    /// Mark day of the week
    pub day: String,
    /// Mark day of the month
    pub day_date: usize,
    /// Mark month of the year
    pub month_date: usize,
    /// Mark year
    pub year: usize,
    /// Hold intervale
    pub interval: Interval
}

#[derive(Serialize,Deserialize, Debug)]
pub struct Interval{
    pub from: String,
    pub to: String
}
