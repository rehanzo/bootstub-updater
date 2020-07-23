use notify::{RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::error::Error;
use clap::Clap;
use std::process::Command;
use std::thread::sleep;

#[derive(Clap)]
#[clap(about = "Updates efibootstub when linux kernel is updated")]
struct Args {
    #[clap(short, long, value_name = "COMMAND", about = "Specify command to be run when linux kernel is updated. Place !VERSION where kernel version should be", required = true)]
    command: String,

    #[clap(short, long, value_name = "NUM", about = "Specify bootnum of entry to be replaced", required = true)]
    bootnum: String,
}

fn run_command(vnum: &str, args: &Args) -> Result<(), Box<dyn Error>> {
    let mut command_split = args.command.split_whitespace();
    let mut rm_handle = Command::new("efibootmgr");
    let mut create_handle = Command::new(command_split.next().unwrap());
    rm_handle.arg("-b")
        .arg(&args.bootnum)
        .arg("-B");
    for word in command_split {
        if word.contains("!VERSION") {
            let word_mod = word.replace("!VERSION", vnum);

            create_handle.arg(word_mod);
        }

        else {
            create_handle.arg(word);
        }
    }
    rm_handle.spawn().unwrap();
    sleep(Duration::from_secs(1));
    create_handle.spawn().unwrap();

    Ok(())
}

fn watch(args: Args) -> notify::Result<()> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch("/boot", RecursiveMode::Recursive).unwrap();

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        let mut vnum: &str = "";
        match rx.recv() {
            Ok(event) => {
                if let DebouncedEvent::Create(path) = event {
                    let path = path.file_name().unwrap();
                    let path = path.to_str().unwrap();
                    if path.contains("vmlinuz") {
                        vnum = &path[8..];
                        run_command(vnum, &args).unwrap();

                    }
                        
                }

            }
            Err(e) => println!("{:?}", e),
        }
    }
}

fn main() {
    let args: Args = Args::parse();
    if let Err(e) = watch(args) {
        println!("error: {:?}", e)
    }
}
