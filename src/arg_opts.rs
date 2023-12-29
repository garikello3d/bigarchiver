use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bigarchiver")]
#[command(author = "Igor Bezzubchenko")]
#[command(version = "0.0.1")]
#[command(about = "Reliably backup/restore data with compression and encryption", long_about = None)]
pub struct ArgOpts {
    #[command(subcommand)]
    pub command: Commands
}

#[derive(Subcommand)]
pub enum Commands {
    /// Backup mode: read data from stdin and write into output files(s)
    Backup {
        /// Template for output chunks; '%' symbols will transform into a sequence number
        #[arg(long, value_name = "path_with_%")]
        out_template: String,

        /// Password to encrypt data with
        #[arg(long, value_name = "password")]
        pass: String,

        /// Public authentication data to embed
        #[arg(long, value_name = "string")]
        auth: String,

        /// Embed authentication data to each portion of data of indicated size, in MB
        #[arg(long, value_name = "size_mb")]
        auth_every: usize,

        /// Size of output chunks, in MB
        #[arg(long, value_name = "size_mb")]
        split_size: usize,

        /// LZMA compression level, 0 - 9
        #[arg(long, value_name = "level")]
        compress_level: u8,

        /// Buffer size for reading stdin data, in MB
        #[arg(long, value_name ="size_mb")]
        buf_size: usize,

        /// Do not check the integrity of the whole archive after backup is done (the default is to always check)
        #[arg(long, action)]
        no_check: bool
    },
    /// Restore mode: restore data from file(s) and write into stdout
    Restore {
        /// Full path to config file of the archive to restore
        #[arg(long, value_name = "full_path")]
        config: String,

        /// Password to decrypt data with
        #[arg(long, value_name = "password")]
        pass: String,

        /// Buffer size for reading disk files, in MB
        #[arg(long, value_name ="size_mb")]
        buf_size: usize,

        /// Check free space available on the indicated filesystem before restore
        #[arg(long, value_name = "mountpoint_or_path")]
        check_free_space: Option<String>,

        /// Do not check the integrity of the whole archive before actual restore (the default is to always check)
        #[arg(long, action)]
        no_check: bool
    },
    /// Check mode: check integrity of data from file(s)
    Check {
        /// Full path to config file of the archive to restore
        #[arg(long, value_name = "full_path")]
        config: String,

        /// Password to decrypt data with
        #[arg(long, value_name = "password")]
        pass: String,

        /// Buffer size for reading disk files, in MB
        #[arg(long, value_name ="size_mb")]
        buf_size: usize,
    }
}
