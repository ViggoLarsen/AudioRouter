use anyhow::{Context, Result};
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

const SERVICE_NAME: &str = "AudioRouter";
const DISPLAY_NAME: &str = "Audio Router Service";
const DESCRIPTION: &str = "Routes audio between different audio devices";

pub fn install_service() -> Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("Failed to connect to service manager")?;

    let exe_path = env::current_exe().context("Failed to get executable path")?;

    let service_binary_path = PathBuf::from(format!("{}", exe_path.display()));

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![OsString::from("service")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };

    let service = match manager.create_service(
        &service_info,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
    ) {
        Ok(s) => {
            println!("Service '{}' installed successfully", SERVICE_NAME);
            s
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("already exists") || error_str.contains("1073") {
                println!("Service '{}' already exists", SERVICE_NAME);
                manager
                    .open_service(
                        SERVICE_NAME,
                        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START,
                    )
                    .context("Failed to open existing service")?
            } else {
                return Err(anyhow::anyhow!("Failed to create service: {}", e));
            }
        }
    };

    service
        .set_description(DESCRIPTION)
        .context("Failed to set service description")?;

    println!("Service description set to: {}", DESCRIPTION);

    match service.start::<&str>(&[]) {
        Ok(_) => {
            println!("Service started successfully");
            println!("Audio Router is now running and will auto-start on system boot");
        }
        Err(e) => {
            println!("Service installed but failed to start automatically: {}", e);
            println!("\nTo start the service manually, run:");
            println!("  sc start {}", SERVICE_NAME);
            println!("\nOr use Services management console (services.msc)");
        }
    }

    Ok(())
}

pub fn uninstall_service() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("Failed to connect to service manager")?;

    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
        )
        .context("Failed to open service. Is it installed?")?;

    let status = service
        .query_status()
        .context("Failed to query service status")?;

    if status.current_state != windows_service::service::ServiceState::Stopped {
        println!("Service is running. Please stop it first:");
        println!("  sc stop {}", SERVICE_NAME);
        return Err(anyhow::anyhow!(
            "Service must be stopped before uninstalling"
        ));
    }

    service.delete().context("Failed to delete service")?;

    println!("Service '{}' uninstalled successfully", SERVICE_NAME);

    Ok(())
}
