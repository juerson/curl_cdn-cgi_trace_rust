extern crate fern;
extern crate chrono;
extern crate threadpool;
extern crate ipnetwork;
extern crate rand;
extern crate url;

use std::fs::File;
use std::io::{self, BufRead, Write};
use std::net::IpAddr;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::net::Ipv6Addr;
use std::str::FromStr;
use ipnetwork::IpNetwork;
use url::Url;
use rand::Rng;
use rand::seq::SliceRandom;

// 初始化日志（设置日志格式）
fn init_logger() -> Result<(), fern::InitError> {
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

// 检查curl是否已经在电脑中安装好
fn is_curl_installed() -> bool {
    let output = Command::new("curl")
        .arg("--version")
        .output()
        .is_ok(); // 检查命令执行是否成功

    output
}

// 使用CURL命令（需要在电脑中就按照有curl才能使用），去检查url链接的状态码情况
fn is_ip_reachable(ip: &str) -> Result<(String, bool), Box<dyn std::error::Error>> {
    let formatted_ip = if let Ok(ip_network) = ip.parse::<IpNetwork>() {
        if ip_network.is_ipv6() {
            format!("[{}]", ip)
        } else {
            ip_network.ip().to_string()
        }
    } else {
        let trimmed_ip = if ip.ends_with('/') { // 用于去掉右侧“/”的字符
            ip.trim_end_matches('/').to_string()
        } else {
            ip.to_string()
        };
        trimmed_ip
    };
    let url = if formatted_ip.starts_with("http://") || formatted_ip.starts_with("https://") {
        format!("{}/cdn-cgi/trace", formatted_ip.clone())
    } else {
        format!("http://{}/cdn-cgi/trace", formatted_ip)
    };
    
    let output = Command::new("curl")
        .arg("-o")
        .arg("/dev/null")
        .arg("-s")
        .arg("-w")
        .arg("%{http_code}")
        .arg("-m")
        .arg("5") // 设置超时(单位：秒)
        .arg(&url)
        .output()?;
        
    let response_code = String::from_utf8_lossy(&output.stdout);
    
    // 从URL中，获取域名、IP地址，用于返回值
    let url_parse = Url::parse(&url).unwrap(); // 解析URL
    let host = url_parse.host_str().unwrap_or_default().to_string(); // 提取域名或IP
    
    Ok((host, response_code.trim() == "200"))
}

// 检查文件读取的地址是IPv4地址、IPv6地址、域名，如果是CIDR，就生成IP地址
fn generate_ip_and_check_ip_type(ip_address: &str) -> Vec<String> {
    
    // 是CIDR的，处理方案
    if let Ok(ip_network) = ip_address.parse::<IpNetwork>() {
        if ip_network.is_ipv6() {
            if ip_network.prefix() < 119 {
                let mut rng = rand::thread_rng();
                let mut addresses = Vec::new();
                let num_addresses = 500; // 生成最多500个IPv6地址

                let generated_addresses: Vec<Ipv6Addr> = generate_random_ipv6_in_cidr(&ip_network, &mut rng, num_addresses);

                for ip in generated_addresses {
                    addresses.push(ip.to_string());
                }
                return addresses;
            } else {
                // 前缀长度大于等于119，生成CIDR范围内的所有IPv6地址
                let addresses: Vec<String> = ip_network.iter().map(|ip| ip.to_string()).collect();
                return addresses;
            }
        } else {
            return ip_network.iter().map(|ip| ip.to_string()).collect(); // 生成IPv4地址
        }
    }
    // 是IPv4地址、IPv6地址的
    if let Ok(ip) = ip_address.parse::<IpAddr>() {
        return vec![ip.to_string()];
    }

    // 不满足上面的条件，就原字符串返回，默认是域名地址
    vec![ip_address.to_string()]
}

// 生成IPv6 CIDR范围内的随机500个IPv6地址
fn generate_random_ipv6_in_cidr(ip_network: &IpNetwork, rng: &mut impl Rng, num_addresses: usize) -> Vec<Ipv6Addr> {
    if let IpNetwork::V6(cidr) = ip_network {
        let lower = u128::from(cidr.network());
        let upper = u128::from(cidr.broadcast());

        if lower <= upper {
            let mut generated_addresses = Vec::with_capacity(num_addresses);
            let mut num_generated = 0;

            while num_generated < num_addresses {
                let random_ipv6_int: u128 = rng.gen_range(lower..=upper);
                let random_ipv6_addr = Ipv6Addr::from(random_ipv6_int);

                // 检查地址是否在CIDR范围内并且不重复
                if !generated_addresses.contains(&random_ipv6_addr) {
                    generated_addresses.push(random_ipv6_addr);
                    num_generated += 1;
                }
            }

            return generated_addresses;
        } else {
            panic!("Invalid CIDR range");
        }
    } else {
        unreachable!();
    }
}


// 按行读取文件的内容
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

// 用于排序（IPv4、IPv6地址、域名）
fn custom_ip_sort(ip1: &String, ip2: &String) -> std::cmp::Ordering {
    let parse_result1 = IpAddr::from_str(ip1);
    let parse_result2 = IpAddr::from_str(ip2);

    match (parse_result1, parse_result2) {
        (Ok(ip1), Ok(ip2)) => ip1.cmp(&ip2),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => std::cmp::Ordering::Equal,
    }
}

// 将结果写入文件
fn write_to_file(data: &[String], file_name: &str) -> Result<(), io::Error> {
    let mut file = File::create(file_name)?;
    for ip in data {
        writeln!(file, "{}", ip)?;
    }
    Ok(())
}

// 计算程序运行的总时长
fn format_duration(duration: Duration) -> (f64, &'static str) {
    if duration.as_secs() > 0 {
        (duration.as_secs_f64(), "秒")
    } else if duration.as_millis() > 0 {
        (duration.as_millis() as f64, "毫秒")
    } else if duration.as_micros() > 0 {
        (duration.as_micros() as f64, "微秒")
    } else {
        (duration.as_nanos() as f64, "纳秒")
    }
}

// 辅助函数
fn wait_for_enter() {
    print!("按Enter键，退出程序！");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    init_logger()?;
    let ip_addresses = read_ips_file("ips-v4.txt")?;
    
    if !is_curl_installed(){
        println!("电脑中，没有安装有curl命令！按Enter键退出程序！");
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }

    // 线程池：生成CIDR范围的IP地址
    let pool_generate = threadpool::ThreadPool::new(20);
    let (tx_generate, rx_generate) = mpsc::channel();

    for item in ip_addresses.iter() {
        let tx_generate = tx_generate.clone();
        let cloned_item = item.clone();
        pool_generate.execute(move || {
            let ips = generate_ip_and_check_ip_type(&cloned_item);
            tx_generate.send(ips).unwrap();
        });
    }

    drop(tx_generate);  // 释放发送端。不影响要接受的(rx_generate)信息；同时便于后续迭代
    
    println!("开始扫描CF CDN中...\n");

    let mut ips: Vec<String> = Vec::new();
    // 从接受端迭代结果放到ips中
    for ips_batch in rx_generate.iter() {
        ips.extend(ips_batch);
    }

    let mut rng = rand::thread_rng();
    ips.shuffle(&mut rng); // 打乱排列顺序
    
    
    // 线程池：调用is_ip_reachable函数，获取测试后的结果
    let pool_method = threadpool::ThreadPool::new(200);
    /* mpsc通道，用于在线程之间安全地传递数据，其中一个线程（或多个线程）充当生产者，而另一个线程（单个线程）充当消费者。
       mpsc（多个生产者、单个消费者）通道，创建了两个端点，一个发送端 (tx_method) 和一个接收端 (rx_method)。
       这种通道类型在并发编程中非常有用，因为它提供了一种线程间通信的方式，以避免竞态条件和数据竞争。
    */
    let (tx_method, rx_method) = mpsc::channel(); 

    for item in ips.iter() {
        let tx_method = tx_method.clone();
        let cloned_item = item.clone();
        pool_method.execute(move || {
            if let Ok((ip, state)) = is_ip_reachable(&cloned_item) {
                log::info!("SCAN {}/cdn-cgi/trace --> RESULT: {}", ip, state);
                if state {
                    tx_method.send(ip).unwrap();
                }
            }
        });
    }
    
    drop(tx_method); // 释放发送端。不影响要接受的(rx_method)信息；同时便于后续迭代

    let mut reachable_ips: Vec<String> = Vec::new();
    // 从接受端迭代结果放到reachable_ips中
    for ip in rx_method.iter() {
        let cleaned_ip = ip.trim_matches(|c| c == '[' || c == ']'); // 去掉IPv6地址的方括号
        reachable_ips.push(cleaned_ip.to_string());
    }
    // 排序
    reachable_ips.sort_by(|ip1, ip2| custom_ip_sort(ip1, ip2));
    // 写入文件中
    write_to_file(&reachable_ips, "output.txt")?;

    // 记录结束的时间
    let end_time = Instant::now();
    // 计算程序运行的总时长
    let elapsed_duration = end_time.duration_since(start_time);
    // 转换为人类易读的时间
    let (elapsed_time, unit) = format_duration(elapsed_duration);

    print!("\n程序运行结束，耗时：{:.2} {}，", elapsed_time, unit);
    io::stdout().flush().expect("Failed to flush stdout");
    wait_for_enter();
    
    Ok(())
}