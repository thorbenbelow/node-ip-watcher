extern crate pnet_datalink;

use clap::Parser;

use pnet_datalink::interfaces;
use std::net::IpAddr;
use std::process::{Command, Stdio};

#[derive(Debug)]
enum Error {
    InterfaceNotFoundError,
    IpNotFoundError,
    KubectlError,
    TodoError,
}

fn get_machine_ip(device: &str) -> Result<IpAddr, Error> {

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
            "-o=go-template={{range .status.loadBalancer.ingress}}{{printf \"%s\\n\" .ip}}{{end}}",
        ])
        .output()
        .map_err(|_| Error::KubectlError)?;

    Ok(get_traefik
        .stdout
        .split(|c| *c == b'\n')
        .filter_map(|line| String::from_utf8(line.to_vec()).ok())
        .filter(|ip| !ip.is_empty())
        .collect::<Vec<_>>())
}

fn kill_current_workloads() -> Result<std::process::Output, Error> {
    Command::new("/usr/local/bin/k3s-killall.sh")
        .output()
        .map_err(|_| Error::TodoError)
}

fn get_k3s_script() -> Result<std::process::Output, Error> {
    Command::new("curl")
        .args(["-sfL", "https://get.k3s.io"])
        .output()
        .map_err(|_| Error::TodoError)
}

fn restart_k3s(ip: String) -> Result<(), Error> {
    kill_current_workloads()?;

    let k3s_script = get_k3s_script()?;

    let k3s_start_output = Command::new("sh")
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
        .output().map_err(|_|{Error::TodoError})?;
    println!("Restarting k3s...");
    println!("{}", String::from_utf8(k3s_start_output.stdout).unwrap());
    println!("{}", String::from_utf8(k3s_start_output.stderr).unwrap());


    Command::new("systemctl")
        .args(["restart", "k3s"])
        .spawn()
        .map_err(|_| Error::TodoError)?;

    Ok(())
}

/// Simple program to sync k3s and node ip
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of network device to sync k3s with
   #[arg(short, long)]
   device: String,
}

fn main() {

    let args = Args::parse();

    let machine_ip = get_machine_ip(&args.device).expect("No machine ip");
    println!("Current machine ip: {:?}", machine_ip);
    let traefik_ips = get_traefik_ips().expect("Failed to extract traefik ips");
    println!("Current traefik ips: {:?}", traefik_ips);
    if !traefik_ips.iter().any(|item| *item == format!("{:?}", machine_ip)) {
        println!("Restart needed");
        restart_k3s(machine_ip.to_string()).expect("Failed to restart k3s");
    } else {
        println!("{:?}", "Nothing to do");
    }
}
