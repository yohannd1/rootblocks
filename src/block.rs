use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

pub use updater::*;

pub type BlockID = usize;

pub mod updater {
    use crate::x11::XDisplay;
    use std::io::Write;

    pub trait Updater {
        fn update(&mut self, slots: &[String]);
    }

    #[derive(Debug, Clone)]
    pub struct UpdaterConfig<'a> {
        pub prefix: &'a str,
        pub postfix: &'a str,
        pub separator: &'a str,
    }

    pub struct WriteUpdater<'a, W> {
        pub config: UpdaterConfig<'a>,
        pub writable: &'a mut W,
    }

    impl<'a, W> WriteUpdater<'a, W>
    where
        W: Write,
    {
        pub fn new(writable: &'a mut W, config: UpdaterConfig<'a>) -> Self {
            Self { writable, config }
        }
    }

    impl<W> Updater for WriteUpdater<'_, W>
    where
        W: Write,
    {
        fn update(&mut self, slots: &[String]) {
            write!(
                &mut self.writable,
                "{}{}{}",
                self.config.prefix,
                slots.join(self.config.separator),
                self.config.postfix,
            )
            .unwrap();

            self.writable.flush().unwrap();
        }
    }

    pub struct XRootUpdater<'a> {
        pub config: UpdaterConfig<'a>,
    }

    impl<'a> XRootUpdater<'a> {
        pub fn new(config: UpdaterConfig<'a>) -> Self {
            Self {
                config: config,
            }
        }
    }

    impl Updater for XRootUpdater<'_> {
        fn update(&mut self, slots: &[String]) {
            let string = format!(
                "{}{}{}",
                self.config.prefix,
                slots.join(self.config.separator),
                self.config.postfix
            );

            XDisplay::open()
                .expect("X Display unavailable")
                .default_screen()
                .root_window()
                .set_name(&string)
                .expect("Failed to set name");
        }
    }
}

pub trait Block {
    fn run(&self, message_passer: MessagePasser) -> JoinHandle<()>;
}

pub struct MessagePasser {
    id: usize,
    sender: mpsc::Sender<(BlockID, String)>,
}

impl MessagePasser {
    pub fn send(&self, message: String) {
        self.sender.send((self.id, message)).unwrap();
    }
}

pub struct BlockManager<'a, U>
where
    U: Updater,
{
    handles: Vec<Option<JoinHandle<()>>>,
    slots: Vec<String>,
    updater: &'a mut U,
    receiver: mpsc::Receiver<(BlockID, String)>,
}

impl<'a, U> BlockManager<'a, U>
where
    U: Updater,
{
    pub fn new(blocks: Vec<Box<dyn Block + '_>>, updater: &'a mut U) -> Self {
        let (sender, receiver) = mpsc::channel();
        let blocks_len = blocks.len();

        BlockManager {
            handles: blocks
                .into_iter()
                .enumerate()
                .map(|(id, block)| {
                    Some(block.run(MessagePasser {
                        id: id,
                        sender: sender.clone(),
                    }))
                })
                .collect(),
            slots: vec![String::from(""); blocks_len],
            updater: updater,
            receiver: receiver,
        }
    }

    pub fn start(&mut self) {
        'main_loop: loop {
            if let Ok((id, string)) = self.receiver.recv() {
                self.slots[id] = string;
            } else {
                break 'main_loop;
            }

            // Sleep for 0.05 seconds, then empty the receiver without blocking it, in an attempt to call the update
            // function (which might be somewhat expensive) as little as possible with multiple simultaneous requests.
            thread::sleep(Duration::from_millis(0050));
            'receive_rest: loop {
                match self.receiver.try_recv() {
                    Ok((id, string)) => self.slots[id] = string,
                    Err(mpsc::TryRecvError::Empty) => break 'receive_rest,
                    Err(mpsc::TryRecvError::Disconnected) => break 'main_loop,
                }
            }

            // Finally update.
            self.updater.update(&self.slots);
        }

        eprintln!("All blocks were finished. Shutting down...");
    }
}

impl<U> Drop for BlockManager<'_, U>
where
    U: Updater,
{
    fn drop(&mut self) {
        for handle in &mut self.handles {
            if let Some(handle) = handle.take() {
                handle.join().unwrap();
            }
        }
    }
}

#[macro_export]
macro_rules! make_cmd_block {
    ($name:ident, $command:expr, interval=$interval:expr, shell=$shell:expr $(,)?) => {
        struct $name;
        impl $crate::block::Block for $name {
            fn run(&self, mp: $crate::block::MessagePasser) -> ::std::thread::JoinHandle<()> {
                ::std::thread::spawn(move || loop {
                    let output = ::std::process::Command::new($shell)
                        .arg("-c")
                        .arg($command)
                        .output()
                        .unwrap()
                        .stdout;

                    mp.send(::std::string::String::from_utf8_lossy(&output).trim().to_string());
                    ::std::thread::sleep($interval);
                })
            }
        }
    };
    ($name:ident, $command:expr, interval=$interval:expr $(,)?) => {
        make_cmd_block!($name, $command, interval=$interval, shell="bash")
    }
}
