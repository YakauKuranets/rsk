# hyperion-ultimate/Dockerfile
# All-in-one security platform Docker image
FROM ubuntu:24.04


ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"


# System base
RUN apt-get update && apt-get install -y \
    curl wget git build-essential pkg-config libssl-dev \
    python3 python3-pip nmap nikto netcat-openbsd \
    hydra sqlmap whatweb gobuster ffuf \
    hashcat john wordlists \
    tcpdump wireshark-common tshark \
    docker.io libpcap-dev \
    && rm -rf /var/lib/apt/lists/*


# Nuclei (vulnerability scanner)
RUN wget -qO /tmp/nuclei.tar.gz \
    https://github.com/projectdiscovery/nuclei/releases/latest/download/nuclei_3.2.4_linux_amd64.tar.gz \
    && tar -xzf /tmp/nuclei.tar.gz -C /usr/local/bin nuclei \
    && rm /tmp/nuclei.tar.gz


# Amass (OSINT/subdomain)
RUN wget -qO /tmp/amass.zip \
    https://github.com/owasp-amass/amass/releases/latest/download/amass_linux_amd64.zip \
    && unzip -q /tmp/amass.zip -d /tmp/amass \
    && mv /tmp/amass/amass_linux_amd64/amass /usr/local/bin/ \
    && rm -rf /tmp/amass /tmp/amass.zip


# Metasploit Framework
RUN curl -fsSL https://raw.githubusercontent.com/rapid7/metasploit-omnibus/master/config/templates/metasploit-framework-wrappers/msfupdate.erb \
    | bash


# Ollama (local LLM)
RUN curl -fsSL https://ollama.ai/install.sh | sh


# Rust toolchain for Hyperion
RUN curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"


# Hyperion source
WORKDIR /hyperion
COPY . .


# Build Hyperion backend
RUN cd src-tauri && cargo build --release


# Entrypoint script
COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh


EXPOSE 7878  # REST API
EXPOSE 11434 # Ollama
EXPOSE 4444  # MSF listener



