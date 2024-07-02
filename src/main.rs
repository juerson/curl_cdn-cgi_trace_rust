mod utils;

use std::{
    io::{self, Write},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};
use threadpool::ThreadPool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 检测curl命令
    if !utils::curl::is_curl_installed() {
        println!("电脑中，没有安装有curl命令！按Enter键退出程序！");
        io::stdout().flush().expect("Failed to flush stdout");
        let _ = io::stdin().read_line(&mut String::new());
        std::process::exit(1);
    }
    let start_time = Instant::now();

    // 初始化日记
    utils::logger::init_logger()?;

    match utils::files::read_text_file() {
        Ok(ips) => {
            // 使用线程池处理IP地址(是CIDR的就生成IP地址，不是就直接添加到向量中)
            let pool_size = 20;
            let ips = utils::network::process_ip_cidr_hosts(ips, pool_size);

            println!("开始扫描 cdn-cgi/trace 中...\n");

            // 通过process_ips_and_filter_cloudflare函数间接调用check_server_is_cloudflare函数，获取测试后的结果
            let pool_size = 200;
            let reachable_ips = process_ips_and_filter_cloudflare(ips, pool_size);

            // 写入文件中
            utils::files::write_to_file(&reachable_ips, "output.txt")?;

            // 记录结束的时间
            let end_time = Instant::now();
            // 计算程序运行的总时长
            let elapsed_duration: Duration = end_time.duration_since(start_time);
            // 转换为人类易读的时间
            let (elapsed_time, unit) = utils::common::format_duration(elapsed_duration);

            print!("\n任务运行完毕，耗时：{:.2}{}，", elapsed_time, unit);
            io::stdout().flush().expect("Failed to flush stdout");
            utils::common::wait_for_enter();
        }
        Err(e) => eprintln!("读取txt文件时发生错误: {}", e),
    }

    Ok(())
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
            if let Ok((ip, state)) = utils::curl::check_server_is_cloudflare(&cloned_item) {
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
    reachable_ips.sort_by(|ip1, ip2| utils::common::sort_ips_and_hosts(ip1, ip2));

    reachable_ips
}
