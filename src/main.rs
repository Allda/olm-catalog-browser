use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

/// YAML Document Types
#[derive(Debug, Deserialize)]
#[serde(tag = "schema")]
enum CatalogEntry {
    #[serde(rename = "olm.package")]
    OlmPackage(Package),
    #[serde(rename = "olm.channel")]
    OpmChannel(Channel),
    #[serde(rename = "olm.bundle")]
    OlmBundle(Bundle),
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ChannelEntry {
    name: String,
    #[serde(default)]
    replaces: String,
    #[serde(default)]
    skips: Vec<String>,
    #[serde(rename = "SkipRange")]
    skip_range: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    name: String,
    package: String,
    entries: Vec<ChannelEntry>,
}

// Implement a custom fucntion for Channel to print the entries
impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Channel: {}\n  Package: {}\n  Entries:",
            self.name, self.package
        )?;
        for entry in &self.entries {
            write!(f, "\n    - {}", entry.name)?;
            if entry.replaces != "" {
                write!(f, "\n      replaces: {}", entry.replaces)?;
            }
            if entry.skips.len() > 0 {
                write!(f, "\n      skips: {:?}", entry.skips)?;
            }
            if let Some(range) = &entry.skip_range {
                write!(f, "\n      skip_range: {}", range)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Bundle {
    name: String,
    image: String,
    package: String,
}

// Create a function for Bundle struct to implement the Display trait
impl std::fmt::Display for Bundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Bundle: {}\n  Image: {}\n  Package: {}",
            self.name, self.image, self.package
        )
    }
}

/// CLI Arguments
#[derive(Parser)]
#[command(
    name = "catalog-cli",
    version = "1.0",
    about = "CLI to browse catalog.yaml files"
)]
struct Cli {
    /// Path to the catalog.yaml file
    #[arg(short, long)]
    file: String,

    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    /// List content in the index image
    List {
        #[arg(value_enum)]
        content_type: ContentType,
    },
    /// Show details of specific content
    Show {
        /// Type of content to show
        #[arg(value_enum)]
        content_type: ContentType,

        /// Name of the content to show
        name: String,
    },
}
#[derive(clap::ValueEnum, Clone)]
enum ContentType {
    Packages,
    Channels,
    Bundles,
    Package,
    Channel,
    Bundle,
}

fn list_handler(
    content_type: ContentType,
    packages: &HashMap<String, CatalogEntry>,
    channels: &HashMap<String, Vec<CatalogEntry>>,
    bundles: &HashMap<String, Vec<CatalogEntry>>,
) {
    match content_type {
        ContentType::Packages => {
            println!("Packages:");
            for package in packages.keys() {
                println!("- {}", package);
            }
        }
        ContentType::Channels => {
            println!("Channels:");
            for entries in channels.values() {
                for entry in entries {
                    if let CatalogEntry::OpmChannel(channel) = entry {
                        println!("- {}", channel.name);
                    }
                }
            }
        }
        ContentType::Bundles => {
            println!("Bundles:");
            for entries in bundles.values() {
                for bundle in entries {
                    if let CatalogEntry::OlmBundle(bundle) = bundle {
                        println!("- {}", bundle.name);
                    }
                }
            }
        }
        _ => {
            println!("Unsupported content type");
        }
    }
}

fn show_handler(
    content_type: ContentType,
    name: &str,
    packages: &HashMap<String, CatalogEntry>,
    channels: &HashMap<String, Vec<CatalogEntry>>,
    bundles: &HashMap<String, Vec<CatalogEntry>>,
) {
    match content_type {
        ContentType::Package => {
            if let Some(entry) = packages.get(name) {
                if let CatalogEntry::OlmPackage(pkg) = entry {
                    println!("{:#?}", pkg);
                }
            }
        }
        ContentType::Channel => {
            if let Some(entries) = channels.get(name) {
                for entry in entries {
                    if let CatalogEntry::OpmChannel(channel) = entry {
                        println!("{}", channel);
                    }
                }
            }
        }
        ContentType::Bundle => {
            if let Some(entries) = bundles.get(name) {
                for entry in entries {
                    if let CatalogEntry::OlmBundle(bundle) = entry {
                        println!("{:#?}", bundle);
                    }
                }
            }
        }
        _ => {
            println!("Unsupported content type");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Read the content of the catalog file
    let content = fs::read_to_string(cli.file).expect("Failed to read the file");

    // Deserialize the content into a Vec<CatalogEntry>
    let entries: Vec<CatalogEntry> = serde_yaml::Deserializer::from_str(&content)
        .into_iter()
        .filter_map(|doc| match CatalogEntry::deserialize(doc) {
            Ok(entry) => Some(entry),
            Err(err) => {
                eprintln!("Failed to deserialize a document: {}", err);
                None
            }
        })
        .collect();

    // Organize data into a HashMap of packages
    let mut packages: HashMap<String, CatalogEntry> = HashMap::new();
    let mut channels: HashMap<String, Vec<CatalogEntry>> = HashMap::new();
    let mut bundles: HashMap<String, Vec<CatalogEntry>> = HashMap::new();

    for entry in entries {
        match &entry {
            CatalogEntry::OlmPackage(pkg) => {
                packages.insert(pkg.name.clone(), entry);
                // packages.entry(pkg.name.clone()) = entry;
            }
            CatalogEntry::OpmChannel(chan) => {
                channels
                    .entry(chan.package.clone())
                    .or_default()
                    .push(entry);
            }
            CatalogEntry::OlmBundle(bund) => {
                bundles.entry(bund.package.clone()).or_default().push(entry);
            }
        }
    }

    // Handle CLI commands
    match cli.command {
        Commands::List { content_type } => {
            list_handler(content_type, &packages, &channels, &bundles)
        }

        Commands::Show { content_type, name } => {
            show_handler(content_type, &name, &packages, &channels, &bundles)
        }
    }

    Ok(())
}
