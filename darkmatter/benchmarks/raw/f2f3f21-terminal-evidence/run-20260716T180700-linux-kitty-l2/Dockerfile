FROM rust:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
      kitty xvfb libgl1-mesa-dri mesa-utils xauth ca-certificates \
      fontconfig libfontconfig1 fonts-dejavu-core util-linux python3 \
    && fc-cache -fv >/dev/null 2>&1 && rm -rf /var/lib/apt/lists/*
ENV LIBGL_ALWAYS_SOFTWARE=1
ENV GALLIUM_DRIVER=llvmpipe
