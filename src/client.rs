use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;

const COMMAND: &str = "pdftotext -layout {}";

pub struct KPLCClient{
    web_client: Option<Client>,
    file_data: Option<Vec<String>>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Region{
    parts: Vec<Part>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Part{
    areas: Vec<Area>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Date{
    day: u32,
    month: u32,
    year: u32
}


#[derive(Deserialize, Serialize, Debug)]
pub struct Time{
    start: String,
    end: String
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Area{
    places: Vec<String>,
    date: Date,
    time: Time
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KPLCData{
    regions: Vec<Region>
}

impl KPLCClient{
    /// Creates a KPLCClient:
    /// <b>NOTE:</b> The the client can only handle storing one
    /// file data at a time, hence if you try read data from web or offline
    /// multiple times it will only hold data from the latest read.
    pub fn new()->Self{
        Self{
            web_client: Some(Client::new()),
            file_data: None
        }
    }

    /// For use with testing only, because we use files from local system
    #[cfg(test)]
    pub fn new_offline()-> Self{
        Self{
            web_client: None,
            file_data: None
        }
    }


    /// This parses the data inside the file passed in at @file_location
    /// then stores it into the file_data field as a String,
    /// later on going ahead to parse the data to derive a KPLCData object
    pub async fn parse_file_as_data_object(&mut self, file_location: &str) -> Result<KPLCData>{
	Err(anyhow::Error::from(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "")))
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[tokio::test]
    async fn test_if_parse_success(){
	let mut client = KPLCClient::new_offline();
	let result = client.parse_file_as_data_object("./test_files/23.06.2022.pdf").await.unwrap();
	assert!(result.regions.len()>0);
    }
}
