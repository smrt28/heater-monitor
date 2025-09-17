use regex::Regex;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = reqwest::get("http://192.168.6.75/").await?.text().await?;

    let re = Regex::new(r"teplota:\s*<b>\s*(\d+\.\d+)\s*%\s*(\d+\.\d+)\s*&deg;C").unwrap();

    if let Some(caps) = re.captures(&text) {
        let percent: f64 = caps[1].parse().unwrap();
        let celsius: f64 = caps[2].parse().unwrap();
        println!("percent = {}, celsius = {}", percent, celsius);
    } else {
        println!("error");
    }

    Ok(())
}
