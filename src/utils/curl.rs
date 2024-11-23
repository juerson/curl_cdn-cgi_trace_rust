use crate::utils::models::Airport;
use crate::utils::models::Record;

use ipnetwork::IpNetwork;
use log::{ info, warn };
use std::{ io::{ self, Write }, process::{ Command, Stdio }, time::Instant };
use url::Url;

// 检查curl是否已安装，没有就退出程序
pub fn check_curl_installed() {
    if !Command::new("curl").arg("--version").output().is_ok() {
        print!("电脑中，没有安装有curl命令！按Enter键退出程序！");
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }
}

pub fn run_command_and_process_data(
    ip: &str,
    airports: Vec<Airport>,
    jetbrains: bool
) -> Result<Record, io::Error> {
    let formatted_ip = if let Ok(ip_network) = ip.parse::<IpNetwork>() {
        if ip_network.is_ipv6() { format!("[{}]", ip) } else { ip_network.ip().to_string() }
    } else {
        let trimmed_ip = if ip.ends_with('/') {
            ip.trim_end_matches('/').to_string()
        } else {
            ip.to_string()
        };
        trimmed_ip
    };
    let host_name = if formatted_ip.starts_with("http://") || formatted_ip.starts_with("https://") {
        let url_parse = Url::parse(&formatted_ip).unwrap(); // 解析URL
        let domain = url_parse.host_str().unwrap_or_default().to_string(); // 提取域名
        domain
    } else {
        formatted_ip
    };
    let url: String = match jetbrains {
        true => ip.to_string(),
        false => format!("http://{}/cdn-cgi/trace", host_name),
    };
    let start_time = Instant::now(); // 开始时间

    let curl_process = Command::new("curl")
        .args(["/dev/null", "-I", &url, "-s", "--connect-timeout", "3", "--max-time", "10"])
        .stdout(Stdio::piped())
        .spawn();

    // 检查curl进程是否成功启动
    match curl_process {
        Ok(child) => {
            let output = child.wait_with_output()?; // 等待子进程完成
            let elapsed_duration = start_time.elapsed(); // 结束时间
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            let mut status_code = String::new();
            for line in lines {
                if line.starts_with("HTTP/1.1") {
                    let parts: Vec<&str> = line.split(' ').collect();
                    if parts.len() >= 2 {
                        status_code = parts[1].to_string();
                    }
                } else if jetbrains == false && line.starts_with("CF-RAY:") {
                    if let Some(pos) = line.rfind('-') {
                        // 获取 `"-"` 后面的部分
                        let colo = &line[pos + 1..]; // +1 是为了跳过 `"-"` 字符
                        match airports.iter().find(|a| a.iata == colo) {
                            Some(airport) => {
                                let record = Record {
                                    ip: ip.to_string(),
                                    colo: colo.to_string(),
                                    country: airport.cca2.clone(),
                                    region: airport.region.clone(),
                                    city: airport.city.clone(),
                                    delay: elapsed_duration,
                                    http_status_code: status_code,
                                    is_jetbrains: false, // 这里没有扫描，不代表不是JetBrains的许可证服务器
                                };
                                info!(
                                    "{} | {} | {} | {} | {} | {} ms",
                                    ip,
                                    colo,
                                    airport.cca2,
                                    airport.region,
                                    airport.city,
                                    elapsed_duration.as_millis()
                                );
                                return Ok(record);
                            }
                            None => {}
                        }
                    }
                } else if
                    jetbrains == true &&
                    line.starts_with("Location: https://account.jetbrains.com/fls-auth")
                {
                    let record = Record {
                        ip: ip.to_string(),
                        colo: "".to_string(),
                        country: "".to_string(),
                        region: "".to_string(),
                        city: "".to_string(),
                        delay: elapsed_duration,
                        http_status_code: status_code,
                        is_jetbrains: true,
                    };
                    info!("{} | JetBrains License server | {}", ip, elapsed_duration.as_millis());
                    return Ok(record);
                }
            }
            // 都不符合条件的情况
            if jetbrains {
                warn!("{} | 连接失败/超时，响应头中，找不到jetbrains相关的fls-auth信息！", ip);
            } else {
                warn!("{} | 连接失败/超时，响应头中，找不到CloudFlare相关的信息！", ip);
            }
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "未知错误！"));
        }
        Err(_e) => {
            warn!("{} | CURL启动失败", ip);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "未知错误！"));
        }
    }
}
