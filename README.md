# Mapper

Mapper is a distributed key-value store written in Rust, for now it is a fully working proof of concept. It provides efficient storage and retrieval of records with support for TTL (time-to-live) policies, sharding, asynchronous operations, and periodic backups.

## Features

- **Sharding**: Data is distributed across multiple shards.
- **TTL Support**: Records can have an optional time-to-live policy.
- **Asynchronous Operations**: Built using `smol`.
- **Customizable**: Configurable via CLI arguments.
- **Backup Functionality**: Periodically creates backups.

## Installation

1. Clone the repository
2. Build the project:

   ```bash
   cargo build --release
   ```

## Usage

Run the application with the following command:

```bash
./mapper
```

## CLI Options

| Option              | Description                              | Default               |
|---------------------|------------------------------------------|-----------------------|
| `--address`         | Address to bind the server               | `127.0.0.1:6379`      |
| `--password`        | Password for authentication              | None                  |
| `--logging-level`   | Logging level (e.g., `info`, `debug`)    | `info`                |
| `--backup-interval` | Backup interval in seconds               | `240`                  |
| `--backup-path`     | Path for backups                         | `.`                   |
| `--backup`          | Enables backup functionality             | `false`               |

## API

The following HTTP API endpoints are supported:

| Method | URL                  | Description                                                                 |
|--------|----------------------|-----------------------------------------------------------------------------|
| GET    | `/GET/{key}`         | Retrieve the value of a record by its key.                                  |
| PUT    | `/SET/{key}`         | Set a record with the specified key and value (value in request body).      |
| PUT    | `/SETEX/{key}/{ttl}` | Set a record with a TTL (time-to-live) in seconds (value in request body).  |
| GET    | `/DEL/{key}`         | Delete a record by its key.                                                 |
| GET    | `/EXISTS/{key}`      | Check if a record exists by its key.                                        |
| GET    | `/EXPIRE/{key}/{ttl}`| Update the TTL of a record.                                                 |
| GET    | `/TTL/{key}`         | Retrieve the remaining TTL of a record.                                     |
| GET    | `/PERSIST/{key}`     | Remove the TTL from a record, making it persistent.                         |
| GET    | `/INFO`              | Retrieve server information.                                                |
| GET    | `/FLUSHALL`          | Remove all records from the database.                                       |
| GET    | `/DBSIZE`            | Retrieve the total number of records in the database.                       |
| GET    | `/PING`              | Check if the server is alive and responsive.                                |

## Example

To start the server with a custom configuration:

```bash
./mapper --address 0.0.0.0:8080 --logging-level debug --backup-path /data/backups --backup-interval 60
```

To interact with the API, you can use tools like `curl`. For example:

- Set a record:

  ```bash
  curl -X PUT http://127.0.0.1:6379/SET/mykey -d "myvalue"
  ```

- Set a record:

  ```bash
  curl -X GET http://127.0.0.1:6379/SET/mykey/myvalue
  ```

- Get a record:

  ```bash
  curl http://127.0.0.1:6379/GET/mykey
  ```

- Set a record with TTL:

  ```bash
  curl -X PUT http://127.0.0.1:6379/SETEX/mykey/60 -d "myvalue"
  ```

- Delete a record:

  ```bash
  curl -X PUT http://127.0.0.1:6379/DEL/mykey
  ```

## TODO

- SSL support
- AOF (Append Only File) logs
- Active/Active cluster or Active/Passive cluster
