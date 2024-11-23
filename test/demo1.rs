use ipnetwork::IpNetwork;
use rand::Rng;
use std::net::{ IpAddr, Ipv4Addr, Ipv6Addr };

fn random_ip(ip_network: IpNetwork) -> IpAddr {
    let mut rng = rand::thread_rng();
    match ip_network {
        IpNetwork::V4(v4_network) => {
            let network = u32::from(v4_network.network());
            let mask_len = v4_network.prefix();
            let host_part_len = 32 - mask_len;
            let max_host_value = (1 << host_part_len) - 1;

            let random_host: u32 = rng.gen_range(0..=max_host_value);
            IpAddr::V4(Ipv4Addr::from(network | random_host))
        }
        IpNetwork::V6(v6_network) => {
            let network = u128::from(v6_network.network());
            let mask_len = v6_network.prefix();
            let host_part_len = 128 - mask_len;
            let max_host_value = (1 << host_part_len) - 1;

            let random_host: u128 = rng.gen_range(0..=max_host_value);
            IpAddr::V6(Ipv6Addr::from(network | random_host))
        }
    }
}

fn main() {
    let ip_network: IpNetwork = "104.17.17.0/24".parse().expect("Invalid network");
    let random_ip_address = random_ip(ip_network);
    println!("随机生成的IP地址: {}", random_ip_address);
}
