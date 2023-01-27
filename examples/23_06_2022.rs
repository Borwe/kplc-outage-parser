use kplc_outage_parser::prelude::*;

#[tokio::main]
async fn main(){
    println!("Parsing data from ./test_files/23.06.2022.pdf");
    let mut client = KPLCClient::new();
    let kplcdata_obj = client
        .parse_file_as_data_object("./test_files/23.06.2022.pdf")
        .await.unwrap();
    println!("Data: {:?}",kplcdata_obj);
}
