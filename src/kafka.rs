// Implementation of DataSink for the rdkafka (https://crates.io/crates/rdkafka) library.
//
// Kafka is overkill for Sonar: Sonar has a low message volume per node, even if the per-cluster
// volume can be somewhat high.  Anything that could deliver a message synchronously to a broker
// with not too much overhead and reliably store-and-forward the messages from the broker to the
// eventual endpoint in an efficient manner would have been fine, especially if it had the option of
// an efficient on-cluster intermediary.  But Kafka is standard, reliable, and will do the job.
//
// Here we use the Rust rdkafka library, which sits on top of the industrial-strength C librdkafka
// library.  Both are backed by Confluence, a big player in the Kafka space.  This is far from a
// "pure Rust" solution but the current pure Rust Kafka libraries leave a lot to be desired.
//
// See TODOs throughout.

#![allow(clippy::comparison_to_empty)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use crate::daemon::{Dur, Ini, Operation};
use crate::datasink::DataSink;
use crate::log;
use crate::time;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::client::ClientContext;
use rdkafka::producer::base_producer::ThreadedProducer;
use rdkafka::producer::{BaseRecord, NoCustomPartitioner, ProducerContext};
use rdkafka::message::DeliveryResult;

struct Message {
    timestamp: u64,
    topic: String,
    key: String,
    value: String,
}

pub struct RdKafka {
    outgoing_message_queue: mpsc::Sender<Message>,
}

// `control_and_errors` is the channel on which incoming control messages from the broker and any
// fatal errors will be posted (as Operation::Incoming(key,value) and Operation::Fatal(msg),
// respectively).

impl RdKafka {
    pub fn new(
        ini: &Ini,
        client_id: String,
        control_topic: String,
        control_and_errors: mpsc::Sender<Operation>,
    ) -> RdKafka {
        let global = &ini.global;
        let kafka = &ini.kafka;
        let debug = &ini.debug;
        let (outgoing_message_queue, incoming_message_queue) = mpsc::channel();

        {
            let broker = kafka.broker_address.clone();
            let ca_file = kafka.ca_file.clone();
            let sasl_identity = if let Some(ref password) = kafka.sasl_password {
                Some((global.cluster.clone(), password.clone()))
            } else {
                None
            };
            let compression = if let Some(s) = &kafka.compression {
                match s.as_ref() {
                    "lz4" => "lz4",
                    _ => "none",
                }
            } else {
                "none"
            };
            let sending_window = ini.kafka.sending_window.to_seconds();
            let control_and_errors = control_and_errors.clone();
            let verbose = debug.verbose;
            thread::spawn(move || {
                kafka_producer(
                    broker,
                    client_id,
                    ca_file,
                    sasl_identity,
                    compression,
                    sending_window,
                    incoming_message_queue,
                    control_and_errors,
                    verbose,
                );
            });
        }

        {
            let broker = kafka.broker_address.clone();
            let poll_interval = kafka.poll_interval;
            let verbose = debug.verbose;
            thread::spawn(move || {
                kafka_consumer(broker, control_topic, poll_interval, control_and_errors, verbose);
            });
        }
        RdKafka { outgoing_message_queue }
    }
}

impl DataSink for RdKafka {
    fn post(&self, topic: String, key: String, value: String) {
        // The send can really only fail if the Kafka producer thread has closed the channel, and
        // that will only happen if it has been told to exit, so in that case ignore the error here.
        let _ignored = self.outgoing_message_queue.send(Message{
            timestamp: time::unix_now(),
            topic,
            key,
            value,
        });
    }

    fn stop(&self) {
        // The outgoing queue is dropped when this object is dropped and we can hope that the
        // underlying queue is closed at that point.  We could make dropping explicit here by
        // stuffing the queue in something that is explicitly droppable I guess.  Not obvious to me
        // what the contract is.
        //
        // Anyway, if the queue is closed the producer thread, when it wakes up and tries to read
        // from the queue, will exit.  This is probably as far as I'm interested in taking it.  The
        // process will long since have terminated at that point unless we take pains to wait for
        // the thread to exit here.  Since that thread can be in a long wait, we probably don't want
        // to do that now.  But using crossbeam we would not have to sync wait in the producer
        // thread, and *then* an exit signal could be delivered properly.
    }
}

// Sending logic works like this:
//
// We use a ThreadedProducer to send messages.  In this scheme, we enqueue messages and a background
// thread will poll the Kafka subsystem as necessary to make sure they are sent.  We set the message
// timeout to 30m and that is our main means of controlling the backlog.  When a message fails to be
// sent for reasons of timeout, we simply drop it.  But there may be other reasons for not sending
// it, and some of those reasons will lead to the message being enqueued and some will lead to it
// being dropped.  Some reasons may lead to a fatal error having to be posted, (don't know yet).

struct SonarProducerContext {
    verbose: bool,
    control_and_errors: mpsc::Sender<Operation>,
}

impl ClientContext for SonarProducerContext {
}

fn kafka_producer(
    broker: String,
    client_id: String,
    ca_file: Option<String>,
    sasl_identity: Option<(String,String)>,
    compression: &str,
    sending_window: u64,
    incoming_message_queue: mpsc::Receiver<Message>,
    control_and_errors: mpsc::Sender<Operation>,
    verbose: bool,
) {
    let mut cfg = ClientConfig::new();
    cfg
        .set("bootstrap.servers", &broker)
        .set("client.id", &client_id)
        .set("queue.buffering.max.ms", "1000")
        .set("compression.codec", compression)
        .set("message.timeout.ms", "1800000"); // 30 minutes
    if let Some(ref filename) = ca_file {
        cfg
            .set("ssl.ca.location", filename)
            .set("ssl.endpoint.identification.algorithm", "none");
        if let Some((ref username, ref password)) = sasl_identity {
            cfg
                .set("security.protocol", "sasl_ssl")
                .set("sasl.mechanism", "PLAIN") // yeah, must be upper case...
                .set("sasl.username", username)
                .set("sasl.password", password);
        } else {
            cfg.set("security.protocol", "ssl");
        }
    }
    let producer : &ThreadedProducer<SonarProducerContext, NoCustomPartitioner> =
        &cfg
        .create_with_context::<SonarProducerContext,
                               ThreadedProducer<SonarProducerContext, NoCustomPartitioner>>(
            SonarProducerContext { verbose, control_and_errors },
        )
        .expect("Producer creation error");

    let mut id = 0;
    let mut rng = Rng::new();
    'producer_loop:
    loop {
        if verbose {
            log::info("Waiting for stuff to send");
        }
        match incoming_message_queue.recv() {
            Err(_) => {
                // Channel was closed, so exit.
                break 'producer_loop;
            }
            Ok(mut msg) => {
                if sending_window > 1 {
                    let sleep = rng.next() as u64 % sending_window;
                    if verbose {
                        log::info(&format!("Sleeping {sleep} before sending"));
                    }
                    thread::sleep(Duration::from_secs(sleep));
                }
                id += 1;
                if verbose {
                    log::info(&format!("Sending to topic: {} with id {id}", msg.topic));
                }

                'sender_loop:
                loop {
                    // TODO: There are various problems with sending here that we should maybe
                    // try to figure out and signal in a sensible way.
                    let _ = producer.send(
                        BaseRecord::with_opaque_to(&msg.topic, id)
                            .payload(&msg.value)
                            .key(&msg.key),
                    );

                    // Once we're sending, send everything we've got, or we may get backed up if the
                    // production cadence is higher than the sending cadence.
                    match incoming_message_queue.try_recv() {
                        Err(mpsc::TryRecvError::Empty) => {
                            break 'sender_loop;
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            break 'producer_loop;
                        }
                        Ok(m) => {
                            msg = m;
                        }
                    }
                }
            }
        }
    }
}

impl ProducerContext for SonarProducerContext {
    type DeliveryOpaque = usize;
    fn delivery(&self,
                delivery_result: &DeliveryResult<'_>,
                delivery_opaque: Self::DeliveryOpaque) {
        match delivery_result {
            Ok(_) => {
                if self.verbose {
                    log::info(&format!("Sent #{delivery_opaque} successfully"));
                }
            }
            Err((e, m)) => {
                // TODO:
                // - if the result is an error, then we may need to re-enqueue the message, or other
                //   actions
                // - fatal errors will need to be delivered on the control_and_errors channel
                if self.verbose {
                    log::info(&format!("Message production error {delivery_opaque} {:?}", e));
                }
            }
        }
    }
}

// Recoverable errors are handled with a log message, but fatal errors must be posted back to the
// main thread as Operation::Fatal messages.

fn kafka_consumer(
    broker: String,
    control_topic: String,
    poll_interval: Dur,
    control_and_errors: mpsc::Sender<Operation>,
    verbose: bool,
) {
// TODO: Implement
//    let consumer : &BaseConsumer =

/*
    // No group as of yet - don't know if we need it, don't know what the implications are.  We need
    // to not risk needing local state on the node.
    let group = "";

    // See comments above about the producer, we need to be resilient to transient errors when
    // creating the consumer.
    let mut consumer = loop {
        match consumer::Consumer::from_hosts(vec![host.clone()])
            .with_topic(control_topic.clone())
            .with_fallback_offset(consumer::FetchOffset::Latest)
            .with_group(group.to_string())
            .create()
        {
            Ok(c) => {
                if verbose {
                    log::info(&format!("Success creating consumer of {control_topic}"));
                }
                break c;
            }
            Err(e) => {
                if verbose {
                    log::info(&format!("Failed to create consumer of {control_topic}\nReason: {e}\nSleeping 1m"));
                }
                thread::sleep(Duration::from_secs(60));
            }
        }
    };

    'consumer_loop: loop {
        thread::sleep(std::time::Duration::from_secs(poll_interval.to_seconds()));
        let responses = match consumer.poll() {
            Ok(r) => r,
            Err(e) => {
                // This happens at least for "unknown topic or partition".  That can be a transient
                // or permanent error.  As we'll be polling at a limited rate it's OK to just log
                // the problem and try again later.
                if verbose {
                    log::info(&format!("Consumer error: {e}"));
                }
                continue;
            }
        };
        for ms in responses.iter() {
            for m in ms.messages() {
                let key = String::from_utf8_lossy(m.key).to_string();
                let value = String::from_utf8_lossy(m.value).to_string();
                match sender.send(Operation::Incoming(key, value)) {
                    Ok(_) => {}
                    Err(e) => {
                        // Channel was closed, no option but to exit.
                        if verbose {
                            log::info(&format!("Send error on consumer channel: {e}"));
                        }
                        break 'consumer_loop;
                    }
                }
            }
            match consumer.consume_messageset(ms) {
                Ok(_) => {}
                Err(e) => {
                    if verbose {
                        log::info(&format!("Could not consume: {e}"));
                    }
                }
            }
        }
        if group != "" {
            match consumer.commit_consumed() {
                Ok(()) => {}
                Err(e) => {
                    if verbose {
                        log::info(&format!("Could not commit consumed: {e}"));
                    }
                }
            }
        }
    }
*/
}

// Generate randomish u32 numbers

pub struct Rng {
    state: u32                  // nonzero
}

impl Rng {
    pub fn new() -> Rng {
        Rng { state: crate::time::unix_now() as u32 }
    }

    // https://en.wikipedia.org/wiki/Xorshift, this supposedly has period 2^32-1 but is not "very
    // random".
    pub fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }
}

#[test]
pub fn rng_test() {
    let mut r = Rng::new();
    let a = r.next();
    let b = r.next();
    let c = r.next();
    let d = r.next();
    // It's completely unlikely that they're all equal, so that would indicate some kind of bug.
    assert!(!(a == b && b == c && c == d));
}
