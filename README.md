# ipfs-gif

This repository contains the source code of https://gif.reeksite.com, a decentralized GIF hosting and discovery website.

After cloning this repository, there are a few manual steps to set up the application before it can run.

## Operating System Dependencies

OpenSSL must be installed.

```sh
# Ubuntu/Debian
sudo apt install pkg-config libssl-dev
# Fedora
sudo dnf install openssl-devel
# OSX
brew install openssl@3
```

ffprobe must be installed.

```sh
# Ubuntu/Debian
sudo apt install -y ffmpeg
# Fedora
sudo dnf install -y ffmpeg
# OSX
brew install ffmpeg
```

A load balancer (like Nginx) needs to set the `X-Forwarded-For` header with the original requester IP address. This is **required**, otherwise rate limit checkers will unfairly impact all visitors.

```nginx
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
```

## Configuration

You must create a "secrets.toml" file at `/config/secrets.toml` to provide the credentials to connect to a SQL database and IPFS server. Here is an example of how that file should look:

```
[website]
host = "example.com"

[database]
host = "localhost"
port = 3306
user = "admin"
password = "password"

[ipfs]
protocol = "http:"
host = "localhost"
port = 5001

[contact]
arbitration_opt_out_email = "arbitration@example.com"
dcma_email = "dcma@example.com"
```

Without this file, the application will not run. Ensure that the `secrets.toml` file has restrictive file permissions.

### 1. Database

ipfs-gif stores all of its metadata in a MySQL database. It may work in similar SQL databases like MariaDB, but this is not guaranteed.

### 2. IPFS Node

GIFs are uploaded and pinned to the [IPFS](https://ipfs.tech/) node you specify, using the [Kubo RPC API](https://docs.ipfs.tech/reference/kubo/rpc/).

### 3. Contact Info

This is mostly information used for legal contacts.

## Building / Running

This is a standard Rust application. To build:

```sh
cargo build
```

To run:

```sh
cargo run
```

To run while watching for source code changes (for development):

```sh
cargo watch -x run
```

If you encounter weird errors, sometimes it is helpful to clean and rebuild.

```sh
cargo clean
```
