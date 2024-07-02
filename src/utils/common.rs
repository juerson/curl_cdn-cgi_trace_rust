use std::{
    io::{self, Write},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};

// 用于排序（IPv4、IPv6地址、域名）
pub fn sort_ips_and_hosts(ip1: &String, ip2: &String) -> std::cmp::Ordering {
    let parse_result1 = IpAddr::from_str(ip1);
    let parse_result2 = IpAddr::from_str(ip2);

    match (parse_result1, parse_result2) {
        (Ok(ip1), Ok(ip2)) => ip1.cmp(&ip2),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        (Err(_), Err(_)) => std::cmp::Ordering::Equal,
    }
}

// 计算程序运行的总时长
pub fn format_duration(duration: Duration) -> (f64, &'static str) {
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

pub fn wait_for_enter() {
    print!("按Enter键，退出程序！");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
}
