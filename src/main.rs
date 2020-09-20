use std::env;
use std::error::Error;
use std::process::exit;
use std::fs::OpenOptions;

use getopts::Options;

use serde_derive::Serialize;

use chrono::{DateTime, Utc};
use std::io::Write;

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct BiscuitSnapshot {
    name: String,
    datetime: DateTime<Utc>,
    package_infos: Vec<PackageInfo>,
}

impl BiscuitSnapshot {
    pub fn create_with_name(name: &str) -> BiscuitSnapshot {
        BiscuitSnapshot {
            name: name.to_string(),
            datetime: Utc::now(),
            package_infos: Vec::new(),
        }
    }

    pub fn add_package_info(&mut self, name: &str, version: &str) {
        self.package_infos.push(PackageInfo{
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    pub fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let toml = toml::to_string(&self)?;
        let mut out_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(filename)?;
        out_file.write_all(toml.as_bytes()).map_err(|e| e.into())
    }
}

fn write_to_snapshot(snapshot: &mut BiscuitSnapshot, root_path: &str, database_path: &str) -> Result<(), Box<dyn Error>> {
    let handle = alpm_rs::initialize(root_path, database_path)?;
    let db = handle.local_db();
    let packages = db.pkgcache();

    for p in packages {
        snapshot.add_package_info(p.name(), p.version());
    }

    Ok(())
}

fn show_usage(launch_name: &str, opts: Options) {
    let brief = format!("Usage:\n{} [-h]", launch_name);
    eprintln!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let launch_name = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print usage");
    opts.optopt("n", "name", "[Required] the name of the snapshot", "NAME");
    opts.optopt("o", "output", "the output filename (default = \"NAME.toml\")", "FILE");
    opts.optopt("r", "root-path", "the absolute path to the system root filesystem (default = \"/\")", "PATH");
    opts.optopt("d", "db-path", "the absolute path to the ALPM database (default = \"/var/lib/pacman\")", "PATH");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            eprintln!("Bad arguments: {}", f.to_string());
            show_usage(&launch_name, opts);
            exit(1);
        }
    };

    if matches.opt_present("h") {
        show_usage(&launch_name, opts);
        exit(0);
    }

    if !matches.opt_present("n") {
        eprintln!("Missing required argument: name");
        show_usage(&launch_name, opts);
        exit(1);
    }

    let name = matches.opt_str("n").unwrap();
    let output_filename = matches.opt_str("o").unwrap_or(format!("{}.toml", name));
    let root_path = matches.opt_str("r").unwrap_or(String::from("/"));
    let database_path = matches.opt_str("d").unwrap_or(String::from("/var/lib/pacman"));
    let mut snapshot = BiscuitSnapshot::create_with_name(&name);
    match write_to_snapshot(&mut snapshot, &root_path, &database_path) {
        Ok(_) => {
            match snapshot.save_to_file(&output_filename) {
                Ok(_) => exit(0),
                Err(e) => {
                    eprintln!("Something went wrong while saving the snapshot to the file: {}", e.to_string());
                    exit(1);
                }
            }
        },
        Err(e) => {
            eprintln!("Something went wrong while reading the ALPM database: {}", e.to_string());
            exit(1);
        }
    }
}
