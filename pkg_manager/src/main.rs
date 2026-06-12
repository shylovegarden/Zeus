use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;
use dirs::home_dir;

#[derive(Parser)]
#[command(name = "zeus")]
#[command(about = "Zeus Package Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and install a package
    Get {
        /// Package name (e.g., username/package)
        package: String,
        /// Version to install (defaults to latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Publish a package to the registry
    Publish {
        /// Path to package directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Search for packages
    Search {
        /// Search query
        query: String,
    },
    /// List installed packages
    List,
    /// Remove an installed package
    Remove {
        /// Package name
        package: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PackageManifest {
    package: PackageInfo,
    dependencies: Option<std::collections::HashMap<String, String>>,
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<std::collections::HashMap<String, String>>,
    lib: Option<LibInfo>,
    bin: Option<Vec<BinInfo>>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PackageInfo {
    name: String,
    version: String,
    description: Option<String>,
    authors: Option<Vec<String>>,
    #[serde(rename = "zeus_version")]
    zeus_version: Option<String>,
    license: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LibInfo {
    name: Option<String>,
    path: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BinInfo {
    name: String,
    path: String,
}

fn get_zeus_home() -> PathBuf {
    home_dir()
        .map(|h| h.join(".zeus"))
        .unwrap_or_else(|| PathBuf::from(".zeus"))
}

fn get_packages_dir() -> PathBuf {
    get_zeus_home().join("packages")
}

fn get_registry_url() -> String {
    std::env::var("ZEUS_REGISTRY_URL")
        .unwrap_or_else(|_| "https://zeus.pkg.dev".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get { package, version } => {
            cmd_get(package, version).await?;
        }
        Commands::Publish { path } => {
            cmd_publish(path).await?;
        }
        Commands::Search { query } => {
            cmd_search(query).await?;
        }
        Commands::List => {
            cmd_list()?;
        }
        Commands::Remove { package } => {
            cmd_remove(package)?;
        }
    }

    Ok(())
}

async fn cmd_get(package: String, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Installing package: {}", package);
    
    let version = version.unwrap_or_else(|| "latest".to_string());
    let registry_url = get_registry_url();
    
    // TODO: Implement actual package download from registry
    println!("Registry: {}", registry_url);
    println!("Version: {}", version);
    
    // Create packages directory if it doesn't exist
    let packages_dir = get_packages_dir();
    fs::create_dir_all(&packages_dir)?;
    
    println!("Package would be installed to: {}", packages_dir.display());
    println!("Note: Package download not yet implemented");
    
    Ok(())
}

async fn cmd_publish(path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let pkg_path = path.unwrap_or_else(|| PathBuf::from("."));
    let manifest_path = pkg_path.join("zeus_pkg.toml");
    
    if !manifest_path.exists() {
        return Err("zeus_pkg.toml not found in package directory".into());
    }
    
    println!("Publishing package from: {}", pkg_path.display());
    
    // Read and validate manifest
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let _manifest: PackageManifest = toml::from_str(&manifest_content)
        .map_err(|e| format!("Invalid manifest: {}", e))?;
    
    println!("Manifest validated successfully");
    println!("Note: Package upload not yet implemented");
    
    Ok(())
}

async fn cmd_search(query: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Searching for: {}", query);
    println!("Note: Package search not yet implemented");
    Ok(())
}

fn cmd_list() -> Result<(), Box<dyn std::error::Error>> {
    let packages_dir = get_packages_dir();
    
    if !packages_dir.exists() {
        println!("No packages installed");
        return Ok(());
    }
    
    println!("Installed packages:");
    for entry in fs::read_dir(&packages_dir)? {
        let entry = entry?;
        println!("  - {}", entry.file_name().to_string_lossy());
    }
    
    Ok(())
}

fn cmd_remove(package: String) -> Result<(), Box<dyn std::error::Error>> {
    let packages_dir = get_packages_dir();
    let pkg_dir = packages_dir.join(&package);
    
    if !pkg_dir.exists() {
        return Err(format!("Package '{}' not installed", package).into());
    }
    
    fs::remove_dir_all(&pkg_dir)?;
    println!("Removed package: {}", package);
    
    Ok(())
}
