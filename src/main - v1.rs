extern crate fern;
extern crate chrono;
extern crate threadpool;
extern crate ipnetwork;
extern crate rand;

use std::fs::File;
use std::io::{self, BufRead, Write};
use std::net::IpAddr;
use ipnetwork::IpNetwork;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use rand::seq::SliceRandom;
// use std::net::ToSocketAddrs;



fn init_logger() -> Result<(), fern::InitError> {
    // 初始化日志
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} {:<5}{}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(io::stdout())
        .apply()?;
    Ok(())
}

fn is_ip_reachable(ip: IpAddr) -> Result<(IpAddr, bool), Box<dyn std::error::Error>> {
    let ip_str = match ip {
        IpAddr::V4(ipv4) => ipv4.to_string(),
        IpAddr::V6(ipv6) => format!("[{}]", ipv6),
    };
    let url = format!("http://{}/cdn-cgi/trace", ip_str);
    println!("{}",url);
    let output = Command::new("curl")
        .arg("-o")
        .arg("/dev/null")
        .arg("-s")
        .arg("-w")
        .arg("%{http_code}")
        .arg("-m")
        .arg("5") // 设置超时时间为5秒
        .arg(&url)
        .output()?;
    let response_code = String::from_utf8_lossy(&output.stdout);

    Ok((ip, response_code.trim() == "200"))
}


// fn generate_ip_and_check_ip_type(ip_address: &str) -> Vec<IpAddr> {
    // match ip_address.parse::<IpAddr>() {
        // Ok(ip) => vec![ip],
        // Err(_) => ip_address
            // .parse::<ipnetwork::IpNetwork>()
            // .ok()
            // .map_or_else(Vec::new, |ip_network| {
                // ip_network.iter().collect()
            // }),
    // }
// }

fn generate_ip_and_check_ip_type(ip_address: &str) -> Vec<IpAddr> {
    
    // Try parsing as IP network
    if let Ok(ip_network) = ip_address.parse::<IpNetwork>() {
        if ip_network.is_ipv6() {
            // Handle IPv6 CIDR networks
            return ip_network
                .iter()
                .take(500) // Limit to 500 IPs or the number of IPs in the CIDR range, whichever is smaller
                .collect();
        } else {
            // Handle IPv4 CIDR networks
            return ip_network.iter().collect();
        }
    }

    // Try parsing as IP address
    if let Ok(ip) = ip_address.parse::<IpAddr>() {
        return vec![ip];
    } 
    
    Vec::new()
}



fn read_ips_file(file_path: &str) -> Result<Vec<String>, io::Error> {
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(e) => {
            println!("打开{}文件错误，错误原因是:{}", file_path, e);
            print!("按Enter键退出程序！");
            io::stdout().flush().expect("Failed to flush stdout");
            let _ = io::stdin().read_line(&mut String::new());
            std::process::exit(1);
        }
    };
    let ips: Vec<String> = io::BufReader::new(file)
        .lines()
        .filter_map(|line| line.ok())
        .collect();
    if ips.is_empty() {
        print!("{}文件是空的，按Enter键退出程序！", file_path);
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }
    Ok(ips)
}

fn write_to_file(data: &[IpAddr], file_name: &str) -> Result<(), io::Error> {
    let mut file = File::create(file_name)?;
    for ip in data {
        writeln!(file, "{}", ip)?;
    }
    Ok(())
}

fn format_duration(duration: Duration) -> (f64, &'static str) {
    if duration.as_secs() > 0 {
        // If duration is in seconds or more
        (duration.as_secs_f64(), "秒")
    } else if duration.as_millis() > 0 {
        // If duration is in milliseconds
        (duration.as_millis() as f64, "毫秒")
    } else if duration.as_micros() > 0 {
        // If duration is in microseconds
        (duration.as_micros() as f64, "微秒")
    } else {
        // If duration is in nanoseconds
        (duration.as_nanos() as f64, "纳秒")
    }
}


fn wait_for_enter() {
    print!("按Enter键，退出程序！");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    // 初始化日志
    init_logger()?;

    // 读取文件中的内容
    let ip_addresses = read_ips_file("ips-v4.txt")?;

    /* 第一个线程池：多线程地生成CIDR段中的所有IP，减少生成IP的时间 */
    let pool_generate = threadpool::ThreadPool::new(20);
    let (tx_generate, rx_generate) = mpsc::channel();

    // 提交连接任务
    for item in ip_addresses.iter() {
        let tx_generate = tx_generate.clone();
        let cloned_item = item.clone(); // Clone the item
        pool_generate.execute(move || {
            let ips = generate_ip_and_check_ip_type(&cloned_item); // Use the cloned item
            tx_generate.send(ips).unwrap();
        });
    }

    drop(tx_generate); // 释放发送端，以便后续迭代

    let mut ips = Vec::new();
    for ips_batch in rx_generate.iter() {
        ips.extend(ips_batch);
    }

    let mut rng = rand::thread_rng();
    ips.shuffle(&mut rng);
    
    
    println!("开始扫描能用的CF CDN...\n");
    
    
    /* 第二个线程池：用于执行curl命令函数 */
    let pool_method = threadpool::ThreadPool::new(200);
    let (tx_method, rx_method) = mpsc::channel();

    // 提交连接任务
    for ip in ips.iter() {
        let tx_method = tx_method.clone();
        let ip = *ip;
        pool_method.execute(move || {
            if let Ok((ip, state)) = is_ip_reachable(ip) {
                log::info!("scan {:?} --> {}", ip, state);
                if state {
                    tx_method.send(ip).unwrap();
                }
            }
        });
    }

    drop(tx_method); // 释放发送端，以便后续迭代

    let mut reachable_ips = Vec::new();
    for ip in rx_method.iter() {
        reachable_ips.push(ip);
    }
    
    reachable_ips.sort();
    
    // 将结果写入文本文件中
    write_to_file(&reachable_ips, "output.txt")?;

    let end_time = Instant::now();
    let elapsed_duration = end_time.duration_since(start_time);

    let (elapsed_time, unit) = format_duration(elapsed_duration);

    println!("\n程序耗时：{:.2} {}", elapsed_time, unit);
    
    wait_for_enter();
    
    Ok(())
}