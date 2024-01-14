use ipnetwork::IpNetwork;
use rand::{seq::SliceRandom, Rng};
use std::{
    fs::File,
    io::{self, BufRead, Write},
    net::{IpAddr, Ipv6Addr},
    process::{Command, Stdio},
    str::FromStr,
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use threadpool::ThreadPool;
use url::Url;

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
    let output = Command::new("curl").arg("--version").output().is_ok(); // 检查命令执行是否成功

    output
}

// 使用CURL命令（需要在电脑中安装curl才能使用，特别是windows系统中），判断headers头文件信息是否有cloudflare字符
fn check_server_is_cloudflare(ip: &str) -> Result<(String, bool), io::Error> {
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

// 检查文件读取的地址是IPv4地址、IPv6地址、域名，如果是CIDR，就生成IP地址
fn generate_ip_and_check_ip_type(ip_address: &str) -> Vec<String> {
    // 是CIDR的，处理方案
    if let Ok(ip_network) = ip_address.parse::<IpNetwork>() {
        if ip_network.is_ipv6() {
            if ip_network.prefix() < 119 {
                let mut rng = rand::thread_rng();
                let mut addresses = Vec::new();
                let num_addresses = 500; // 生成最多500个IPv6地址

                let generated_addresses: Vec<Ipv6Addr> =
                    generate_random_ipv6_in_cidr(&ip_network, &mut rng, num_addresses);

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
fn generate_random_ipv6_in_cidr(
    ip_network: &IpNetwork,
    rng: &mut impl Rng,
    num_addresses: usize,
) -> Vec<Ipv6Addr> {
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
            println!("打开{}文件失败，错误原因是:{}", file_path, e);
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
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
}

// 处理IP地址，检测到是cidr，就生成IP地址，不是就直接添加ips中
fn process_ip_addresses(ip_addresses: Vec<String>, pool_size: usize) -> Vec<String> {
    let pool_generate = ThreadPool::new(pool_size);
    let (tx_generate, rx_generate) = mpsc::channel();
    let ip_addresses = Arc::new(Mutex::new(ip_addresses));

    for item in ip_addresses.lock().unwrap().iter() {
        let tx_generate = tx_generate.clone();
        let cloned_item = item.clone();
        pool_generate.execute(move || {
            let ips = generate_ip_and_check_ip_type(&cloned_item);
            tx_generate.send(ips).unwrap();
        });
    }
    // 释放发送端。不影响要接受的(rx_generate)信息；同时便于后续迭代
    drop(tx_generate);

    let mut ips: Vec<String> = Vec::new();
    // 从接受端迭代结果放到ips中
    for ips_batch in rx_generate.iter() {
        ips.extend(ips_batch);
    }
    let mut rng = rand::thread_rng();
    ips.shuffle(&mut rng); // 打乱排列顺序

    ips
}

// 间接调用check_server_is_cloudflare函数，获取测试后的结果，并排序
fn process_ips_and_filter_cloudflare(ips: Vec<String>, pool_size: usize) -> Vec<String> {
    let pool_method = ThreadPool::new(pool_size);
    /* mpsc通道，用于在线程之间安全地传递数据，其中一个线程（或多个线程）充当生产者，而另一个线程（单个线程）充当消费者。
       mpsc（多个生产者、单个消费者）通道，创建了两个端点，一个发送端 (tx_method) 和一个接收端 (rx_method)。
       这种通道类型在并发编程中非常有用，因为它提供了一种线程间通信的方式，以避免竞态条件和数据竞争。
    */
    let (tx_method, rx_method) = mpsc::channel();
    let ips = Arc::new(Mutex::new(ips));
    for item in ips.lock().unwrap().iter() {
        let tx_method = tx_method.clone();
        let cloned_item = item.clone();
        pool_method.execute(move || {
            if let Ok((ip, state)) = check_server_is_cloudflare(&cloned_item) {
                log::info!("{}/cdn-cgi/trace -> Result: {}", ip, state);
                if state {
                    tx_method.send(ip).unwrap();
                }
            }
        });
    }
    // 释放发送端。不影响要接受的(rx_method)信息；同时便于后续迭代
    drop(tx_method);
    let mut reachable_ips: Vec<String> = Vec::new();
    // 从接受端迭代结果放到reachable_ips中
    for ip in rx_method.iter() {
        let cleaned_ip = ip.trim_matches(|c| c == '[' || c == ']'); // 去掉IPv6地址的方括号
        reachable_ips.push(cleaned_ip.to_string());
    }
    // 调用reachable_ips函数排序(先根据IP地址排序，域名在后面)
    reachable_ips.sort_by(|ip1, ip2| custom_ip_sort(ip1, ip2));

    reachable_ips
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    init_logger()?;
    let ip_addresses = read_ips_file("ips-v4.txt")?;

    // 检测curl命令
    if !is_curl_installed() {
        println!("电脑中，没有安装有curl命令！按Enter键退出程序！");
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }

    // 使用线程池处理IP地址(是CIDR的就生成IP地址，不是就直接添加到向量中)
    let pool_size = 20;
    let ips = process_ip_addresses(ip_addresses, pool_size);

    println!("开始扫描 cdn-cgi/trace 中...\n");

    // 通过process_ips_and_filter_cloudflare函数间接调用check_server_is_cloudflare函数，获取测试后的结果
    let pool_size = 200;
    let reachable_ips = process_ips_and_filter_cloudflare(ips, pool_size);

    // 写入文件中
    write_to_file(&reachable_ips, "output.txt")?;

    // 记录结束的时间
    let end_time = Instant::now();
    // 计算程序运行的总时长
    let elapsed_duration = end_time.duration_since(start_time);
    // 转换为人类易读的时间
    let (elapsed_time, unit) = format_duration(elapsed_duration);

    print!("\n程序运行结束，耗时：{:.2}{}，", elapsed_time, unit);
    io::stdout().flush().expect("Failed to flush stdout");
    wait_for_enter();

    Ok(())
}
