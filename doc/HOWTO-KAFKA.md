# Kafka exfiltration

In the "daemon mode", Sonar stays memory-resident and pushes data to a network sink; one of these
sinks is a Kafka broker.  See HOWTO-DAEMON.md for general information about this mode and the
options available for configuring the Kafka producer in Sonar.

See `../util/ingest-kafka/` for examples of config files for various node and master types.

Data and control messages are as described in HOWTO-DAEMON.md, with "topic", "key" and "value"
having their standard Kafka meanings.

## CONFIGURING A STANDARD KAFKA BROKER

### Topics

For each cluster with canonical name `<cluster>` that is going to be handled by the broker, the broker needs
to be able to handle messages with these topics coming from the cluster:

```
<cluster>.sample
<cluster>.sysinfo
<cluster>.job
<cluster>.cluster
```

The broker also needs to be able to handle these control topics (tentative, may change) that are
sent from the back-end to the clients in the cluster:

```
<cluster>.control.node
<cluster>.control.master
```

### Testing with Apache Kafka

Test notes with standard Kafka server, see https://kafka.apache.org/quickstart.

#### Setup

You're going to be running several shells, let's call them Zookeeper, Server, Consumer, Work, and Sonar.

The working directory for the following is the root directory of the unpacked Kafka distribution, eg
`kafka_2.13-3.9.0/`.

NOTE!  Currently Sonar does not work with Kafka 4.0.0, there's a protocol error, not debugged.  Use
3.9.0 for the time being.

#### 2.13-3.9.0

In the Zookeeper shell:

```
   bin/zookeeper-server-start.sh config/zookeeper.properties
```

In the Server shell:

```
  bin/kafka-server-start.sh config/server.properties
```

In the Work shell, topics need to be added with `kafka-topics.sh` a la this, if you haven't done it
before (or if you did it, but did not shut down the broker properly):

```
  bin/kafka-topics.sh --create --topic fox.educloud.no.sample --bootstrap-server localhost:9092
```

The topics to add are these (the last two are for control messages):

```
  fox.educloud.no.sample
  fox.educloud.no.sysinfo
  fox.educloud.no.job
  fox.educloud.no.cluster
  fox.educloud.no.control.node
  fox.educloud.no.control.master
```

#### Running sonar and examining the data

Then from the Sonar root directory, after building it, run Sonar in daemon mode with a suitable
config file in the Sonar shell:

```
  target/debug/sonar daemon util/ingest-kafka/sonar-slurm-node.cfg
```

And/or on a single node with access to slurm (eg a login node):

```
  target/debug/sonar daemon util/ingest-kafka/sonar-slurm-master.cfg
```

Sonar will run continuously and start pumping data to Kafka.

In the Consumer shell, go to `util/ingest-kafka` and build `ingest-kafka` if you haven't already.  Run
it; it will subscribe to Kafka and store messages it receives in a data store.  See instructions in
`ingest-kafka.go`.  Typical use when running on the same node as the broker with a non-standard port
XXXX would be:

```
mkdir -p data/fox.educloud.no
./ingest-kafka -cluster fox.educloud.no -data-dir data/fox.educloud.no -broker localhost:XXXX
```

Alternatively, for easy testing, run this in the Consumer shell to listen for sysinfo messages and echo them:

```
  bin/kafka-console-consumer.sh --topic 'fox.educloud.no.sysinfo' --bootstrap-server localhost:XXXX
```

Or run this in the Consumer shell to listen for sample messages and echo them:

```
  bin/kafka-console-consumer.sh --topic 'fox.educloud.no.sample' --bootstrap-server localhost:XXXX
```

#### Sending control messages

To send control messages to Sonar's compute node daemons:

```
  bin/kafka-console-producer.sh --bootstrap-server localhost:XXXX --topic fox.educloud.no.control.node --property parse.key=true
```

and then use TAB to separate key and value on each line.  A good test is `dump true` and
`dump false`, but `exit` should work (without a value).

#### Shutting down Kafka in an orderly way

In the Work shell, in the Kafka root directory:

```
bin/kafka-server-stop.sh --bootstrap-server localhost:9092
bin/zookeeper-server-stop.sh
```

#### Encryption

To enable encryption, three things must happen:

- generate and sign the necessary keys
- update Kafka's `server.properties`
- update the Sonar daemon's .ini file

We will need a CA certificate (to be used by both Kafka and Sonar) and a key store containing the
server's public and private keys (to be used by Kafka).  NOTE, the server keys will be tied to a
particular server name.

For testing, we will generate our own CA and key materials.  In util/ssl, there is a Makefile that
will generate the necessary files: `sonar-ca.crt` is the CA certificate, and
`sonar-kafka-keystore.pem` is the key store.  Just run `make all` to make the files for the local
hostname.

Having generated those, update Kafka's `server.properties` as follows, this is almost in the form of
a diff and the lines starting with `+` have been added to the default config file, NOTE you must
supply full paths for the keystore and CA where the text says `...` below.

```
 #listeners=PLAINTEXT://:9092
+listeners=CLIENT://:9093,CONSUMER://:9099

 # Listener name, hostname and port the broker will advertise to clients.
 # If not set, it uses the value for "listeners".
 #advertised.listeners=PLAINTEXT://your.host.name:9092
+advertised.listeners=CLIENT://:9093,CONSUMER://:9099
+inter.broker.listener.name=CONSUMER

 # Maps listener names to security protocols, the default is for them to be the same. See the config documentation for more details
 #listener.security.protocol.map=PLAINTEXT:PLAINTEXT,SSL:SSL,SASL_PLAINTEXT:SASL_PLAINTEXT,SASL_SSL:SASL_SSL
+listener.security.protocol.map=CLIENT:SSL,CONSUMER:PLAINTEXT

+security.protocol=SSL
+ssl.keystore.type=PEM
+ssl.keystore.location=.../sonar/util/ssl/sonar-kafka-keystore.pem
+ssl.truststore.type=PEM
+ssl.truststore.location=.../sonar/util/ssl/sonar-ca.crt
+ssl.endpoint.identification.algorithm=
```

The meaning of this is that Kafka will continue to communicate in plaintext on port 9099 (for
testing convenience) but will communicate over TLS on port 9093.  The old port 9092 is no longer
active, to avoid confusion.

Finally, the daemon's .ini file must be updated (see [HOWTO-DAEMON](HOWTO-DAEMON.md) for more about
the daemon file).  In the `[kafka]` section, the `broker-address` must use the port 9093, and the
new `ca-file` property must be set to point to `.../sonar/util/ssl/sonar-ca.crt`:

```
[kafka]
broker-address=localhost:9093
...
ca-file = .../sonar/util/ssl/sonar-ca.crt
```

#### Authentication

We will use SASL PLAIN authentication over SSL for now.  In this scheme, the server's config file
contains information about security principals and their passwords.  We will use the cluster name as
the principal name (this will come into play later when we implement authorization).

We can use the same key materials we generated above for SSL, but there are additions to the config
file and the daemon's .ini file.

Changes to `server.properties` relative to the above:

```
-listener.security.protocol.map=CLIENT:SSL,CONSUMER:PLAINTEXT
+listener.security.protocol.map=CLIENT:SASL_SSL,CONSUMER:PLAINTEXT
...
-security.protocol=SSL
+security.protocol=SASL_SSL
 ssl.keystore.type=PEM
 ...
 ssl.endpoint.identification.algorithm=
+
+sasl.enabled.mechanisms=PLAIN
+listener.name.client.plain.sasl.jaas.config=org.apache.kafka.common.security.plain.PlainLoginModule required \
   user_fox.educloud.no="whatdoesthefoxsay";
```

Note that in that last phrase, `client` is the same as `CLIENT` higher up but must here be in lower
case for reasons unknown.

The .ini file gets an addition in the `[kafka]` section: the new `sasl-password` property must hold
the password for the cluster that is configured in the `[global]` section.  (This may change to
point to a file with that password, eventually.)

```
[kafka]
...
sasl-password = whatdoesthefoxsay
```

#### Authorization

TODO.  Here we will use Kafka ACLs to restrict write access to topics <clustername>.<whatever> to
principals <clustername>, and maybe read access to topics <clustername>.control.<role> ditto.  This
is all very TBD.
