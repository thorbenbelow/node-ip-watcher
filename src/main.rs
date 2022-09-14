extern crate pnet_datalink;

use pnet_datalink::interfaces;
use std::env;
use std::io::Write;
use std::net::IpAddr;
use std::process::{Command, Stdio};

#[derive(Debug)]
enum Error {
    DeviceNotFoundError,
    InterfaceNotFoundError,
    IpNotFoundError,
    IpParseError
}

fn get_machine_ip() -> Result<IpAddr, Error>{
    let device = env::var("NODE_IP_NETWORK_DEVICE").map_err(|_| Error::DeviceNotFoundError)?;

    let ifaces = interfaces();
    let iface = ifaces
        .iter()
        .find(|e| e.name == device)
        .ok_or(Error::InterfaceNotFoundError)?;
    
    let network = iface
        .ips
        .iter()
        .find(|e| e.is_ipv6() && e.prefix() == 128)
        .ok_or(Error::IpNotFoundError)?;

    Ok(network.ip())
}

fn get_traefik_ips() -> Result<Vec<String>, Error> {

    let get_traefik = Command::new("kubectl")
            .args([
                "get",
                "svc",
                "traefik",
                "-n",
                "kube-system",
                "-o=go-template={{range .status.loadBalancer.ingress}}{{printf \"%s\\n\" .ip}}{{end}}"
            ]).output().unwrap();
            Ok(get_traefik.stdout
                .split(|c| *c == b'\n')
                .map(|line|
                    if let OK(ip) = String::from_utf8(line.to_vec()) {
                        ip
                    }else {
                        
                    }
                )
                .collect::<Vec<_>>()
            )
}

fn main() {
    if let Ok(ip) = get_machine_ip() {
        println!("Current machine ip {:?}", ip);
        
        let traefik_ips = get_traefik_ips().unwrap();
        if !traefik_ips.iter().any(|item| *item == format!("{:?}", ip)) {
            println!("{:?}", ip);
            Command::new("/usr/local/bin/k3s-killall.sh").spawn();
            let k3s_script = Command::new("curl")
                .args(["-sfL", "https://get.k3s.io"])
                .output()
                .unwrap();

            let k3s = match Command::new("sh")
                .args([
                    "-s",
                    "server",
                    "--node-ip",
                    &format!("192.168.0.2,{}", ip),
                    "--cluster-cidr",
                    "10.42.0.0/16,2001:cafe:42:0::/56",
                    "--service-cidr",
                    "10.43.0.0/16,2001:cafe:42:1::/112",
                ])
                .stdin(Stdio::piped())
                .spawn()
            {
                Err(e) => panic!("k3s spawn failed: {}", e),
                Ok(k3s) => k3s,
            };

            match k3s.stdin.unwrap().write_all(&k3s_script.stdout) {
                Err(e) => panic!("k3s init failed: {}", e),
                Ok(_) => {}
            };

            Command::new("systemctl")
                .args(["restart", "k3s"])
                .spawn()
                .unwrap();
        } else {
            println!("{:?}", "nothing to do");
        }
    }
}
