use ipnetwork::IpNetwork;
use rand::{ prelude::SliceRandom, Rng };
use std::{ collections::HashSet, net::{ IpAddr, Ipv4Addr, Ipv6Addr }, sync::{ mpsc, Arc, Mutex } };
use threadpool::ThreadPool;

// 处理IPv4、IPv6、CIDR、域名，是CIDR的话，就随机生成IP，否则就返回原字符串
pub fn process_ip_cidr_hosts(
    ip_addresses: Vec<String>,
    pool_size: usize,
    count: usize
) -> Vec<String> {
    let pool_generate = ThreadPool::new(pool_size);
    let (tx_generate, rx_generate) = mpsc::channel();
    let ip_addresses = Arc::new(Mutex::new(ip_addresses));
    for item in ip_addresses.lock().unwrap().iter() {
        let tx_generate = tx_generate.clone();
        let cloned_item = item.clone();
        pool_generate.execute(move || {
            if count > 1 {
                let ips = generate_ip_and_check_ip_type2(&cloned_item, count);
                tx_generate.send(ips).unwrap();
            } else {
                let ips = generate_ip_and_check_ip_type(&cloned_item);
                tx_generate.send(ips).unwrap();
            }
        });
    }
    drop(tx_generate);

    // 从接受端迭代结果放到ips中
    let mut ips: Vec<String> = Vec::new();
    rx_generate.iter().for_each(|ips_batch| ips.extend(ips_batch));
    // 打乱顺序
    ips.shuffle(&mut rand::thread_rng());

    ips
}

// ---------------------------------分支1----------------------------------------------------

fn generate_ip_and_check_ip_type(ip_address: &str) -> Vec<String> {
    // 是CIDR的，处理方案，支持ipv4和ipv6的cidr，只生成单个IP
    if let Ok(ip_network) = ip_address.parse::<IpNetwork>() {
        let mut rng = rand::thread_rng();
        match ip_network {
            IpNetwork::V4(v4_network) => {
                let network = u32::from(v4_network.network());
                let mask_len = v4_network.prefix();
                let host_part_len = 32 - mask_len;
                let max_host_value = (1 << host_part_len) - 1;

                let random_host: u32 = rng.gen_range(0..=max_host_value);
                return vec![IpAddr::V4(Ipv4Addr::from(network | random_host)).to_string()];
            }
            IpNetwork::V6(v6_network) => {
                let network = u128::from(v6_network.network());
                let mask_len = v6_network.prefix();
                let host_part_len = 128 - mask_len;
                let max_host_value = (1 << host_part_len) - 1;

                let random_host: u128 = rng.gen_range(0..=max_host_value);
                return vec![IpAddr::V6(Ipv6Addr::from(network | random_host)).to_string()];
            }
        }
    }
    // 是IPv4地址、IPv6地址的
    if let Ok(ip) = ip_address.parse::<IpAddr>() {
        return vec![ip.to_string()];
    }
    // 不满足上面的条件，就原字符串返回，默认是域名地址
    vec![ip_address.to_string()]
}

// ---------------------------------分支2----------------------------------------------------

fn generate_ip_and_check_ip_type2(ip_address: &str, count: usize) -> Vec<String> {
    // 尝试解析为 CIDR 或 IP 地址
    if let Ok(ip_network) = ip_address.parse::<IpNetwork>() {
        return match ip_network {
            // 处理 IPv6 CIDR
            _ if ip_network.is_ipv6() => generate_random_ipv6_in_cidr(ip_network, count),
            // 处理 IPv4 CIDR
            _ if ip_network.is_ipv4() => generate_random_ipv4_in_cidr(ip_network, count),
            // 正常情况下，不应该到达这个分支，只处理网络段
            _ => unreachable!(),
        };
    }
    // 尝试解析为单个 IP 地址
    if let Ok(ip) = ip_address.parse::<IpAddr>() {
        return vec![ip.to_string()];
    }
    // 返回原字符串，假设其为域名
    vec![ip_address.to_string()]
}

fn generate_random_ipv6_in_cidr(ip_network: IpNetwork, count: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    if ip_network.prefix() < 119 {
        if let IpNetwork::V6(cidr) = ip_network {
            let lower = u128::from(cidr.network());
            let upper = u128::from(cidr.broadcast());
            let mut generated_addresses: Vec<String> = Vec::with_capacity(count);

            while generated_addresses.len() < count {
                let random_ipv6_int = rng.gen_range(lower..=upper);
                let random_ipv6_addr = Ipv6Addr::from(random_ipv6_int);
                // 检查地址是否在 CIDR 范围内并且不重复
                if !generated_addresses.contains(&random_ipv6_addr.to_string()) {
                    generated_addresses.push(random_ipv6_addr.to_string());
                }
            }

            generated_addresses
        } else {
            unreachable!(); // 正常情况下，不应该到达这个分支，只处理IPv6网络
        }
    } else {
        // 生成所有 IPv6 地址
        ip_network
            .iter()
            .map(|ip| ip.to_string())
            .collect()
    }
}

fn generate_random_ipv4_in_cidr(ip_network: IpNetwork, count: usize) -> Vec<String> {
    if let IpNetwork::V4(v4_network) = ip_network {
        let mut rng = rand::thread_rng();
        let mut unique_ips = HashSet::new();
        let ip_range = v4_network.size();

        // 生成 IP 地址并将其转换为字符串
        while unique_ips.len() < count && unique_ips.len() < (ip_range as usize) {
            let host = rng.gen_range(0..ip_range);
            let ip = Ipv4Addr::from(u32::from(v4_network.network()) + host);
            if v4_network.contains(ip) {
                unique_ips.insert(ip.to_string()); // 直接将 IP 地址转换为字符串形式
            }
        }

        unique_ips.into_iter().collect()
    } else {
        unreachable!(); // 正常情况下，不应该到达这个分支，只处理IPv4网络
    }
}
