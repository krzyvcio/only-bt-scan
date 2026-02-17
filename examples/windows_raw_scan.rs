use log::info;
use only_bt_scan::windows_hci::WindowsHciScanner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting Windows Raw HCI Scan Example");

    let mut scanner = WindowsHciScanner::new("HCI0".to_string());
    scanner.start_scan(true).await?;

    info!("Scanning for advertisements...");

    for _ in 0..100 {
        // Scan for 100 advertisements
        match scanner.receive_advertisement().await {
            Ok(Some(report)) => {
                info!("Received advertisement: {:?}", report);
            }
            Ok(None) => {
                // This can happen if the event is not an advertising report
                continue;
            }
            Err(e) => {
                // Timeouts are expected if no data is received
                if format!("{}", e).contains("timed out") {
                    continue;
                }
                eprintln!("Error receiving advertisement: {}", e);
                break;
            }
        }
    }

    scanner.stop_scan().await?;
    info!("Scan finished.");

    Ok(())
}
