use rand::Rng;
use ipnetwork::IpNetwork;
use std::{ collections::HashSet, net::{ IpAddr, Ipv4Addr, Ipv6Addr } };
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

fn main() {
    // let cidr = "2001:db8::/32";
    // let cidr = "192.168.1.0/24";
    let cidr = "github.com";
    let result = generate_ip_and_check_ip_type2(cidr, 1);
    println!("{:?}", result);
    println!("Total: {}", result.len());
}
