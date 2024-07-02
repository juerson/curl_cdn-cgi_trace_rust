use ipnetwork::IpNetwork;
use rand::{prelude::SliceRandom, Rng};
use std::{
    net::{IpAddr, Ipv6Addr},
    sync::{mpsc, Arc, Mutex},
};
use threadpool::ThreadPool;

// 处理IP地址，检测到是cidr，就生成IP地址，不是就直接添加ips中
pub fn process_ip_cidr_hosts(ip_addresses: Vec<String>, pool_size: usize) -> Vec<String> {
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
