mod utils;

use crate::utils::models::Airport;
use std::{ fs::{ self }, sync::{ mpsc, Arc, Mutex }, time::Instant };
use reqwest::Error;
use threadpool::ThreadPool;
use clap::Parser;
// use clap::CommandFactory;

/// 批量扫描是否走CloudFlare CDN的流量。
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// 输入的数据文件(*.txt)，支持域名地址、IPv4/IPv6地址、IPv4/IPv6的CIDR的数据
    #[arg(short = 'f', default_value_t = format!("ips-v4.txt"))]
    file: String,

    /// 数据输出的文件，结果输出到这个文件中
    #[arg(short = 'o', default_value_t = format!("output.csv"))]
    output: String,

    /// 如果是IPv4/IPv6的CIDR，就它的范围，随机生成指定数量的IP地址
    #[arg(short, default_value_t = 1)]
    num: usize,

    /// 同时并行执行的任务数量，拿多个地址并行执行curl命令
    #[arg(long, default_value_t = 50)]
    pool: u16,

    /// 只扫描是否为jetbrains的许可证服务器
    #[arg(long, default_value_t = false)]
    jetbrains: bool,
}

static LOCATIONS: &str = "locations.json";
static LOCATIONS_URL: &str = "https://speed.cloudflare.com/locations";

/// 用于下载locations.json文件
async fn download_file(url: &str, path: &str) -> Result<(), Error> {
    let response = reqwest::get(url).await?;
    let content = response.text().await?;
    fs::write(path, content).expect("Unable to write file");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 检测curl是否安装，没有安装就退出程序
    utils::curl::check_curl_installed();
    // 初始化日记
    utils::logger::init_logger()?;

    let args = Args::parse();
    /*
        检查是否未提供任何参数（程序名称除外）
        注释掉这个if条件，如果设置Args的默认参数值，双击编译后的exe程序会自动执行
    */
    // if std::env::args().len() <= 1 {
    //     // 显示帮助信息
    //     let mut cmd = Args::command();
    //     cmd.print_help().unwrap();
    //     std::process::exit(0);
    // }

    // 加载locations.json文件
    let locations = match fs::read_to_string(LOCATIONS) {
        Ok(data) => data,
        Err(_) => {
            // 文件不存在或读取失败，下载文件
            download_file(LOCATIONS_URL, LOCATIONS).await?;
            fs::read_to_string(LOCATIONS).expect("Unable to read file")
        }
    };

    // 解析为 Airport 结构体
    let airports: Vec<Airport> = serde_json::from_str(&locations)?;
    let start_time = Instant::now();
    match utils::files::read_text_file(&args.file) {
        Ok(line) => {
            let data_vec = utils::network::process_ip_cidr_hosts(line, 20, args.num);

            println!("开始扫描 cdn-cgi/trace 中...\n");
            let (tx_method, rx_method) = mpsc::channel();
            let pool_method = ThreadPool::new(args.pool.into());
            let arc_addr = Arc::new(Mutex::new(data_vec));
            for addr in arc_addr.lock().unwrap().iter() {
                let tx_method = tx_method.clone();
                let cloned_addr = addr.clone();
                let airports = airports.clone();
                pool_method.execute(move || {
                    match
                        utils::curl::run_command_and_process_data(
                            &cloned_addr,
                            airports,
                            args.jetbrains
                        )
                    {
                        Ok(record) => tx_method.send(record).unwrap(),
                        Err(_e) => {}
                    }
                });
            }
            drop(tx_method);

            let mut records: Vec<Vec<String>> = Vec::new();
            // 读取通道数据，添加到records向量中
            rx_method.iter().for_each(|item| {
                let delay = item.delay.as_millis();
                let vec = vec![
                    item.ip.clone(),
                    item.colo.clone(),
                    item.country.clone(),
                    item.region.clone(),
                    item.city.clone(),
                    delay.to_string(),
                    item.http_status_code.to_string()
                ];
                records.push(vec);
            });

            // ----------------------------------------------------------------------------

            // 按延迟(毫秒)排序，注意：延迟没有单位ms和s的字符串
            records.sort_by(|a, b| {
                let latency_a: i32 = a[5].parse().unwrap_or(i32::MAX);
                let latency_b: i32 = b[5].parse().unwrap_or(i32::MAX);
                latency_a.cmp(&latency_b) // 比较
            });

            // 将标题行添加到开头
            records.insert(
                0,
                vec![
                    "IP地址".to_string(),
                    "数据中心".to_string(),
                    "alpha-2".to_string(),
                    "地区".to_string(),
                    "城市".to_string(),
                    "延迟(毫秒)".to_string(), // 该值仅供参考，只是执行curl命令的耗时
                    "HTTP状态码".to_string()
                ]
            );

            // ----------------------------------------------------------------------------
            match args.jetbrains {
                true => {
                    // 过滤掉不需要的列
                    let mut jetbrains_records: Vec<Vec<String>> = Vec::new();

                    // 添加标题，包含 "jetBrains激活服务器" 列
                    let header: Vec<String> = vec![
                        "IP地址".to_string(),
                        "延迟(毫秒)".to_string(),
                        "HTTP状态码".to_string(),
                        "jetBrains激活服务器".to_string()
                    ];
                    jetbrains_records.push(header);

                    // 遍历原始记录并保留需要的列，以及插入新列
                    for row in records.iter().skip(1) {
                        let mut new_row: Vec<String> = Vec::new();
                        new_row.push(row[0].clone()); // 添加第一列
                        new_row.push(row[row.len() - 2].clone()); // 添加倒数第二列
                        new_row.push(row[row.len() - 1].clone()); // 添加倒数第一列
                        new_row.push("true".to_string()); // JetBrains License server
                        jetbrains_records.push(new_row);
                    }

                    // 写入CSV文件
                    utils::files::write_to_csv(&args.output, jetbrains_records)?;
                }
                false => {
                    // 写入CSV文件
                    utils::files::write_to_csv(&args.output, records)?;
                }
            }
        }
        Err(e) => eprintln!("读取txt文件时发生错误: {}", e),
    }
    println!("\n程序扫描的总时长: {:?}", start_time.elapsed());

    Ok(())
}
