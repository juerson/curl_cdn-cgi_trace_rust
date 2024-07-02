use ipnetwork::IpNetwork;
use std::io;
use std::process::Command;
use std::process::Stdio;
use url::Url;

// 检查curl是否已经在电脑中安装好
pub fn is_curl_installed() -> bool {
    // 检查命令执行是否成功
    let output = Command::new("curl").arg("--version").output().is_ok();

    output
}

// 使用CURL命令（需要在电脑中安装curl才能使用，特别是windows系统中），判断headers头文件信息是否有cloudflare字符
pub fn check_server_is_cloudflare(ip: &str) -> Result<(String, bool), io::Error> {
    let formatted_ip = if let Ok(ip_network) = ip.parse::<IpNetwork>() {
        if ip_network.is_ipv6() {
            format!("[{}]", ip)
        } else {
            ip_network.ip().to_string()
        }
    } else {
        let trimmed_ip = if ip.ends_with('/') {
            // 用于去掉右侧"/"的字符
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
    let url = format!("http://{}/cdn-cgi/trace", host_name);

    let curl_process = Command::new("curl")
        .arg("/dev/null")
        .arg("-I")
        .arg(url)
        .arg("-s")
        .arg("-m")
        .arg("8") // 设置超时(单位：秒)
        .stdout(Stdio::piped())
        .spawn();

    // 检查curl进程是否成功启动
    match curl_process {
        Ok(child) => {
            let output = child.wait_with_output()?; // 等待子进程完成
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 如果curl输出中，server参数中是"cloudflare"
            if let Some(server_header) = extract_server_header(&stdout) {
                if server_header.contains("cloudflare") {
                    return Ok((host_name.to_string(), true));
                }
            }
            return Ok((host_name.to_string(), false));
        }
        Err(_e) => return Ok((host_name.to_string(), false)),
    }
}

// 提取响应头的server服务器是什么？cloudflare？
fn extract_server_header(curl_output: &str) -> Option<String> {
    let lines: Vec<&str> = curl_output.lines().collect();

    for line in lines {
        if line.starts_with("Server:") {
            // 删除前后的空格并返回值
            return Some(line["Server:".len()..].trim().to_string());
        }
    }

    None
}
