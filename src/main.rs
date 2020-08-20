use clap::Clap;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::error::Error;
use std::process::Command;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;
use std::env;
use std::str;
use efibootstub_updater::Version;

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

fn run_command(vnum: &str, args: &Args, debug: bool) -> Result<(), Box<dyn Error>> {
    let command_split = args.command.split("'");
    let mut rm_handle = Command::new("efibootmgr");
    let mut create_handle = Command::new("");
    let mut first_run = true;
    let mut sing_quote_switch = false;
    rm_handle.arg("-b").arg(&args.bootnum).arg("-B");
    for block in command_split {
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

fn watch(args: Args) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();
    let debug = env::var("EFI_DBG").is_ok();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    if !debug {
        watcher.watch("/boot", RecursiveMode::Recursive).unwrap();
    } else {
        watcher.watch("/home/rehan/Downloads/test", RecursiveMode::Recursive).unwrap();
    }

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        let vnum: &str;
        match rx.recv() {
            Ok(event) => {
                if let DebouncedEvent::Create(path) = event {
                    let file_name = path.file_name().unwrap();
                    let file_name = file_name.to_str().unwrap();
                    if file_name.contains("vmlinuz") {
                        let mut uname = Command::new("uname");
                        uname.arg("-r");
                        let mut uname = uname.output().unwrap();
                        //for some reason there is an additional element at the end of the stdout
                        //output that messes everything up, so we gotta pop it
                        uname.stdout.pop();
                        let curr_version = String::from_utf8(uname.stdout.clone()).unwrap();

                        let curr_version = Version::new(String::from(curr_version), args.format.clone(), false);
                        let new_version = Version::new(String::from(file_name), args.format.clone(), true);

                        if new_version > curr_version {
                            vnum = &new_version.string;
                            run_command(vnum, &args, debug).unwrap();
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
