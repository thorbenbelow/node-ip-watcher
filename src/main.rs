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
    KubectlError,
    TodoError
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
            ])
            .output()
            .map_err(|_|Error::KubectlError)?;
            
            Ok(get_traefik.stdout
                .split(|c| *c == b'\n')
                .filter_map(|line|
                    String::from_utf8(line.to_vec()).ok()
                )
                .collect::<Vec<_>>()
            )
}

fn kill_current_workloads() -> Result<std::process::Child, Error> {
    Command::new("/usr/local/bin/k3s-killall.sh").spawn().map_err(|_|Error::TodoError)
}

fn get_k3s_script() -> Result<std::process::Output, Error> {
    Command::new("curl")
            .args(["-sfL", "https://get.k3s.io"])
            .output().map_err(|_|Error::TodoError)
}

fn restart_k3s(ip: String) -> Result<(), Error> {
    kill_current_workloads()?;
        
    let k3s_script = get_k3s_script()?;

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

    let mut k3s = k3s.stdin.ok_or(Error::TodoError)?;
    k3s.write_all(&k3s_script.stdout).map_err(|_|Error::TodoError)?;

    Command::new("systemctl")
        .args(["restart", "k3s"])
        .spawn().map_err(|_|Error::TodoError)?;

    Ok(())

}

fn main() {
    let machine_ip = get_machine_ip().expect("No machine ip");
    println!("Current machine ip {:?}", machine_ip);
    let traefik_ips = get_traefik_ips().expect("Failed to extract traefik ips");

    if !traefik_ips.iter().any(|item| *item == format!("{:?}", machine_ip)) {
        restart_k3s(machine_ip.to_string()).expect("Failed to restart k3s");
    } else {
        println!("{:?}", "Nothing to do");
    }
}
