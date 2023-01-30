use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;
use std::process::Command;

const COMMAND: &str = "pdftotext -layout {}";
/// Contains strings to be ignored when parsing pdfs
/// as they don't contain any real data
const CONST_STRINGS_TO_IGNORE: [&str;12] = [
    "Interruption of",
    "Electricity Supply",
    "Notice is hereby given under Rule 27 of the Electric Power Rules",
    "That the electricity supply will be interrupted as here under:",
    "(It is necessary to interrupt supply periodically in order to",
    "facilitate maintenance and upgrade of power lines to the network;",
    "to connect new customers or to replace power lines during road",
    "construction, etc.)",
    "For further information, contact",
    "the nearest Kenya Power office",
    "Interruption notices may be viewed at",
    "www.kplc.co.ke",
];

//hold strings
const REGION: &str = "REGION";
const PARTS_OF: &str = "PARTS OF ";

pub struct KPLCClient{
    web_client: Option<Client>,
    file_data: Option<Vec<String>>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Region{
    region: String,
    parts: Vec<Part>
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Part{
    part:  String,
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

impl KPLCData {
    pub fn new()-> Self{
        Self{
            regions: Vec::new()
        }
    }

    pub fn insert_region(&mut self,region: String) {
        self.regions.push(Region{
            region,
            parts: Vec::new()
        });
    }

    pub fn insert_part_to_prev_region(&mut self, part: String){
        self.regions.last_mut().unwrap().parts.push(Part{
            part,
            areas: Vec::new()
        });
    }
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

    fn parse_book_for_kplc_data(&mut self, book: Book) -> Result<KPLCData> {

        let mut kplc_data = KPLCData::new();
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


            //use split to make it a single column page with no splits
            //if there is a split, otherwise this should just be the 
            //same as the lines in page field
            let mut lines: Vec<&str> = page.lines.iter()
                .map(|x| x.as_str()).collect();

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

                //make lines to take into account splits on the right as
                //another set of lines
                lines.clear();
                for l in page.lines.iter(){
                    if l.len() > right_start_pos{
                        let left = &l[0..right_start_pos];
                        lines.push(left);
                    }else{
                        lines.push(l);
                    }
                }
                for l in page.lines.iter(){
                    if l.len() > right_start_pos{
                        let right = &l[right_start_pos..];
                        lines.push(right);
                    }
                }
            }

            let filtered_lines: Vec<&&str> = lines.iter().filter(|l|{
                //skip lines with nothing in them
                if l.trim().is_empty() {
                    return false;
                }
                //remove bloat lines
                for f in  CONST_STRINGS_TO_IGNORE{
                    if l.contains(f) {
                        return false;
                    }
                }
                true
            }).collect();

            let mut l_itr = filtered_lines.iter();
            loop {
                let l_option = l_itr.next();
                if l_option.is_none() {
                    //if there is no more items, exit
                    break;
                }
                let l = l_option.unwrap();

                //check for REGION key word, means we are now starting
                //a new region and then continue to new line
                if l.contains(REGION) {
                    let region = l.replace(REGION,"");
                    kplc_data.insert_region(region);
                    continue;
                }
                
                //check for PARTS keyword, then add part to current top
                //region and continue to next line.
                if l.contains(PARTS_OF) {
                    let part = l.replace(PARTS_OF,"");
                    kplc_data.insert_part_to_prev_region(part);
                }

                //check for AREA keyword, means the next lines are all for
                //area information
            }

            #[cfg(test)]
            {
                println!("RIGHT collumn: {right_start_pos}");
                //show pages as single column
                //filtered_lines.iter().for_each(|l|{
                //    println!("{l}");
                //});
            }
        }
        Ok(kplc_data)
    }

    /// This parses the data inside the file passed in at @file_location
    /// then stores it into the file_data field as a String,
    /// later on going ahead to parse the data to derive a KPLCData object
    pub async fn parse_file_as_data_object(&mut self, file_location: &str) -> Result<KPLCData>{
        let file_with_info = self.run_command_and_fill(file_location)?;
        let book = self.read_file_data_into_book(&file_with_info)?;
        self.parse_book_for_kplc_data(book)
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
        //assert!(result.regions.len()>0);
    }
}
