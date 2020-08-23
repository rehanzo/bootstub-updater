use structopt::StructOpt;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::str;
use version::Version;
use serde::{Deserialize, Serialize};
use std::fs::{ File, canonicalize };
use std::io::Read;
use glob::glob;

mod version;

#[derive(StructOpt, Serialize, Deserialize, Clone, Debug)]
#[structopt(about = "Updates efibootstub when linux kernel is updated")]
struct Args {
    #[structopt(
        short,
        long,
        value_name = "COMMAND",
        about = "Specify command to be run when linux kernel is updated. Place \"%v\" where kernel version should be",
        required_unless = "toml"
    )]
    command: Option<String>,

    #[structopt(
        short,
        long,
        value_name = "NUM",
        about = "Specify bootnum of entry to be replaced",
        required_unless = "toml"
    )]
    bootnum: Option<String>,

    #[structopt(
        short,
        long,
        value_name = "FILENAME",
        about = "Provide an example of the naming format your distro follows, replacing the verison number with %v. Ex. vmlinuz-%v",
        required_unless = "toml"
        )]
    format: Option<String>,

    #[structopt(
        name = "toml",
        short = "t",
        long = "toml",
        value_name = "PATH",
        about = "Specify location of a TOML formatted file containing arguments for the program",
        )]
    #[serde(skip)]
    config_location: Option<String>,

    #[structopt(
        short = "k",
        long = "kernel-dir",
        value_name = "PATH",
        about = "Specify location of the kernel",
        )]
    #[serde(rename = "kernel-dir")]
    kernel_dir: Option<String>
}

///Runs the commands for deleting the boot entry and adding a new one
///
///vnum - the version portion of the linux kernel file name
///bootnum - bootnum to remove from efibootmgr
///command - command to run to create new boot entry
///debug - true if in debug mode, false otherwise
fn run_command(vnum: &str, bootnum: &str, command: &str, debug: bool) -> Result<(), Box<dyn Error>> {
    let command_split = command;
    let command_split = command_split.split("'");
    let mut rm_handle = Command::new("efibootmgr");
    let mut create_handle = Command::new("");
    let mut first_run = true;
    let mut sing_quote_switch = false;
    rm_handle.arg("-b").arg(&bootnum).arg("-B");
    for block in command_split {
        //sing_quote_switch is used to determine if something was sent in with single quotes so
        //that the entire block within the single quotes can be sent in as one argument, vs sending
        //in single words from the argument in the quotes
        if sing_quote_switch {
            create_handle.arg(block.replace("%v", vnum));
        } else {
            for word in block.split_whitespace() {
                if first_run {
                    first_run = false;
                    create_handle = Command::new(word);
                } else {
                    create_handle.arg(word.replace("%v", vnum));
                }
            }
        }
        sing_quote_switch = !sing_quote_switch;
    }

    if !debug {
        rm_handle.spawn().unwrap();
    }
    //the remove and add command seem like they run in random order without sleeping
    sleep(Duration::from_secs(1));
    create_handle.spawn().unwrap();
    Ok(())
}

///watches directory for changes
fn watch(args: Args) -> notify::Result<()> {
    let (tx, rx) = channel();
    //set environment variable EFI_DBG to get debug mode going
    let debug = env::var("EFI_DBG").is_ok();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();
    let format = args.format.expect("Missing argument: format");
    let bootnum = args.bootnum.expect("Missing argument: bootnum");
    let command = args.command.expect("Missing argument: command");
    let kernel_dir = args.kernel_dir.unwrap_or(String::from("/boot"));

    watcher.watch(&kernel_dir, RecursiveMode::Recursive).expect("Error in watching directory");

    loop {
        match rx.recv() {
            Ok(event) => {
                if let DebouncedEvent::NoticeRemove(_) = event {
                    continue;
                }
                else {
                    let mut ver_vec: Vec<Version> = vec![];
                    for entry in glob(&format!("{}/*vmlinuz*", kernel_dir)).expect("Failed to read glob pattern") {
                        match entry {
                            Ok(path) => { 
                                ver_vec.push(Version::new(path.file_name().unwrap().to_str().unwrap(), &format));
                            },
                            Err(e) => eprintln!("{:?}", e),
                        }
                    } 

                    let max = ver_vec.into_iter().max().unwrap();
                    if let Err(e) = run_command(&max.string, &bootnum, &command, debug) {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn config_parse(config_location: &str) -> Result<Args, Box<dyn Error>> {
    let config_location = canonicalize(config_location).expect("Failed to canonicalize, check file path");
    let config_location = config_location.to_str().unwrap();
    
    let file = File::open(config_location);

    if file.is_err() {
        return Err(format!("File not found at {}",config_location).into());
    }
    let mut file = file.unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let args_string = toml::from_str(contents.as_str());
    let args_serialized: Args = args_string?;
    Ok(args_serialized)
}

fn main() {
    let mut args: Args = Args::from_args();
        
    if let Some(config_location) = &args.config_location {
        match config_parse(config_location) {
            Err(e) => eprintln!("{}", e),
            Ok(s) => args = s,
        }
    }
    if let Err(e) = watch(args) {
        eprintln!("error: {:?}", e)
    }
}
