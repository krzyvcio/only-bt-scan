use only_bt_scan::run;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    run().await
}
