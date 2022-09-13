extern crate pnet_datalink;

use pnet_datalink::interfaces;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

fn main() {
    let device = env::var("NODE_IP_NETWORK_DEVICE").unwrap();

    let ifaces = interfaces();
    let iface = ifaces.iter().find(|e| e.name == device).unwrap();

    if let Some(ip) = iface.ips.iter().find(|e| e.is_ipv6() && e.prefix() == 128) {
        println!("Current machine ip {:?}", ip.ip());
        let get_traefik_ip = Command::new("kubectl")
            .args([
                "get",
                "svc",
                "traefik",
                "-n",
                "kube-system",
                "-o=go-template={{range .status.loadBalancer.ingress}}{{printf \"%s\\n\" .ip}}{{end}}"
            ]).output().unwrap();

        let traefik_ips = String::from_utf8(get_traefik_ip.stdout).unwrap();

        if !traefik_ips
            .lines()
            .any(|item| item == format!("{:?}", ip.ip()))
        {
            println!("{:?}", ip.ip());
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
                    &format!("192.168.0.2,{}", ip.ip()),
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

            // todo: change AAAA entries
        } else {
            println!("{:?}", "nothing to do");
        }
    }
}
