// SPDX-License-Identifier: APACHE-2.0 AND MIT
use clap::arg_enum;
use flatbuffers::FlatBufferBuilder;
use futures::executor::{block_on, LocalPool};
use futures_executor::{enter, ThreadPool};
use futures_util::{stream::StreamExt, task::LocalSpawnExt, task::SpawnExt};
use rustmq::{prelude::*, Error};
use std::thread;

arg_enum! {
    enum Runtime {
        ThreadPool,
        LocalPool,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::parse();
    match cfg.runtime {
        Runtime::ThreadPool => thread_pool(cfg),
        Runtime::LocalPool => local_pool(cfg),
    }
}

fn thread_pool(cfg: Config) -> Result<(), Box<dyn std::error::Error>> {
    let pool = ThreadPool::new()?;
    let client = Client::new();
    let request_queue = "request";

    // One connection for multiple thread pool producers and consumers each.
    let producer_conn = block_on(client.connect(&cfg.uri))?;
    let consumer_conn = block_on(client.connect(&cfg.uri))?;

    let enter = enter()?;
    let mut builder = producer_conn.producer_builder();
    builder.with_queue(String::from(request_queue));
    for _ in 0..cfg.producers {
        let builder = builder.clone();
        pool.spawn(async move {
            match builder.build().await {
                Err(e) => eprintln!("{}", e),
                Ok(p) => {
                    let mut p = ASCIIGenerator(p);
                    if let Err(err) = p.run().await {
                        eprintln!("{}", err);
                    }
                }
            }
        })?;
    }
    let mut builder = consumer_conn.consumer_builder();
    builder.with_queue(String::from(request_queue));
    for _ in 0..cfg.consumers {
        let builder = builder.clone();
        pool.spawn(async move {
            match builder.build().await {
                Err(err) => eprintln!("{}", err),
                Ok(c) => {
                    let mut c = EchoConsumer(c);
                    if let Err(err) = c.run().await {
                        eprintln!("{}", err);
                    }
                }
            }
        })?;
    }
    drop(enter);

    // idle loop.
    loop {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn local_pool(cfg: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut threads = Vec::new();
    let client = Client::new();
    let request_queue = "request";

    // A single connection for multiple local pool producers.
    let conn = block_on(client.connect(&cfg.uri))?;
    let mut builder = conn.producer_builder();
    builder.with_queue(String::from(request_queue));
    for _ in 0..cfg.producers {
        let builder = builder.clone();
        let producer = thread::spawn(move || {
            LocalPool::new().run_until(async {
                match builder.build().await {
                    Err(e) => eprintln!("{}", e),
                    Ok(p) => {
                        let mut p = ASCIIGenerator(p);
                        if let Err(err) = p.run().await {
                            eprintln!("{}", err);
                        }
                    }
                }
            });
        });
        threads.push(producer);
    }

    // A single connection for multiple local pool consumers.
    let consumers_per_thread = cfg.consumers_per_thread;
    let consumers = cfg.consumers / consumers_per_thread;
    let conn = block_on(client.connect(&cfg.uri))?;
    let mut builder = conn.consumer_builder();
    builder.with_queue(String::from(request_queue));
    for _ in 0..consumers {
        let builder = builder.clone();
        let consumer = thread::spawn(move || {
            let mut pool = LocalPool::new();
            let spawner = pool.spawner();
            for _ in 0..consumers_per_thread {
                let builder = builder.clone();
                if let Err(err) = spawner.spawn_local(async move {
                    match builder.build().await {
                        Err(err) => eprintln!("{}", err),
                        Ok(c) => {
                            let mut c = EchoConsumer(c);
                            if let Err(err) = c.run().await {
                                eprintln!("{}", err);
                            }
                        }
                    }
                }) {
                    eprintln!("{:?}", err);
                }
            }
            pool.run();
        });
        threads.push(consumer);
    }

    // Cleanup all instances.
    for t in threads {
        if let Err(err) = t.join() {
            eprintln!("{:?}", err);
        }
    }
    Ok(())
}

struct ASCIIGenerator(Producer);

impl ASCIIGenerator {
    async fn run(&mut self) -> Result<(), Error> {
        let mut builder = FlatBufferBuilder::new();
        loop {
            // Generate ASCII character FlatBuffer messages
            // and print the received message to stderr.
            for data in { b'!'..=b'~' } {
                let req = Self::make_buf(&mut builder, vec![data]);
                let resp = self.0.rpc(req).await?;
                Self::print_buf(resp);
            }
        }
    }
    fn make_buf(builder: &mut FlatBufferBuilder, data: Vec<u8>) -> Vec<u8> {
        let data = builder.create_string(&String::from_utf8(data).unwrap());
        let mut mb = crate::msg::MessageBuilder::new(builder);
        mb.add_msg(data);
        let msg = mb.finish();
        builder.finish(msg, None);
        let req = builder.finished_data().to_vec();
        builder.reset();
        req
    }
    fn print_buf(resp: Vec<u8>) {
        if resp.is_empty() {
            return;
        }
        let msg = crate::msg::get_root_as_message(&resp);
        if let Some(data) = msg.msg() {
            eprint!("{}", data);
        }
    }
}

struct EchoConsumer(Consumer);

impl EchoConsumer {
    async fn run(&mut self) -> Result<(), Error> {
        while let Some(msg) = self.0.next().await {
            match msg {
                // Echo back the message.
                Ok(req) => self.0.response(&req, req.data()).await?,
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

const PRODUCERS: usize = 32;
const CONSUMERS: usize = 64;
const CONSUMERS_PER_THREAD: usize = 8;

struct Config {
    uri: String,
    runtime: Runtime,
    producers: usize,
    consumers: usize,
    consumers_per_thread: usize,
}

impl Config {
    fn parse() -> Self {
        use clap::{value_t, App, Arg, SubCommand};
        let producers_str = PRODUCERS.to_string();
        let consumers_str = CONSUMERS.to_string();
        let consumers_per_thread = CONSUMERS_PER_THREAD.to_string();
        let opts = App::new("rustmq crate example")
            .author("Keith Noguchi <keith.noguchi@gmail.com>")
            .arg(
                Arg::with_name("runtime")
                    .short("r")
                    .long("runtime")
                    .help("Rust runtime")
                    .takes_value(true)
                    .default_value("thread-pool")
                    .possible_values(&["thread-pool", "local-pool"]),
            )
            .arg(
                Arg::with_name("username")
                    .short("u")
                    .long("username")
                    .help("AMQP username")
                    .takes_value(true)
                    .default_value("rabbit"),
            )
            .arg(
                Arg::with_name("password")
                    .short("p")
                    .long("password")
                    .help("AMQP password")
                    .takes_value(true)
                    .default_value("RabbitMQ"),
            )
            .arg(
                Arg::with_name("scheme")
                    .short("s")
                    .long("scheme")
                    .help("AMQP scheme")
                    .takes_value(true)
                    .default_value("amqp")
                    .possible_values(&["amqp", "amqps"]),
            )
            .arg(
                Arg::with_name("cluster")
                    .short("c")
                    .long("cluster")
                    .help("AMQP cluster")
                    .takes_value(true)
                    .default_value("127.0.0.1:5672"),
            )
            .arg(
                Arg::with_name("vhost")
                    .short("v")
                    .long("vhost")
                    .help("AMQP vhost name")
                    .takes_value(true)
                    .default_value("mx"),
            )
            .subcommand(
                SubCommand::with_name("tune")
                    .about("Tuning parameters")
                    .arg(
                        Arg::with_name("producers")
                            .short("p")
                            .long("producers")
                            .help("Number of producers")
                            .takes_value(true)
                            .default_value(&producers_str),
                    )
                    .arg(
                        Arg::with_name("consumers")
                            .short("c")
                            .long("consumers")
                            .help("Number of consumers")
                            .takes_value(true)
                            .default_value(&consumers_str),
                    )
                    .arg(
                        Arg::with_name("consumers-per-thread")
                            .short("t")
                            .long("consumers-per-thread")
                            .help("Number of consumers")
                            .takes_value(true)
                            .default_value(&consumers_per_thread),
                    ),
            )
            .get_matches();
        let runtime = value_t!(opts, "runtime", Runtime).unwrap_or(Runtime::ThreadPool);
        let scheme = opts.value_of("scheme").unwrap_or("amqp");
        let user = opts.value_of("username").unwrap_or("rabbit");
        let pass = opts.value_of("password").unwrap_or("password");
        let cluster = opts.value_of("cluster").unwrap_or("cluster");
        let vhost = opts.value_of("vhost").unwrap_or("");
        let uri = format!("{}://{}:{}@{}/{}", scheme, user, pass, cluster, vhost);
        let mut producers = PRODUCERS;
        let mut consumers = PRODUCERS;
        let mut consumers_per_thread = CONSUMERS_PER_THREAD;
        if let Some(opts) = opts.subcommand_matches("tune") {
            if let Ok(val) = value_t!(opts, "producers", usize) {
                producers = val;
            }
            if let Ok(val) = value_t!(opts, "consumers", usize) {
                consumers = val;
            }
            if let Ok(val) = value_t!(opts, "consumers_per_thread", usize) {
                consumers_per_thread = val;
            }
        }
        Self {
            runtime,
            uri,
            producers,
            consumers,
            consumers_per_thread,
        }
    }
}

mod msg {
    #![allow(
        unused_imports,
        clippy::extra_unused_lifetimes,
        clippy::needless_lifetimes,
        clippy::redundant_closure,
        clippy::redundant_static_lifetimes
    )]
    include!("../schema/model_generated.rs");

    pub use model::get_root_as_message;
    pub use model::{Message, MessageArgs, MessageBuilder, MessageType};

    #[cfg(test)]
    mod tests {
        use flatbuffers::FlatBufferBuilder;
        #[test]
        fn message_create() {
            use super::get_root_as_message;
            use super::{Message, MessageArgs, MessageType};
            let msgs = ["a", "b", "c", "d"];
            for msg in &msgs {
                let mut b = FlatBufferBuilder::new();
                let bmsg = b.create_string(msg);
                let data = Message::create(
                    &mut b,
                    &MessageArgs {
                        msg: Some(bmsg),
                        ..Default::default()
                    },
                );
                b.finish(data, None);
                let buf = b.finished_data();
                let got = get_root_as_message(buf);
                assert_eq!(msg, &got.msg().unwrap());
                assert_eq!(0, got.id());
                assert_eq!(MessageType::Hello, got.msg_type());
                println!("mesg = {:?}", got);
            }
        }
        #[test]
        fn message_builder() {
            use super::get_root_as_message;
            use super::MessageType;
            let mut b = FlatBufferBuilder::new();
            let bmsg = b.create_string("a");
            let mut mb = super::MessageBuilder::new(&mut b);
            mb.add_id(1000);
            mb.add_msg(bmsg);
            mb.add_msg_type(super::MessageType::Goodbye);
            let data = mb.finish();
            b.finish(data, None);
            let buf = b.finished_data();
            let got = get_root_as_message(buf);
            assert_eq!("a", got.msg().unwrap());
            assert_eq!(1000, got.id());
            assert_eq!(MessageType::Goodbye, got.msg_type());
            println!("msg = {:?}", got);
        }
    }
}
