use core::time;

use crate::api::API;
use reqwest::{Client,header::HeaderMap, header::HeaderValue, Method};
use serde::{Serialize,Deserialize};
use async_trait::async_trait;

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

    #[tokio::test]
    async fn test_geting_txt_from_pdf() {
        dotenv().ok();
        let key = std::env::var("TROLL_KEY").unwrap();
        //url for 16/06/2022
        let pdf_url1 = String::from("https://www.kplc.co.ke/img/full/Interruptions%20-%2016.06.2022.pdf");
        let pdf_url2 = String::from("https://www.kplc.co.ke/img/full/Interruptions%20-%2023.06.2022.pdf");

        let free_convert = FreeConvertAPI::new(key.clone(), pdf_url1);
        let free_convert2 = FreeConvertAPI::new(key.clone(), pdf_url2);
        let pdf_txt1 = free_convert.get_json().await.unwrap();
        let pdf_txt2 = free_convert2.get_json().await.unwrap();

        println!("1:\n {}",pdf_txt1);
        println!("2:\n {}",pdf_txt2);
        assert!(pdf_txt1 != pdf_txt2);
    }
}
