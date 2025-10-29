# Terrastack Director

**Terrastack Director** is a lightweight, high-performance DNS forwarder and proxy built for modern cloud environments.  
Developed in **Rust** for maximum performance and **memory safety**, it securely forwards DNS queries to upstream resolvers with low latency and high reliability.

---

## 🚀 Features
- **DNS Forwarding** – Efficiently proxy DNS queries to any upstream resolver.  
- **Caching** – Reduce latency and upstream load with built-in query caching.  
- **Memory Safe** – Fully implemented in Rust, eliminating common security vulnerabilities.  
- **Lightweight & Cloud-Native** – Runs anywhere — from bare metal to containers.

---

## 🧩 Architecture
```
[ Client ] → [ Terrastack Director ] → [ Upstream Resolver(s) ]
```

The Director acts as a secure DNS server for clients, receiving DNS queries (potentially over TLS) from local clients or internal applications, and then forwarding them to your chosen upstream DNS servers.

---

## ⚙️ Installation

You can install Terrastack Director either from source using Cargo or by using Docker.

### From Source

Ensure you have Rust and Cargo installed. If not, you can install them via `rustup`: [https://rustup.rs/](https://rustup.rs/)

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/terrastack-cloud/director.git
    cd director
    ```
2.  **Build the project:**
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/director`.
3.  **Install (optional):**
    ```bash
    cargo install --path .
    ```
    This will install `director` to your Cargo bin directory, making it available in your PATH.

### Using Docker

Terrastack Director can be easily deployed using Docker. The official Docker image is available on `ghcr.io/terrastack-cloud/director`.

1.  **Pull the Docker image:**
    ```bash
    docker pull ghcr.io/terrastack-cloud/director:latest
    ```
2.  **Run the Docker container:**
    ```bash
    docker run -d -p 8080:8080/tcp -p 8081:8081/udp -p 8082:8082/tcp -p 8083:8083/tcp --name director ghcr.io/terrastack-cloud/director:latest
    ```
    This command runs the director in detached mode, mapping the default ports. You can customize port mappings and mount a custom `config.yml` if needed.

---

## 🧾 Configuration

Terrastack Director can be configured using a `config.yml` (or `config.toml`) file or environment variables.

### Generating a Default Configuration

You can generate a default configuration file using the `generate` command:

```bash
# Generate default YAML configuration
director generate --format yaml > config.yml

# Generate default TOML configuration
director generate --format toml > config.toml

# Generate default environment variables
director generate --format env > .env
```

### Example `config.yml`

Here's an example `config.yml` with default values:

```yaml
listen:
  http: 0.0.0.0:8080
  udp: 0.0.0.0:8081
  tcp: 0.0.0.0:8082
  tls: 0.0.0.0:8083
upstreams:
  - 1.1.1.1:53
  - 8.8.8.8:53
cache:
  ttl: 300
  enabled: true
```

### TLS and HTTPS Configuration

Terrastack Director supports both DNS over TLS (DoT) and DNS over HTTPS (DoH).

**DNS over TLS (DoT):**
To enable DoT, configure the `tls_cert_config` section in your `config.yml` to specify the paths to your TLS certificate and private key.

```yaml
tls_cert_config:
  cert_path: /path/to/your/cert.pem
  key_path: /path/to/your/key.pem
```

**DNS over HTTPS (DoH):**
For DoH, you can specify the `https_endpoint` in your `config.yml`. This defines the URL path where DoH queries will be served. The default value is `/dns-query`. The TLS certificate configured in `tls_cert_config` will also be used for the HTTPS server.

```yaml
https_endpoint: /dns-query
```

### Environment Variables

All configuration options can also be set via environment variables, prefixed with `DIRECTOR_`. For example:

*   `DIRECTOR_LISTEN_HTTP=0.0.0.0:8080`
*   `DIRECTOR_UPSTREAMS=1.1.1.1:53,8.8.8.8:53`
*   `DIRECTOR_CACHE_ENABLED=true`
*   `DIRECTOR_CACHE_TTL=300`
*   `DIRECTOR_TLS_CERT_CONFIG_CERT_PATH=/path/to/your/cert.pem`
*   `DIRECTOR_TLS_CERT_CONFIG_KEY_PATH=/path/to/your/key.pem`
*   `DIRECTOR_HTTPS_ENDPOINT=/dns-query`

---

## 🧠 Usage

Once installed and configured, you can run Terrastack Director using the `run` command.

### Starting the Director

To start the director with a specific configuration file:

```bash
director run --config-file config.yml
```

If no `--config-file` is specified, the director will look for `config.yml`, `config.yaml`, or `config.toml` in the current directory, and then apply environment variables.

### Using Docker Compose

For more complex Docker deployments, you can use `docker-compose`.

1.  **Create a `config` directory and generate your `config.yml`:**
    ```bash
    mkdir -p config
    director generate --format yaml > config/config.yml
    # Edit config/config.yml as needed, especially for TLS paths if used
    ```
2.  **Create a `docker-compose.yml` file:**
    ```yaml
    services:
      director:
        image: ghcr.io/terrastack-cloud/director:latest
        container_name: director
        volumes:
          - ./config:/app/config
          # If using TLS, mount your cert and key files
          # - ./certs:/app/certs
        ports:
          - "8080:8080/tcp" # HTTP
          - "8081:8081/udp" # UDP
          - "8082:8082/tcp" # TCP
          - "8083:8083/tcp" # TLS
        command: run --config-file /app/config/config.yml
        restart: unless-stopped
    ```
    *   **Note on TLS:** If you are using TLS, ensure your `cert_path` and `key_path` in `config/config.yml` point to the locations *inside the container* (e.g., `/app/certs/cert.pem`). You would also need to uncomment and adjust the `volumes` entry for `certs`.

3.  **Run Docker Compose:**
    ```bash
    docker-compose up -d
    ```

### Command Line Options

```bash
director --help
director run --help
director generate --help
```  

---

## 💬 Community & Support
- Website: [https://terrastack.cloud](https://terrastack.cloud)  
- Issues & feedback: [GitHub Issues](https://github.com/terrastack-cloud/director/issues)  