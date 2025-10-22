FROM archlinux:latest

# Update system and install base development tools
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
    base-devel \
    curl \
    wget \
    git \
    file \
    openssl \
    gtk3 \
    webkit2gtk-4.1 \
    libappindicator-gtk3 \
    librsvg \
    gdk-pixbuf2 \
    pango \
    cairo \
    glib2 \
    nodejs \
    npm

# Create a build user to match host UID/GID
ARG USER_ID=1000
ARG GROUP_ID=1000
RUN groupadd -g ${GROUP_ID} builder && \
    useradd -m -u ${USER_ID} -g builder builder && \
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Install Rust for the builder user
USER builder
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/home/builder/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /app

# Build command will be run from the host via docker run
CMD ["bash"]
