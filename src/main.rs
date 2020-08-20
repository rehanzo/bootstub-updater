use clap::Clap;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::str;
use version::Version;

mod version;

#[derive(Clap)]
#[clap(about = "Updates efibootstub when linux kernel is updated")]
struct Args {
    #[clap(
        short,
        long,
        value_name = "COMMAND",
        about = "Specify command to be run when linux kernel is updated. Place \"%v\" where kernel version should be",
        required = true
    )]
    command: String,

    #[clap(
        short,
        long,
        value_name = "NUM",
        about = "Specify bootnum of entry to be replaced",
        required = true
    )]
    bootnum: String,

    #[clap(
        short,
        long,
        value_name = "FILENAME",
        about = "Provide an example of the naming format your distro follows, replacing the verison number with %v. Ex. vmlinuz-%v",
        required = true
        )]
    format: String,
}

///Runs the commands for deleting the boot entry and adding a new one
///
///vnum - the version portion of the linux kernel file name
///args - struct with cli arguments
///debug - true if in debug mode, false otherwise
fn run_command(vnum: &str, args: &Args, debug: bool) -> Result<(), Box<dyn Error>> {
    let command_split = args.command.split("'");
    let mut rm_handle = Command::new("efibootmgr");
    let mut create_handle = Command::new("");
    let mut first_run = true;
    let mut sing_quote_switch = false;
    rm_handle.arg("-b").arg(&args.bootnum).arg("-B");
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
                        let new_version = Version::new(String::from(file_name), Some(&args.format));

                        if new_version > curr_version {
                            if let Err(e) = run_command(&new_version.string, &args, debug) {
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

fn main() {
    let args: Args = Args::parse();
    if let Err(e) = watch(args) {
        eprintln!("error: {:?}", e)
    }
}
