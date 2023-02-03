use kplc_outage_parser::prelude::*;

#[tokio::main]
async fn main(){
    let link = "https://www.kplc.co.ke/img/full/Interruptions%20-%2026.01.2023.pdf";
    println!("Parsing data from {link}");
    let mut client = KPLCClient::new();
    let kplcdata_obj = client.parse_from_web(link).await.unwrap();
    println!("{:?}",kplcdata_obj);
}
