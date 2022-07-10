use core::time;

use crate::data;
use crate::api::API;
use reqwest::{Client,header::HeaderMap, header::HeaderValue, Method};
use serde::{Serialize,Deserialize};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;

/// represent a single line on a page
type Line = String;

/// Represent a group of lines that are a page
type Page = HashMap<String,Vec<Line>>;

/// Represent a grou of pages
type Book = Vec<Page>;

const FREE_CONVERT_URL: &str = "https://api.freeconvert.com/v1/process";

pub struct FreeConvertAPI{
    client: Client,
    pdf_url: String
}

#[derive(Serialize,Deserialize,Debug)]
struct FreeConvertImportFromUrl{
    url: String
}

#[derive(Serialize,Deserialize,Debug)]
struct FreeConvertPDF2TXT{
    input: String,
    input_format: String,
    output_format: String
}

type FreeConvertDownloadUrl = FreeConvertImportFromUrl;

#[derive(Serialize,Deserialize,Debug)]
struct FreeConvertDownloadLink{
    status: String,
    result: Option<FreeConvertDownloadUrl>
}

#[derive(Serialize,Deserialize,Debug)]
struct FreeConvertDownloadTask{
    input: Vec<String>,
    filename: String,
    archive_multiple_files: bool
}

#[derive(Serialize,Deserialize,Debug)]
struct FreeConvertUploadTask{
    id: String
}

impl FreeConvertAPI{
    pub fn new(key: String, pdf_url: String)-> Self{
        let mut headers = HeaderMap::new();
        headers.insert("Accept",
                       HeaderValue::from_str("application/json").unwrap());
        headers.insert("Authorization",
                       HeaderValue::from_str(
                           &format!("Bearer {}",key))
                       .unwrap());
        
        let client = Client::builder()
            .default_headers(headers).build().unwrap();
        Self {client, pdf_url}
    }

    pub fn set_pdf_url(&mut self, pdf_url: String){
        self.pdf_url = pdf_url;
    }

    /// Used to get pages storing each line
    fn get_pages(&self ,data: &str)-> Book{
        let mut pages = Book::new();

        // a is left side of page
        let mut page_side_a: Vec<Line> = Vec::new();
        // b is right side of page
        let mut page_side_b: Vec<Line> = Vec::new();


        let is_second_page_str = "Interruption of";
        let mut middle_page=0;

        data.split("\n").collect::<Vec<&str>>()
            .iter().for_each(|line|{
            if line.contains("") {
                // meaning we are now in a new page
                let mut page = Page::new();
                page.insert(String::from("a"),page_side_a.clone());
                page.insert(String::from("b"),page_side_b.clone());
                pages.push(page);
                page_side_a.clear();
                page_side_b.clear();
                // reset to mark begining of new page
                middle_page=0;
            }

            
            if middle_page == 0{
                // means we are in top of a brand new page,
                // so get center split
                // of the columns
                if line.contains(is_second_page_str) {
                    // meaning it isn't the first page
                    let index = is_second_page_str.len();
                    // mark the middle, but excluding the pre index section
                    let mut  middle_after_index= 0;

                    line[index..].chars().for_each(|c|{
                        if c.is_whitespace(){
                            middle_after_index+=1;
                        }
                    });
                    //println!("index: {}",index);
                    //println!("middle_after_index: {}",middle_after_index);
                    middle_page = index+middle_after_index;

                    middle_page-=3;// update due to offset
                    let part_b = line[middle_page..].to_string();

                    //println!("POS: {}",middle_page);
                    //println!("PARTB: {}",part_b);
                    
                    page_side_b.push(part_b);
                    page_side_a.push(String::from(is_second_page_str));
                }else if line.len()>1 {
                    // meaning this is the first page
                    // which has an empty side a
                    line.chars().for_each(|c|{
                        if c.is_whitespace(){
                            middle_page+=1;
                        }
                    });
                    middle_page-=2;//update due to offset
                    let part_b = line[middle_page..].to_string();

                    //println!("POS: {}",middle_page);
                    //println!("PARTB: {}",part_b);
                    
                    page_side_b.push(part_b);
                    page_side_a.push(String::default());
                }
            }else if line.is_empty()==false {
                // meaning we aren't at the top of the pages
                // but already know where the split at the middle 
                // occurs
                if line.len()> middle_page {
                    page_side_a.push(line[..middle_page].to_string());
                    page_side_b.push(line[middle_page..].to_string());
                }else{
                    page_side_a.push(line.clone().to_string());
                    page_side_b.push(String::default());
                }
            }
        });


        pages
    }

    /// Can only parse Date object from String with the format:
    /// <br>
    ///DATE: Sunday 19.06.2022                    TIME: 9.00 A.M. ­ 5.00 P.M.<br>
    ///
    ///or matches the regex (\w+\.*\w+)
    fn parse_date(&self,date_line: &str)-> Result<data::Date,&dyn std::error::Error>{
        lazy_static::lazy_static!(
            static ref DATE_REGEX: Regex = {
                Regex::new(r"(\w+\.*\w+)").unwrap()
            };
        );
        let capture = DATE_REGEX
            .find_iter(date_line).map(|f| f.as_str()).collect::<Vec<&str>>();
        let mut from = capture[5].to_string();
        from.push_str(" ");
        from.push_str(&capture[6]);
        let mut to = capture[7].to_string();
        to.push_str(" ");
        to.push_str(&capture[8]);
        Ok(data::Date{
            day: capture[1].to_string(),
            day_date: capture[2].split(".").collect::<Vec<&str>>().get(0).unwrap().parse::<usize>().unwrap(),
            month_date: capture[2].split(".").collect::<Vec<&str>>().get(1).unwrap().parse::<usize>().unwrap(),
            year: capture[3].parse().unwrap(),
            interval: data::Interval { 
                from,
                to 
            }
        })
    }
}

#[async_trait]
impl API for FreeConvertAPI{
    async fn get_json(&self)-> Result<String, std::io::Error> {
        let url_upload_json = FreeConvertImportFromUrl{
            url: self.pdf_url.clone()
        };
        let req = self.client.request(Method::POST,
                            format!("{}/import/url",FREE_CONVERT_URL))
                    .json(&url_upload_json).build().unwrap();

        let resp = self.client.execute(req).await.unwrap();
        let upload_task_id = resp.json::<FreeConvertUploadTask>().await.unwrap().id;


        tokio::time::sleep(time::Duration::from_millis(500)).await;

        let converter_laod = FreeConvertPDF2TXT{
            input: upload_task_id.clone(),
            input_format: String::from("pdf"),
            output_format: String::from("txt") 
        };

        let req = self.client.request(Method::POST, 
                    format!("{}/convert",FREE_CONVERT_URL))
                .json(&converter_laod).build().unwrap();
        let resp = self.client.execute(req).await.unwrap();
        let convert_task_id = resp.json::<FreeConvertUploadTask>().await.unwrap().id;

        tokio::time::sleep(time::Duration::from_millis(500)).await;

        //create donwload/export access
        let mut inputs = Vec::default();
        inputs.push(convert_task_id);
        let download_task_load = FreeConvertDownloadTask{
            input: inputs,
            filename: String::from("Temp1"),
            archive_multiple_files: false
        };
        let req = self.client.request(Method::POST, 
                    format!("{}/export/url",FREE_CONVERT_URL))
                .json(&download_task_load).build().unwrap();
        let download_task_id = self.client.execute(req).await.unwrap().
            json::<FreeConvertUploadTask>().await.unwrap().id;

        tokio::time::sleep(time::Duration::from_millis(500)).await;

        //check download link
        while self.client.execute(self.client.request(Method::GET, 
                    format!("{}/tasks/{}",FREE_CONVERT_URL,download_task_id))
                .build().unwrap()).await.unwrap()
                .json::<FreeConvertDownloadLink>().await.unwrap().status != "completed"{
            tokio::time::sleep(time::Duration::from_millis(500)).await;
        }
        let resp = self.client.execute(self.client.request(Method::GET, 
                    format!("{}/tasks/{}",FREE_CONVERT_URL,download_task_id))
                .build().unwrap()).await.unwrap();
        let download_url = resp .json::<FreeConvertDownloadLink>()
                 .await.unwrap().result.unwrap().url;

        tokio::time::sleep(time::Duration::from_secs(2)).await;

        let mut header_map = HeaderMap::default();
        header_map.insert("Accept",
                          HeaderValue::from_str("text/*")
                          .unwrap());
        header_map.insert("User-Agent",
                          HeaderValue::from_str("curl/7.79.1")
                          .unwrap());
        let download_req = self.client
            .request(Method::GET, download_url).headers(header_map).build().unwrap();
        println!("URL: {}",download_req.url().as_str());
        let data = self
            .client.execute(download_req).await.unwrap().text().await.unwrap();

        Ok(data)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use tokio::join;

    fn setup_free_convert(url: &str)-> FreeConvertAPI{
        dotenv().ok();
        let key = std::env::var("FREECONV_KEY").unwrap();
        FreeConvertAPI::new(key, String::from(url))
    }

    //#[tokio::test]
    //async fn test_geting_txt_from_pdf() {
    //    let free_convert = setup_free_convert("https://www.kplc.co.ke/img/full/Interruptions%20-%2016.06.2022.pdf");
    //    let free_convert2 = setup_free_convert("https://www.kplc.co.ke/img/full/Interruptions%20-%2023.06.2022.pdf");

    //    let (pdf_txt1,pdf_txt2) = join!(free_convert.get_json(),free_convert2.get_json());

    //    let pdf_txt1 = pdf_txt1.unwrap();
    //    let pdf_txt2 = pdf_txt2.unwrap();

    //    println!("1:\n {}",pdf_txt1);
    //    println!("2:\n {}",pdf_txt2);
    //    assert!(pdf_txt1 != pdf_txt2);
    //}

    #[test]
    fn test_parsing_date(){
        let to_be_parsed = 
            String::from("DATE: Sunday 19.06.2022                    TIME: 9.00 A.M. ­ 5.00 P.M.");
        let free_convert = setup_free_convert("https://www.kplc.co.ke/img/full/Interruptions%20-%2016.06.2022.pdf");
        let date = free_convert.parse_date(&to_be_parsed).unwrap();
        assert!(date.day == String::from("Sunday"));
        assert!(date.day_date == 19);
        assert!(date.month_date == 6);
        assert!(date.year == 2022);
        assert!(date.interval.from == String::from("9.00 A.M"));
        assert!(date.interval.to == String::from("5.00 P.M"));
    }

    /// Used to find when we are at the end of a page, and beginning a new one.
    /// test file @ ../test_files/Temp1 contains a text translation of
    /// ../test_files/23.06.2022.pdf from freeconvert.com
    /// which only has a total of two pages.
    #[test]
    fn test_getting_end_of_page(){
        let lines = std::fs::read_to_string("./test_files/23.06.2022.txt")
            .unwrap();
        let free_convert = setup_free_convert("https://www.kplc.co.ke/img/full/Interruptions%20-%2016.06.2022.pdf");
        let book = free_convert.get_pages(&lines);
        assert!(book.len() == 2);
        assert!(book.get(0).unwrap().get("a").unwrap().len()>4);
        
        print(&book);
    }

    /// just for printing the book data, user can parse to
    /// see if code aligns
    fn print(book:& Book){
        book.iter().for_each(|page|{
            let a = page.get("a").unwrap();
            let b = page.get("b").unwrap();

            a.iter().for_each(|line|{
                println!("{}",line);
            });
            b.iter().for_each(|line|{
                println!("{}",line);
            });

            println!("\n\n");
        });
    }
}
