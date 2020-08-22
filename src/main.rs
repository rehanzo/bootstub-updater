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

mod version;

#[derive(StructOpt, Serialize, Deserialize, Clone, Debug)]
#[structopt(about = "Updates efibootstub when linux kernel is updated")]
struct Args {
    #[structopt(
        short,
        long,
        value_name = "COMMAND",
        about = "Specify command to be run when linux kernel is updated. Place \"%v\" where kernel version should be",
        required_unless = "path"
    )]
    command: Option<String>,

    #[structopt(
        short,
        long,
        value_name = "NUM",
        about = "Specify bootnum of entry to be replaced",
        required_unless = "path"
    )]
    bootnum: Option<String>,

    #[structopt(
        short,
        long,
        value_name = "FILENAME",
        about = "Provide an example of the naming format your distro follows, replacing the verison number with %v. Ex. vmlinuz-%v",
        required_unless = "path"
        )]
    format: Option<String>,

    #[structopt(
        name = "path",
        short = "p",
        long = "path",
        value_name = "PATH",
        about = "Specify location of a TOML formatted file containing arguments for the program",
        )]
    #[serde(skip)]
    config_location: Option<String>
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

    if !debug {
        watcher.watch("/boot", RecursiveMode::Recursive).expect("Error in watching directory");
    } else {
        watcher.watch("/home/rehan/Downloads/test", RecursiveMode::Recursive).expect("Error in watching directory");
    }
    let format = args.format.expect("Missing argument: format");
    let bootnum = args.bootnum.expect("Missing argument: bootnum");
    let command = args.command.expect("Missing argument: command");

    loop {
        match rx.recv() {
            Ok(event) => {
                //if file is created in watched directory
                if let DebouncedEvent::Create(path) = event {
                    let file_name = path.file_name().unwrap();
                    let file_name = file_name.to_str().unwrap();
                    if file_name.contains("vmlinuz") {
                        let mut uname = Command::new("uname");
                        uname.arg("-r");
                        let mut uname = uname.output().expect("Failed to extract output from uname");
                        //for some reason there is an additional element at the end of the stdout
                        //output that messes everything up, so we gotta pop it
                        uname.stdout.pop();
                        let curr_version = String::from_utf8(uname.stdout).unwrap();

                        let curr_version = Version::new(String::from(curr_version), None);
                        let new_version = Version::new(String::from(file_name), Some(&format));

                        if new_version > curr_version {
                            if let Err(e) = run_command(&new_version.string, &bootnum, &command, debug) {
                                eprintln!("{}", e);
                            }
                        }
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
