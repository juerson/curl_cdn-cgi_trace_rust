use rand::Rng;
use std::{ collections::HashSet, net::Ipv4Addr };
use ipnetwork::{ IpNetwork, Ipv4Network };

fn main() {
    let cidr = "192.168.1.0/24";
    let num_ips = 50;

    match cidr.parse::<IpNetwork>() {
        Ok(IpNetwork::V4(ipv4_network)) => {
            let ip_strings = generate_unique_ip_strings(ipv4_network, num_ips);
            for ip_str in &ip_strings {
                println!("{}", ip_str);
            }
            println!("Total: {}", ip_strings.len());
        }
        _ => eprintln!("Invalid CIDR or not an IPv4 CIDR"),
    }
}

fn generate_unique_ip_strings(network: Ipv4Network, count: usize) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut unique_ips = HashSet::new();
    let ip_range = network.size();

    // 生成 IP 地址并将其转换为字符串
    while unique_ips.len() < count && unique_ips.len() < (ip_range as usize) {
        let host = rng.gen_range(0..ip_range);
        let ip = Ipv4Addr::from(u32::from(network.network()) + host);
        if network.contains(ip) {
            unique_ips.insert(ip.to_string()); // 直接将 IP 地址转换为字符串形式
        }
    }

    unique_ips.into_iter().collect()
}
