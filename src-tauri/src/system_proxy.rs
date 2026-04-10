use anyhow::{Context, Result};

pub async fn set_system_proxy(enable: bool, listen_addr: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    return set_system_proxy_macos(enable, listen_addr).await;

    #[cfg(target_os = "windows")]
    return set_system_proxy_windows(enable, listen_addr);

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (enable, listen_addr);
        anyhow::bail!("system proxy not supported on this platform")
    }
}

#[cfg(target_os = "macos")]
async fn set_system_proxy_macos(enable: bool, listen_addr: &str) -> Result<()> {
    let services = get_macos_network_services().await?;

    let (host, port) = if enable {
        parse_host_port(listen_addr)?
    } else {
        (String::new(), String::new())
    };

    for service in &services {
        if enable {
            run_networksetup(&["-setsocksfirewallproxy", service, &host, &port]).await?;
            run_networksetup(&["-setsocksfirewallproxystate", service, "on"]).await?;
        } else {
            run_networksetup(&["-setsocksfirewallproxystate", service, "off"]).await?;
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
async fn get_macos_network_services() -> Result<Vec<String>> {
    let output = tokio::process::Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .await
        .context("failed to run networksetup")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let services: Vec<String> = stdout
        .lines()
        .skip(1) // first line is a header
        .filter(|l| !l.starts_with('*') && !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    Ok(services)
}

#[cfg(target_os = "macos")]
async fn run_networksetup(args: &[&str]) -> Result<()> {
    let status = tokio::process::Command::new("networksetup")
        .args(args)
        .status()
        .await
        .context("failed to run networksetup")?;

    if !status.success() {
        anyhow::bail!("networksetup {:?} exited with {}", args, status);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_system_proxy_windows(enable: bool, listen_addr: &str) -> Result<()> {
    use std::process::Command;

    if enable {
        let proxy_value = format!("socks={listen_addr}");
        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                "/v", "ProxyServer",
                "/t", "REG_SZ",
                "/d", &proxy_value,
                "/f",
            ])
            .status()
            .context("reg add ProxyServer")?;

        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                "/v", "ProxyEnable",
                "/t", "REG_DWORD",
                "/d", "1",
                "/f",
            ])
            .status()
            .context("reg add ProxyEnable")?;
    } else {
        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                "/v", "ProxyEnable",
                "/t", "REG_DWORD",
                "/d", "0",
                "/f",
            ])
            .status()
            .context("reg add ProxyEnable=0")?;
    }

    Ok(())
}

fn parse_host_port(addr: &str) -> Result<(String, String)> {
    let addr: std::net::SocketAddr = addr.parse().context("invalid listen address")?;
    Ok((addr.ip().to_string(), addr.port().to_string()))
}
