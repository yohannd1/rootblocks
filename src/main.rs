pub mod block;
pub mod x11;

// TODO: replace a lot of the `unwrap` with `expect`

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use block::updater::{UpdaterConfig, XRootUpdater};
use block::{Block, BlockManager, MessagePasser};

const DEFAULT_INTERVAL: Duration = Duration::from_secs(10);

fn main() {
    let blocks: Vec<Box<dyn Block>> = vec![
        Box::new(Ram),
        Box::new(Cpu),
        Box::new(Battery),
        Box::new(Volume),
        Box::new(Clock),
    ];

    let mut updater = XRootUpdater::new(UpdaterConfig {
        prefix: "[ ",
        postfix: " ]",
        separator: " | ",
    });

    let mut manager = BlockManager::new(blocks, &mut updater);
    manager.start();
}

struct Battery;
impl Block for Battery {
    fn run(&self, message_passer: MessagePasser) -> JoinHandle<()> {
        thread::spawn(move || loop {
            const BATTERIES_DIR: &'static str = "/sys/class/power_supply";

            let batteries = std::fs::read_dir(BATTERIES_DIR)
                .expect("Failed to read batteries directory")
                .map(|b| b.unwrap())
                .filter(|battery_name| {
                    battery_name.path().file_name().map_or(false, |f| {
                        f.to_str()
                            .expect("failed to convert OsStr into str")
                            .starts_with("BAT")
                    })
                })
                .map(|battery_name| {
                    let mut buf = Path::new(BATTERIES_DIR).to_path_buf();
                    buf.push(battery_name.path());
                    buf
                });

            let string = batteries
                .map(|battery_path| {
                    let capacity = match std::fs::read_to_string(&battery_path.join("capacity")) {
                        Ok(contents) => format!("{}%", contents.trim()),
                        Err(_) => "[BADFILE]".into(),
                    };

                    let status = match std::fs::read_to_string(&battery_path.join("status")) {
                        Ok(contents) => match contents.to_lowercase().as_str().trim() {
                            "discharging" => "bat",
                            "not charging" => "not",
                            "charging" => "chr",
                            "unknown" => "???",
                            "full" => "max",
                            _ => "[INVALID]",
                        },
                        Err(_) => "[BADFILE]",
                    };

                    format!("{} {}", status, capacity)
                })
                .collect::<Vec<_>>()
                .join(", ");

            message_passer.send(string);
            thread::sleep(DEFAULT_INTERVAL);
        })
    }
}

struct Volume;

impl Volume {
    fn get() -> String {
        let stdout = std::process::Command::new("amixer")
            .arg("get")
            .arg("Master")
            .output()
            .unwrap()
            .stdout;

        let string = String::from_utf8_lossy(&stdout);
        let lines = string.split("\n").collect::<Vec<&str>>();

        let is_on: Option<bool> = match lines[lines.len() - 2].split("[").nth(3).unwrap() {
            "on]" => Some(true),
            "off]" => Some(false),
            _ => None,
        };

        match is_on {
            Some(true) => {
                let percentage = lines[lines.len() - 2]
                    .split("[")
                    .nth(1)
                    .unwrap()
                    .split("]")
                    .nth(0)
                    .unwrap();

                format!("vol {}", percentage)
            }
            Some(false) => {
                format!("vol OFF")
            }
            None => {
                format!("vol UNK")
            }
        }
    }
}

impl Block for Volume {
    fn run(&self, message_passer: MessagePasser) -> JoinHandle<()> {
        thread::spawn(move || {
            let (s, receiver) = mpsc::channel();

            let s2 = s.clone();
            let periodic_handle = thread::spawn(move || loop {
                if let Err(_) = s2.send(()) {
                    break;
                }

                thread::sleep(DEFAULT_INTERVAL);
            });

            let pactl_handle = thread::spawn(move || {
                let pactl = std::process::Command::new("pactl")
                    .arg("subscribe")
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .expect("failed to start `pactl subscribe`");

                let reader = BufReader::new(pactl.stdout.unwrap());

                for line in reader.lines() {
                    if line.unwrap().contains("sink") {
                        if let Err(_) = s.send(()) {
                            break;
                        }
                    }
                }
            });

            while let Ok(()) = receiver.recv() {
                message_passer.send(Self::get());
            }

            // FIXME: might get stuck
            periodic_handle.join().unwrap();
            pactl_handle.join().unwrap();
        })
    }
}

make_cmd_block!(
    Cpu,
    r#"top -bn1 | grep 'Cpu(s)' | awk '{printf "cpu %02.0f%%", $2}'"#,
    interval = DEFAULT_INTERVAL,
    shell = "sh",
);

make_cmd_block!(
    Ram,
    r#"free -m | awk 'NR==2 {printf "ram %.0f%%", $3*100/$2 }'"#,
    interval = DEFAULT_INTERVAL,
    shell = "sh",
);

#[allow(dead_code)]
make_cmd_block!(
    Swap,
    r#"free -m | awk 'NR==3 {printf "swap %.0f%%", $3*100/$2 }'"#,
    interval = DEFAULT_INTERVAL,
    shell = "sh",
);

make_cmd_block!(
    Clock,
    r#"date "+%Y-%m-%d %H:%M""#,
    interval = DEFAULT_INTERVAL,
    shell = "sh",
);
