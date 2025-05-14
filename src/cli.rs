use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "beepkg")]
#[command(about = "Generic Package Manager supporting multiple languages", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available packages
    List {
        /// MinIO endpoint URL
        #[arg(short, long)]
        endpoint: String,

        /// MinIO bucket name
        #[arg(short, long)]
        bucket: String,
    },

    /// Push a package to registry
    Push {
        /// Path to package directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        package: String,

        /// MinIO access key
        #[arg(short, long)]
        key: Option<String>,

        /// MinIO secret key
        #[arg(short, long)]
        secret: Option<String>,

        /// Force push (overwrite existing package or ignore version warnings)
        #[arg(short, long)]
        force: bool,
    },

    /// Pull a package from registry
    Pull {
        /// Package name and version (e.g. demo-pkg@2.1.0)
        package: String,

        /// Output directory
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Test connection to MinIO server and bucket
    Test {
        /// MinIO endpoint URL (optional, defaults to S3_ENDPOINT env var)
        #[arg(short, long)]
        endpoint: Option<String>,

        /// MinIO bucket name (optional, defaults to S3_BUCKET env var)
        #[arg(short, long)]
        bucket: Option<String>,

        /// MinIO access key (optional)
        #[arg(short, long)]
        key: Option<String>,

        /// MinIO secret key (optional)
        #[arg(short, long)]
        secret: Option<String>,
    },

    /// Lock a package to prevent modifications
    Lock {
        /// Package name and version (e.g. demo-pkg@2.1.0)
        package: String,

        /// Reason for locking the package
        #[arg(short, long)]
        reason: String,

        /// Username of the person locking the package
        #[arg(short, long)]
        user: String,
    },

    /// Unlock a previously locked package
    Unlock {
        /// Package name and version (e.g. demo-pkg@2.1.0)
        package: String,
    },

    /// Backup a package version
    Backup {
        /// Package name and version (e.g. demo-pkg@2.1.0)
        package: String,

        /// Reason for creating the backup
        #[arg(short, long)]
        reason: String,
    },

    /// Restore a package from backup
    Restore {
        /// Package name and version (e.g. demo-pkg@2.1.0)
        package: String,

        /// Specific backup timestamp (optional, uses latest if not specified)
        #[arg(short, long)]
        timestamp: Option<String>,
    },

    /// Configure package encryption
    Encrypt {
        /// Path to package directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        package: String,

        /// Enable encryption
        #[arg(short, long)]
        enable: bool,

        /// Encryption algorithm (default: aes-256-gcm)
        #[arg(short, long, default_value = "aes-256-gcm")]
        algorithm: String,
    },
}
