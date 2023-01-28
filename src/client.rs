use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;
use std::process::Command;

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

/// Hold the lines in a page
struct Page {
    lines: Vec<String>
}

/// Used for storing page info from reading
struct Book {
    pages: Vec<Page>
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

impl Page {
    pub fn new()->Self{
        Self{
            lines: Vec::new()
        }
    }

    pub fn insert_line(&mut self, line: String){
        self.lines.push(line);
    }
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

    /// Run cmd to run the file
    fn run_command_and_fill(&mut self, file_location: &str) -> Result<PathBuf> {
        let mut tmp_file = std::env::temp_dir();
        tmp_file.push("kplc_data");
        std::fs::create_dir_all(tmp_file.clone())?;
        //get random num
        let ran = uuid::Uuid::new_v4();
        tmp_file.push(format!("fs-{}.txt",ran.as_u128()));

        //show where file is
        dbg!("FILE: {}",tmp_file.to_str().unwrap());
        //show cmd
        let cmd = format!("pdftotext -layout {} {}",
            file_location, tmp_file.to_str().unwrap());
        Command::new("sh")
            .args(["-c", &cmd])
            .output().unwrap();

        Ok(tmp_file)
    }

    fn read_file_data_into_book(&mut self, file_path: &PathBuf) 
        -> Result<Book>{
        let begin_if_new_pg = "";
        let mut book = Book{ pages:Vec::new()};
        let file = File::open(file_path)?;
        let buff_reader = BufReader::new(file);

        let mut page = Page::new();
        //read lines, separating pages into books
        for l in  buff_reader.lines(){
            let line = l.unwrap();
            if line.contains(begin_if_new_pg) {
                book.pages.push(page);
                page = Page::new();
            }
            page.insert_line(line);
        }
        book.pages.push(page);
        Ok(book)
    }

    fn parse_book_for_kplc_data(&mut self, book: Book) -> Result<String> {

        for page in book.pages.iter(){
            if page.lines.len() <= 1 {
                //for last page which normally contains a blank line
                //skip doing anything on it.
                continue;
            }
            // variables check if page has a divide gap, and get where the 
            // start of the right collumn begins
            
            //used for checking if page has a gap
            //if so, then this will be greater than 3
            let mut biggest_gap = 0; 

            for l in page.lines.iter(){
                let mut gap_length = 0;
                for c in l.chars() {
                    // break since we now know this page has a divide
                    if biggest_gap >=3 {
                        break;
                    }
                    if c == ' ' {
                        gap_length+=1;
                    }else{
                        biggest_gap = gap_length;
                        gap_length = 0;
                    }
                }
                // break since we now know this page has a divide
                if biggest_gap >=3 {
                    break;
                }
            }


            //this is to get the collumn where the right side starts from
            let mut right_start_pos = 0;

            //means we have a split
            //so we get the beggining of the right collumn
            if biggest_gap>=3 {
                //now get the center point
                
                // to hold when we have reached a zone with 3 or more spaces
                let mut spaces = 0; 
                for l in page.lines.iter() {
                    for (i, c) in l.chars().enumerate() {
                        if c == ' '{
                            spaces+=1;
                        }else {
                            if spaces >=3 {
                                right_start_pos = i;
                                break;
                            }
                            spaces=0;
                        }
                    }
                    if right_start_pos>=3 {
                        break;
                    }
                }
            }

            println!("RIGHT collumn: {right_start_pos}");
        }
        Ok("".to_string())
    }

    /// This parses the data inside the file passed in at @file_location
    /// then stores it into the file_data field as a String,
    /// later on going ahead to parse the data to derive a KPLCData object
    pub async fn parse_file_as_data_object(&mut self, file_location: &str) -> Result<KPLCData>{
        let file_with_info = self.run_command_and_fill(file_location)?;

        let book = self.read_file_data_into_book(&file_with_info)?;
        let kplc_data = self.parse_book_for_kplc_data(book);

        Err(anyhow::Error::from(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "")))
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_reading_books(){
        let mut kplc = KPLCClient::new_offline();
        let path = kplc
            .run_command_and_fill("./test_files/23.06.2022.pdf").unwrap();
        let book = kplc.read_file_data_into_book(&path).unwrap();
        dbg!("BOOK_PAGES: {}",book.pages.len());
        assert!(book.pages.len()==3);
        //book.pages.iter().for_each(|p|{
        //    p.lines.iter().for_each(|l|{
        //        println!("{l}");
        //    })
        //})
    }

    #[tokio::test]
    async fn test_if_parse_success(){
        let mut client = KPLCClient::new_offline();
        let result = client.parse_file_as_data_object("./test_files/23.06.2022.pdf").await.unwrap();
        assert!(result.regions.len()>0);
    }
}
